use std::{
    io::{self, Read},
    iter::repeat_with,
    usize,
};

use itertools::Itertools;

use crate::{
    jvm::{
        annotation::{Annotation, ElementValue, Type},
        class::{
            BootstrapMethod, EnclosingMethod, InnerClassInfo, RecordComponent, SourceDebugExtension,
        },
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
    SourceDebugExtension(SourceDebugExtension),
    LocalVariableTable(Vec<LocalVariableDescAttr>),
    LocalVariableTypeTable(Vec<LocalVariableTypeAttr>),
    RuntimeVisibleAnnotations(Vec<Annotation>),
    RuntimeInvisibleAnnotations(Vec<Annotation>),
    RuntimeVisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeInvisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeVisibleTypeAnnotations(Vec<Type>),
    RuntimeInvisibleTypeAnnotations(Vec<Type>),
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

impl FromRaw for Attribute {
    type Raw = AttributeInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let AttributeInfo { name_idx, info } = raw;
        let name = ctx.constant_pool.get_str(name_idx)?;
        let reader = &mut info.as_slice();

        let result = match name {
            "ConstantValue" => JvmElement::parse(reader, ctx).map(Self::ConstantValue),
            "Code" => JvmElement::parse(reader, ctx).map(Self::Code),
            "StackMapTable" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::StackMapTable)
            }
            "Exceptions" => JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::Exceptions),
            "InnerClasses" => JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::InnerClasses),
            "EnclosingMethod" => JvmElement::parse(reader, ctx).map(Self::EnclosingMethod),
            "Synthetic" => Ok(Attribute::Synthetic),
            "Deprecated" => Ok(Attribute::Deprecated),
            "Signature" => JvmElement::parse(reader, ctx).map(Self::Signature),
            "SourceFile" => JvmElement::parse(reader, ctx).map(Self::SourceFile),
            "SourceDebugExtension" => {
                JvmElement::parse(reader, ctx).map(Self::SourceDebugExtension)
            }
            "LineNumberTable" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::LineNumberTable)
            }
            "LocalVariableTable" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::LocalVariableTable)
            }
            "LocalVariableTypeTable" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::LocalVariableTypeTable)
            }
            "RuntimeVisibleAnnotations" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                JvmElement::parse_vec::<u16, _>(reader, ctx).map(Self::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => {
                let num_parameters: u8 = reader.read_value()?;
                repeat_with(|| JvmElement::parse_vec::<u16, _>(reader, ctx))
                    .take(num_parameters.into())
                    .collect::<Result<_, _>>()
                    .map(Self::RuntimeVisibleParameterAnnotations)
            }
            "RuntimeInvisibleParameterAnnotations" => {
                let num_parameters: u8 = reader.read_value()?;
                repeat_with(|| JvmElement::parse_vec::<u16, _>(reader, ctx))
                    .take(num_parameters.into())
                    .collect::<Result<_, _>>()
                    .map(Self::RuntimeInvisibleParameterAnnotations)
            }
            "RuntimeVisibleTypeAnnotations" => JvmElement::parse_vec::<u16, _>(reader, ctx)
                .map(Self::RuntimeVisibleTypeAnnotations),
            "RuntimeInvisibleTypeAnnotations" => JvmElement::parse_vec::<u16, _>(reader, ctx)
                .map(Self::RuntimeInvisibleTypeAnnotations),
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
        let mut should_not_be_filled = [0u8; 1];
        match reader.read(&mut should_not_be_filled) {
            Ok(0) => Ok(result),
            Ok(_) => Err(Error::UnexpectedData),
            Err(e) => Err(e.into()),
        }
    }
}

impl JvmElement for ConstantValue {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let value_index = reader.read_value()?;
        ctx.constant_pool.get_constant_value(value_index)
    }
}

impl JvmElement for EnclosingMethod {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let class_index = reader.read_value()?;
        let class = ctx.constant_pool.get_class_ref(class_index)?;
        let method_index = reader.read_value()?;
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
