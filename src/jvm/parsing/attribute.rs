use std::{
    io::{self, Read},
    usize,
};

use itertools::Itertools;

use crate::{
    jvm::{
        annotation::{Annotation, ElementValue, TypeAnnotation},
        class::{BootstrapMethod, EnclosingMethod, InnerClassInfo, RecordComponent},
        code::{LineNumberTableEntry, MethodBody, StackMapFrame},
        field::ConstantValue,
        method::ParameterInfo,
        module::Module,
        references::{ClassRef, PackageRef},
    },
    macros::see_jvm_spec,
};

use super::{
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    jvm_element_parser::{FromRaw, JvmElement},
    raw_attributes,
    reader_utils::{read_byte_chunk, FromReader, ValueReaderExt},
    Context, Error,
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

macro_rules! parse_from_raw {
    ($reader:expr, $ctx:expr) => {{
        let raw_attr = $reader.read_value()?;
        FromRaw::from_raw(raw_attr, $ctx)
    }};
}

macro_rules! parse_multiple {
    ($len_type:ty; $reader:expr, || $with:expr) => {{
        let count: $len_type = $reader.read_value()?;
        (0..count).map(|_| $with).try_collect()
    }};
    ($len_type:ty; $reader:expr, $ctx:expr) => {
        parse_multiple![$len_type; $reader, || {
            let raw = FromReader::from_reader($reader)?;
            FromRaw::from_raw(raw, $ctx)
        }]
    };
}

impl FromRaw for Attribute {
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
            "Code" => MethodBody::from_raw(reader.read_value()?, ctx).map(Self::Code),
            "StackMapTable" => parse_multiple![u16; reader, ctx].map(Self::StackMapTable),
            "Exceptions" => parse_multiple![u16; reader, || {
                let idx = reader.read_value()?;
                ctx.constant_pool.get_class_ref(idx)
            }]
            .map(Self::Exceptions),
            "InnerClasses" => parse_multiple![u16; reader, ctx].map(Self::InnerClasses),
            "EnclosingMethod" => parse_from_raw!(reader, ctx).map(Self::EnclosingMethod),
            "Synthetic" => Ok(Attribute::Synthetic),
            "Deprecated" => Ok(Attribute::Deprecated),
            "Signature" => {
                let str_idx = reader.read_value()?;
                ctx.constant_pool
                    .get_str(str_idx)
                    .map(str::to_owned)
                    .map(Self::Signature)
            }
            "SourceFile" => {
                let str_idx = reader.read_value()?;
                ctx.constant_pool
                    .get_str(str_idx)
                    .map(str::to_owned)
                    .map(Self::SourceFile)
            }
            "SourceDebugExtension" => {
                let bytes = reader.bytes().try_collect()?;
                Ok(Self::SourceDebugExtension(bytes))
            }
            "LineNumberTable" => parse_multiple![u16; reader, ctx].map(Self::LineNumberTable),
            "LocalVariableTable" => parse_multiple![u16; reader, ctx].map(Self::LocalVariableTable),
            "LocalVariableTypeTable" => {
                parse_multiple![u16; reader, ctx].map(Self::LocalVariableTypeTable)
            }
            "RuntimeVisibleAnnotations" => {
                parse_multiple![u16; reader, ctx].map(Self::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                parse_multiple![u16; reader, ctx].map(Self::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => {
                parse_multiple![u8; reader, || parse_multiple![u16; reader, ctx]]
                    .map(Self::RuntimeVisibleParameterAnnotations)
            }
            "RuntimeInvisibleParameterAnnotations" => {
                parse_multiple![u8; reader, || parse_multiple![u16; reader, ctx]]
                    .map(Self::RuntimeInvisibleParameterAnnotations)
            }
            "RuntimeVisibleTypeAnnotations" => {
                parse_multiple![u16; reader, ctx].map(Self::RuntimeVisibleTypeAnnotations)
            }
            "RuntimeInvisibleTypeAnnotations" => {
                parse_multiple![u16; reader, ctx].map(Self::RuntimeInvisibleTypeAnnotations)
            }
            "AnnotationDefault" => JvmElement::parse(reader, ctx).map(Self::AnnotationDefault),
            "BootstrapMethods" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::BootstrapMethods)
            }
            "MethodParameters" => {
                JvmElement::parse_vec::<u8, _>(reader, ctx).map(Self::MethodParameters)
            }
            "Module" => JvmElement::parse(reader, ctx).map(Self::Module),
            "ModulePackages" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::ModulePackages)
            }
            "ModuleMainClass" => JvmElement::parse(reader, ctx).map(Self::ModuleMainClass),
            "NestHost" => JvmElement::parse(reader, ctx).map(Self::NestHost),
            "NestMembers" => JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::NestMembers),
            "Record" => JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::Record),
            "PermittedSubclasses" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::PermittedSubclasses)
            }
            name => reader
                .bytes()
                .try_collect()
                .map(|bytes| Attribute::Unrecognized(name.to_owned(), bytes))
                .map_err(Into::into),
        }?;
        match reader.read(&mut [0]) {
            Ok(0) => Ok(result),
            Ok(_) => Err(Error::IO(io::Error::new(
                io::ErrorKind::InvalidData,
                "Extra data at the end of the attribute",
            ))),
            Err(e) => Err(e.into()),
        }
    }
}
impl FromRaw for EnclosingMethod {
    type Raw = raw_attributes::EnclosingMethod;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            class_index,
            method_index,
        } = raw;
        let class = ctx.constant_pool.get_class_ref(class_index)?;
        let method_name_and_desc = if method_index > 0 {
            let name_and_desc = ctx.constant_pool.get_name_and_type(method_index)?;
            Some(name_and_desc)
        } else {
            None
        };
        Ok(EnclosingMethod {
            class,
            method_name_and_desc,
        })
    }
}
