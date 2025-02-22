use std::io::{self, Read};

use itertools::Itertools;

use crate::{
    jvm::{
        Annotation, ConstantValue, Module, TypeAnnotation,
        annotation::ElementValue,
        class::{BootstrapMethod, EnclosingMethod, InnerClassInfo, RecordComponent},
        code::{LineNumberTableEntry, MethodBody, StackMapFrame},
        method::ParameterInfo,
        references::{ClassRef, PackageRef},
    },
    macros::see_jvm_spec,
};

use super::{
    Context, Error,
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    jvm_element_parser::ClassElement,
    reader_utils::{FromReader, ValueReaderExt, read_byte_chunk},
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
        let reader = &mut info.as_slice();

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
        match reader.read(&mut [0]) {
            Ok(0) => Ok(result),
            Ok(1) => Err(Error::IO(io::Error::new(
                io::ErrorKind::InvalidData,
                "Extra data at the end of the attribute",
            ))),
            Err(e) => Err(e.into()),
            _ => unreachable!(),
        }
    }
}

#[inline]
fn parse_string<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<String, Error> {
    let str_idx = reader.read_value()?;
    ctx.constant_pool.get_str(str_idx).map(str::to_owned)
}
