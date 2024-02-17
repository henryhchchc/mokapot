use std::{
    io::{Cursor, Read},
    iter::repeat_with,
    str::FromStr,
    usize,
};

use crate::jvm::{
    annotation::{Annotation, ElementValue, TypeAnnotation},
    class::{
        BootstrapMethod, ClassReference, EnclosingMethod, InnerClassInfo, RecordComponent,
        SourceDebugExtension,
    },
    code::{LineNumberTableEntry, MethodBody, StackMapFrame},
    field::ConstantValue,
    method::{MethodDescriptor, ParameterInfo},
    module::{Module, PackageReference},
};

use super::{
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    jvm_element_parser::JvmElement,
    parsing_context::ParsingContext,
    reader_utils::{read_byte_chunk, ValueReaderExt},
    Error,
};

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum Attribute {
    ConstantValue(ConstantValue),
    Code(MethodBody),
    StackMapTable(Vec<StackMapFrame>),
    Exceptions(Vec<ClassReference>),
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
    RuntimeVisibleTypeAnnotations(Vec<TypeAnnotation>),
    RuntimeInvisibleTypeAnnotations(Vec<TypeAnnotation>),
    AnnotationDefault(ElementValue),
    BootstrapMethods(Vec<BootstrapMethod>),
    MethodParameters(Vec<ParameterInfo>),
    Module(Module),
    ModulePackages(Vec<PackageReference>),
    ModuleMainClass(ClassReference),
    NestHost(ClassReference),
    NestMembers(Vec<ClassReference>),
    Record(Vec<RecordComponent>),
    PermittedSubclasses(Vec<ClassReference>),
}

impl Attribute {
    pub const fn name<'a>(&self) -> &'a str {
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
        }
    }
}

impl JvmElement for Attribute {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let name_idx = reader.read_value()?;
        let name = ctx.constant_pool.get_str(name_idx)?;
        let attribute_length: u32 = reader.read_value()?;
        let reader = {
            let attribute_length = usize::try_from(attribute_length)
                .expect("32-bit size is not supported on the current platform");
            let attribute_bytes = read_byte_chunk(reader, attribute_length)?;
            &mut Cursor::new(attribute_bytes)
        };

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
            unexpected => Err(Error::UnknownAttribute(unexpected.to_owned())),
        };
        result.and_then(|attribute| {
            let bytes_read = u32::try_from(reader.position())
                .expect("The size of the attribute should fit in u32");
            if bytes_read == attribute_length {
                Ok(attribute)
            } else {
                Err(Error::UnexpectedData)
            }
        })
    }
}

impl JvmElement for ConstantValue {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let value_index = reader.read_value()?;
        ctx.constant_pool.get_constant_value(value_index)
    }
}

impl JvmElement for EnclosingMethod {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let class_index = reader.read_value()?;
        let class = ctx.constant_pool.get_class_ref(class_index)?;
        let method_index = reader.read_value()?;
        let method_name_and_desc = if method_index > 0 {
            let (name, descriptor) = ctx.constant_pool.get_name_and_type(method_index)?;
            let descriptor = MethodDescriptor::from_str(descriptor)?;
            Some((name.to_owned(), descriptor))
        } else {
            None
        };
        Ok(EnclosingMethod {
            class,
            method_name_and_desc,
        })
    }
}
