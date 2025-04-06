use std::{
    collections::VecDeque,
    io::{self, Read},
    num::TryFromIntError,
};

use itertools::Itertools;
use num_traits::ToBytes;

use crate::{
    jvm::{
        Annotation, ConstantValue, Module, TypeAnnotation,
        annotation::ElementValue,
        class::{BootstrapMethod, ConstantPool, EnclosingMethod, InnerClassInfo, RecordComponent},
        code::{LineNumberTableEntry, MethodBody, StackMapFrame},
        method::ParameterInfo,
        references::{ClassRef, PackageRef},
    },
    macros::see_jvm_spec,
};

use super::{
    Context, Error, ToWriter, ToWriterError,
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    jvm_element_parser::ClassElement,
    reader_utils::{FromReader, ValueReaderExt, read_byte_chunk},
    write_length,
};

/// Represent an attribute of a class file, method, field, or code.
#[doc = see_jvm_spec!(4, 7)]
#[derive(Debug)]
pub(crate) struct AttributeInfo {
    name_idx: u16,
    info: Vec<u8>,
}

impl AttributeInfo {
    fn from_raw_parts(name_idx: u16, info: Vec<u8>) -> Self {
        Self { name_idx, info }
    }
}

impl FromReader for AttributeInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let name_idx = reader.read_value()?;
        let attribute_length: u32 = reader.read_value()?;
        let attribute_length = usize::try_from(attribute_length)
            .expect("32-bit size is not supported on the current platform");
        let info = read_byte_chunk(reader, attribute_length)?;
        Ok(Self::from_raw_parts(name_idx, info))
    }
}

impl ToWriter for AttributeInfo {
    fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), ToWriterError> {
        writer.write_all(&self.name_idx.to_be_bytes())?;
        write_length::<u32>(writer, self.info.len())?;
        writer.write_all(&self.info)?;
        Ok(())
    }
}

impl ToWriter for Vec<AttributeInfo> {
    fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), ToWriterError> {
        write_length::<u16>(writer, self.len())?;
        for attr in self {
            attr.to_writer(writer)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum Attribute {
    ConstantValue(ConstantValue),
    Code(MethodBody),
    StackMapTable(Vec<StackMapFrame>),
    Exceptions(Vec<ClassRef>),
    SourceFile(String),
    LineNumberTable(Vec<LineNumberTableEntry>),
    InnerClasses(Vec<InnerClassInfo>),
    Synthetic,
    Deprecated,
    EnclosingMethod(EnclosingMethod),
    Signature(String),
    SourceDebugExtension(Vec<u8>),
    LocalVariableTable(Vec<LocalVariableDescAttr>),
    LocalVariableTypeTable(Vec<LocalVariableTypeAttr>),
    RuntimeVisibleAnnotations(Vec<Annotation>),
    RuntimeInvisibleAnnotations(Vec<Annotation>),
    RuntimeVisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeInvisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeVisibleTypeAnnotations(Vec<TypeAnnotation>),
    RuntimeInvisibleTypeAnnotations(Vec<TypeAnnotation>),
    AnnotationDefault(ElementValue),
    BootstrapMethods(Vec<BootstrapMethod>),
    MethodParameters(Vec<ParameterInfo>),
    Module(Module),
    ModulePackages(Vec<PackageRef>),
    ModuleMainClass(ClassRef),
    NestHost(ClassRef),
    NestMembers(Vec<ClassRef>),
    Record(Vec<RecordComponent>),
    PermittedSubclasses(Vec<ClassRef>),
    Unrecognized(String, Vec<u8>),
}

impl Attribute {
    pub fn name(&self) -> &str {
        match self {
            Self::ConstantValue(_) => "ConstantValue",
            Self::Code(_) => "Code",
            Self::StackMapTable(_) => "StackMapTable",
            Self::Exceptions(_) => "Exceptions",
            Self::SourceFile(_) => "SourceFile",
            Self::LineNumberTable(_) => "LineNumberTable",
            Self::InnerClasses(_) => "InnerClasses",
            Self::Synthetic => "Synthetic",
            Self::Deprecated => "Deprecated",
            Self::EnclosingMethod(_) => "EnclosingMethod",
            Self::Signature(_) => "Signature",
            Self::SourceDebugExtension(_) => "SourceDebugExtension",
            Self::LocalVariableTable(_) => "LocalVariableTable",
            Self::LocalVariableTypeTable(_) => "LocalVariableTypeTable",
            Self::RuntimeVisibleAnnotations(_) => "RuntimeVisibleAnnotations",
            Self::RuntimeInvisibleAnnotations(_) => "RuntimeInvisibleAnnotations",
            Self::RuntimeVisibleParameterAnnotations(_) => "RuntimeVisibleParameterAnnotations",
            Self::RuntimeInvisibleParameterAnnotations(_) => "RuntimeInvisibleParameterAnnotations",
            Self::RuntimeVisibleTypeAnnotations(_) => "RuntimeVisibleTypeAnnotations",
            Self::RuntimeInvisibleTypeAnnotations(_) => "RuntimeInvisibleTypeAnnotations",
            Self::AnnotationDefault(_) => "AnnotationDefault",
            Self::BootstrapMethods(_) => "BootstrapMethods",
            Self::MethodParameters(_) => "MethodParameters",
            Self::Module(_) => "Module",
            Self::ModulePackages(_) => "ModulePackages",
            Self::ModuleMainClass(_) => "ModuleMainClass",
            Self::NestHost(_) => "NestHost",
            Self::NestMembers(_) => "NestMembers",
            Self::Record(_) => "Record",
            Self::PermittedSubclasses(_) => "PermittedSubclasses",
            Self::Unrecognized(name, _) => name,
        }
    }
}

macro_rules! parse {
    ($reader:expr_2021, $ctx:expr_2021 $(=> $attr:ident )?) => {{
        let raw = $reader.read_value()?;
        ClassElement::from_raw(raw, $ctx)$( .map(Self::$attr) )?
    }};
    ($len_type:ty; $reader:expr_2021, || $with:expr_2021 $(=> $attr:ident )?) => {{
        let count: $len_type = $reader.read_value()?;
        (0..count).map(|_| $with).try_collect()$( .map(Self::$attr) )?
    }};
    ($len_type:ty; $reader:expr_2021, $ctx:expr_2021 $(=> $attr:ident )?) => {
        parse![$len_type; $reader, || parse!($reader, $ctx)] $( .map(Self::$attr) )?
    };
}

impl ClassElement for Attribute {
    type Raw = AttributeInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let AttributeInfo { name_idx, info } = raw;
        let name = ctx.constant_pool.get_str(name_idx)?;
        let reader = &mut VecDeque::from(info);

        let result = match name {
            "ConstantValue" => {
                let idx = reader.read_value()?;
                ctx.constant_pool
                    .get_constant_value(idx)
                    .map(Self::ConstantValue)
            }
            "Code" => parse!(reader, ctx => Code),
            "StackMapTable" => parse![u16; reader, ctx => StackMapTable],
            "Exceptions" => parse![u16; reader, || {
                let idx = reader.read_value()?;
                ctx.constant_pool.get_class_ref(idx)
            } => Exceptions],
            "InnerClasses" => parse![u16; reader, ctx => InnerClasses],
            "EnclosingMethod" => parse!(reader, ctx).map(Self::EnclosingMethod),
            "Synthetic" => Ok(Attribute::Synthetic),
            "Deprecated" => Ok(Attribute::Deprecated),
            "Signature" => parse_string(reader, ctx).map(Self::Signature),
            "SourceFile" => parse_string(reader, ctx).map(Self::SourceFile),
            "SourceDebugExtension" => {
                let bytes = reader.bytes().try_collect()?;
                Ok(Self::SourceDebugExtension(bytes))
            }
            "LineNumberTable" => parse![u16; reader, ctx => LineNumberTable],
            "LocalVariableTable" => parse![u16; reader, ctx => LocalVariableTable],
            "LocalVariableTypeTable" => parse![u16; reader, ctx => LocalVariableTypeTable],
            "RuntimeVisibleAnnotations" => parse![u16; reader, ctx => RuntimeVisibleAnnotations],
            "RuntimeInvisibleAnnotations" => {
                parse![u16; reader, ctx => RuntimeInvisibleAnnotations]
            }
            "RuntimeVisibleParameterAnnotations" => parse![u8; reader, || parse![u16; reader, ctx]]
                .map(Self::RuntimeVisibleParameterAnnotations),
            "RuntimeInvisibleParameterAnnotations" => {
                parse![u8; reader, || parse![u16; reader, ctx] => RuntimeInvisibleParameterAnnotations]
            }
            "RuntimeVisibleTypeAnnotations" => {
                parse![u16; reader, ctx => RuntimeVisibleTypeAnnotations]
            }
            "RuntimeInvisibleTypeAnnotations" => {
                parse![u16; reader, ctx => RuntimeInvisibleTypeAnnotations]
            }
            "AnnotationDefault" => parse!(reader, ctx => AnnotationDefault),
            "BootstrapMethods" => parse![u16; reader, ctx => BootstrapMethods],
            "MethodParameters" => parse![u8; reader, ctx => MethodParameters],
            "Module" => parse!(reader, ctx => Module),
            "ModulePackages" => parse![u16; reader, || {
                let idx = reader.read_value()?;
                ctx.constant_pool.get_package_ref(idx)
            } => ModulePackages],
            "ModuleMainClass" => {
                let idx = reader.read_value()?;
                ctx.constant_pool
                    .get_class_ref(idx)
                    .map(Self::ModuleMainClass)
            }
            "NestHost" => {
                let idx = reader.read_value()?;
                ctx.constant_pool.get_class_ref(idx).map(Self::NestHost)
            }
            "NestMembers" => parse![u16; reader, || {
                let idx = reader.read_value()?;
                ctx.constant_pool.get_class_ref(idx)
            }]
            .map(Self::NestMembers),
            "Record" => parse![u16; reader, ctx => Record],
            "PermittedSubclasses" => parse![u16; reader, || {
                let idx = reader.read_value()?;
                ctx.constant_pool.get_class_ref(idx)
            } => PermittedSubclasses],
            name => reader
                .bytes()
                .try_collect()
                .map(|bytes| Attribute::Unrecognized(name.to_owned(), bytes))
                .map_err(Into::into),
        }?;
        if reader.is_empty() {
            Ok(result)
        } else {
            Err(Error::IO(io::Error::new(
                io::ErrorKind::InvalidData,
                "Extra data at the end of the attribute",
            )))
        }
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let name_idx = cp.put_string(self.name().to_owned())?;
        let info = self.into_bytes(cp)?;
        Ok(Self::Raw { name_idx, info })
    }
}

impl Attribute {
    fn into_bytes(self, cp: &mut ConstantPool) -> Result<Vec<u8>, ToWriterError> {
        let mut bytes = match self {
            Attribute::ConstantValue(constant_value) => {
                let constant_value_idx = cp.put_constant_value(constant_value)?;
                constant_value_idx.to_be_bytes().to_vec()
            }
            Attribute::Code(method_body) => method_body.into_bytes(cp)?,
            Attribute::StackMapTable(entries) => serialize_vec::<u16>(entries, cp)?,
            Attribute::Exceptions(exception_types) => {
                let mut bytes = Vec::new();
                write_length::<u16>(&mut bytes, exception_types.len())?;
                for exception_type in exception_types {
                    bytes.extend(cp.put_class_ref(exception_type)?.to_be_bytes());
                }
                bytes
            }
            Attribute::InnerClasses(classes) => serialize_vec::<u16>(classes, cp)?,
            Attribute::EnclosingMethod(enclosing_method) => enclosing_method.into_bytes(cp)?,
            Attribute::Synthetic | Attribute::Deprecated => Vec::default(),
            Attribute::Signature(str_value) | Attribute::SourceFile(str_value) => {
                cp.put_string(str_value)?.to_be_bytes().into()
            }
            Attribute::SourceDebugExtension(data) | Attribute::Unrecognized(_, data) => data,
            Attribute::LineNumberTable(entries) => serialize_vec::<u16>(entries, cp)?,
            Attribute::LocalVariableTable(entries) => serialize_vec::<u16>(entries, cp)?,
            Attribute::LocalVariableTypeTable(entries) => serialize_vec::<u16>(entries, cp)?,
            Attribute::RuntimeVisibleAnnotations(annotations)
            | Attribute::RuntimeInvisibleAnnotations(annotations) => {
                serialize_vec::<u16>(annotations, cp)?
            }
            Attribute::RuntimeVisibleTypeAnnotations(annotations)
            | Attribute::RuntimeInvisibleTypeAnnotations(annotations) => {
                serialize_vec::<u16>(annotations, cp)?
            }
            Attribute::RuntimeVisibleParameterAnnotations(outer)
            | Attribute::RuntimeInvisibleParameterAnnotations(outer) => {
                let mut buf = Vec::new();
                write_length::<u8>(&mut buf, outer.len())?;
                for inner in outer {
                    buf.extend(serialize_vec::<u16>(inner, cp)?);
                }
                buf
            }
            Attribute::AnnotationDefault(value) => value.into_bytes(cp)?,
            Attribute::BootstrapMethods(bsms) => serialize_vec::<u16>(bsms, cp)?,
            Attribute::MethodParameters(params) => serialize_vec::<u8>(params, cp)?,
            Attribute::Module(module) => module.into_bytes(cp)?,
            Attribute::ModulePackages(mod_pkg) => {
                let mut buf = Vec::new();
                write_length::<u16>(&mut buf, mod_pkg.len())?;
                for pkg in mod_pkg {
                    buf.extend(cp.put_package_ref(pkg)?.to_be_bytes());
                }
                buf
            }
            Attribute::NestHost(class_ref) | Attribute::ModuleMainClass(class_ref) => {
                cp.put_class_ref(class_ref)?.to_be_bytes().to_vec()
            }
            Attribute::NestMembers(classes) | Attribute::PermittedSubclasses(classes) => {
                let mut buf = Vec::new();
                write_length::<u16>(&mut buf, classes.len())?;
                for class in classes {
                    buf.extend(cp.put_class_ref(class)?.to_be_bytes());
                }
                buf
            }
            Attribute::Record(components) => serialize_vec::<u16>(components, cp)?,
        };
        bytes.shrink_to_fit();
        Ok(bytes)
    }
}

#[inline]
fn parse_string<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<String, Error> {
    let str_idx = reader.read_value()?;
    ctx.constant_pool.get_str(str_idx).map(str::to_owned)
}

#[inline]
fn serialize_vec<Len>(
    items: Vec<impl ClassElement<Raw: ToWriter>>,
    cp: &mut ConstantPool,
) -> Result<Vec<u8>, ToWriterError>
where
    usize: TryInto<Len, Error = TryFromIntError>,
    Len: ToBytes,
    <Len as ToBytes>::Bytes: IntoIterator<Item = u8>,
{
    let mut buf = Vec::new();
    let len = items.len().try_into()?;
    buf.extend(len.to_be_bytes());
    for item in items {
        buf.extend(item.into_bytes(cp)?);
    }
    Ok(buf)
}
