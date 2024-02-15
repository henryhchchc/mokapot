use std::{iter::repeat_with, str::FromStr};

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
    jvm_element_parser::{parse_jvm, ParseJvmElement},
    parsing_context::ParsingContext,
    reader_utils::{read_byte_chunk, ClassReader},
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

impl<R: std::io::Read> ParseJvmElement<R> for Attribute {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let name_idx = reader.read_value()?;
        let name = ctx.constant_pool.get_str(name_idx)?;
        let attribute_length: u32 = reader.read_value()?;
        let attribute_bytes = read_byte_chunk(reader, attribute_length as usize)?;
        let reader = &mut std::io::Cursor::new(attribute_bytes);
        let result = match name {
            "ConstantValue" => parse_jvm!(reader, ctx).map(Self::ConstantValue),
            "Code" => parse_jvm!(reader, ctx).map(Self::Code),
            "StackMapTable" => parse_jvm!(u16, reader, ctx).map(Self::StackMapTable),
            "Exceptions" => parse_jvm!(u16, reader, ctx).map(Self::Exceptions),
            "InnerClasses" => parse_jvm!(u16, reader, ctx).map(Self::InnerClasses),
            "EnclosingMethod" => parse_jvm!(reader, ctx).map(Self::EnclosingMethod),
            "Synthetic" => Ok(Attribute::Synthetic),
            "Deprecated" => Ok(Attribute::Deprecated),
            "Signature" => parse_jvm!(reader, ctx).map(Self::Signature),
            "SourceFile" => parse_jvm!(reader, ctx).map(Self::SourceFile),
            "SourceDebugExtension" => parse_jvm!(reader, ctx).map(Self::SourceDebugExtension),
            "LineNumberTable" => parse_jvm!(u16, reader, ctx).map(Self::LineNumberTable),
            "LocalVariableTable" => parse_jvm!(u16, reader, ctx).map(Self::LocalVariableTable),
            "LocalVariableTypeTable" => {
                parse_jvm!(u16, reader, ctx).map(Self::LocalVariableTypeTable)
            }
            "RuntimeVisibleAnnotations" => {
                parse_jvm!(u16, reader, ctx).map(Self::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                parse_jvm!(u16, reader, ctx).map(Self::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => {
                let num_parameters: u8 = reader.read_value()?;
                repeat_with(|| parse_jvm!(u16, reader, ctx))
                    .take(num_parameters as usize)
                    .collect::<Result<_, _>>()
                    .map(Self::RuntimeVisibleParameterAnnotations)
            }
            "RuntimeInvisibleParameterAnnotations" => {
                let num_parameters: u8 = reader.read_value()?;
                repeat_with(|| parse_jvm!(u16, reader, ctx))
                    .take(num_parameters as usize)
                    .collect::<Result<_, _>>()
                    .map(Self::RuntimeInvisibleParameterAnnotations)
            }
            "RuntimeVisibleTypeAnnotations" => {
                parse_jvm!(u16, reader, ctx).map(Self::RuntimeVisibleTypeAnnotations)
            }
            "RuntimeInvisibleTypeAnnotations" => {
                parse_jvm!(u16, reader, ctx).map(Self::RuntimeInvisibleTypeAnnotations)
            }
            "AnnotationDefault" => parse_jvm!(reader, ctx).map(Self::AnnotationDefault),
            "BootstrapMethods" => parse_jvm!(u16, reader, ctx).map(Self::BootstrapMethods),
            "MethodParameters" => parse_jvm!(u8, reader, ctx).map(Self::MethodParameters),
            "Module" => parse_jvm!(reader, ctx).map(Self::Module),
            "ModulePackages" => parse_jvm!(u16, reader, ctx).map(Self::ModulePackages),
            "ModuleMainClass" => parse_jvm!(reader, ctx).map(Self::ModuleMainClass),
            "NestHost" => parse_jvm!(reader, ctx).map(Self::NestHost),
            "NestMembers" => parse_jvm!(u16, reader, ctx).map(Self::NestMembers),
            "Record" => parse_jvm!(u16, reader, ctx).map(Self::Record),
            "PermittedSubclasses" => parse_jvm!(u16, reader, ctx).map(Self::PermittedSubclasses),
            unexpected => Err(Error::UnknownAttribute(unexpected.to_owned())),
        };
        result.and_then(|attribute| {
            if reader.position() == u64::from(attribute_length) {
                Ok(attribute)
            } else {
                Err(Error::UnexpectedData)
            }
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ConstantValue {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let value_index = reader.read_value()?;
        ctx.constant_pool.get_constant_value(value_index)
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for EnclosingMethod {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
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
