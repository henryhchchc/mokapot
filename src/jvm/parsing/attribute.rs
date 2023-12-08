use std::{iter::repeat_with, str::FromStr};

use crate::jvm::{
    annotation::{Annotation, ElementValue, TypeAnnotation},
    class::{
        BootstrapMethod, ClassReference, EnclosingMethod, InnerClassInfo, RecordComponent,
        SourceDebugExtension,
    },
    code::{LineNumberTableEntry, MethodBody, StackMapFrame},
    field::ConstantValue,
    method::{MethodDescriptor, MethodParameter},
    module::{Module, PackageReference},
    ClassFileParsingError, ClassFileParsingResult,
};

use super::{
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    constant_pool::ConstantPoolEntry,
    jvm_element_parser::{parse_jvm_element, ParseJvmElement},
    parsing_context::ParsingContext,
    reader_utils::{read_byte_chunk, read_u16, read_u32, read_u8},
};

#[derive(Debug)]
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
    MethodParameters(Vec<MethodParameter>),
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
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let name_idx = read_u16(reader)?;
        let name = ctx.constant_pool.get_str(name_idx)?;
        let attribute_length = read_u32(reader)?;
        let attribute_bytes = read_byte_chunk(reader, attribute_length as usize)?;
        let reader = &mut std::io::Cursor::new(attribute_bytes);
        let result = match name {
            "ConstantValue" => parse_jvm_element(reader, ctx).map(Self::ConstantValue),
            "Code" => parse_jvm_element(reader, ctx).map(Self::Code),
            "StackMapTable" => parse_jvm_element(reader, ctx).map(Self::StackMapTable),
            "Exceptions" => parse_jvm_element(reader, ctx).map(Self::Exceptions),
            "InnerClasses" => parse_jvm_element(reader, ctx).map(Self::InnerClasses),
            "EnclosingMethod" => parse_jvm_element(reader, ctx).map(Self::EnclosingMethod),
            "Synthetic" => match attribute_length {
                0 => Ok(Attribute::Synthetic),
                _ => Err(ClassFileParsingError::UnexpectedData),
            },
            "Deprecated" => match attribute_length {
                0 => Ok(Attribute::Deprecated),
                _ => Err(ClassFileParsingError::UnexpectedData),
            },
            "Signature" => parse_jvm_element(reader, ctx).map(Self::Signature),
            "SourceFile" => parse_jvm_element(reader, ctx).map(Self::SourceFile),
            "SourceDebugExtension" => {
                parse_jvm_element(reader, ctx).map(Self::SourceDebugExtension)
            }
            "LineNumberTable" => parse_jvm_element(reader, ctx).map(Self::LineNumberTable),
            "LocalVariableTable" => parse_jvm_element(reader, ctx).map(Self::LocalVariableTable),
            "LocalVariableTypeTable" => {
                parse_jvm_element(reader, ctx).map(Self::LocalVariableTypeTable)
            }
            "RuntimeVisibleAnnotations" => {
                parse_jvm_element(reader, ctx).map(Self::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                parse_jvm_element(reader, ctx).map(Self::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => {
                // NOTE: Unlike other attributes, the number of parameters is stored in a u8.
                let num_parameters = read_u8(reader)?;
                let param_annos = repeat_with(|| parse_jvm_element(reader, ctx))
                    .take(num_parameters as usize)
                    .collect::<Result<_, _>>()?;
                Ok(Self::RuntimeVisibleParameterAnnotations(param_annos))
            }
            "RuntimeInvisibleParameterAnnotations" => {
                // NOTE: Unlike other attributes, the number of parameters is stored in a u8.
                let num_parameters = read_u8(reader)?;
                let param_annos = repeat_with(|| parse_jvm_element(reader, ctx))
                    .take(num_parameters as usize)
                    .collect::<Result<_, _>>()?;
                Ok(Self::RuntimeInvisibleParameterAnnotations(param_annos))
            }
            "RuntimeVisibleTypeAnnotations" => {
                parse_jvm_element(reader, ctx).map(Self::RuntimeVisibleTypeAnnotations)
            }
            "RuntimeInvisibleTypeAnnotations" => {
                parse_jvm_element(reader, ctx).map(Self::RuntimeInvisibleTypeAnnotations)
            }
            "AnnotationDefault" => parse_jvm_element(reader, ctx).map(Self::AnnotationDefault),
            "BootstrapMethods" => parse_jvm_element(reader, ctx).map(Self::BootstrapMethods),
            "MethodParameters" => parse_jvm_element(reader, ctx).map(Self::MethodParameters),
            "Module" => parse_jvm_element(reader, ctx).map(Self::Module),
            "ModulePackages" => parse_jvm_element(reader, ctx).map(Self::ModulePackages),
            "ModuleMainClass" => parse_jvm_element(reader, ctx).map(Self::ModuleMainClass),
            "NestHost" => parse_jvm_element(reader, ctx).map(Self::NestHost),
            "NestMembers" => parse_jvm_element(reader, ctx).map(Self::NestMembers),
            "Record" => parse_jvm_element(reader, ctx).map(Self::Record),
            "PermittedSubclasses" => parse_jvm_element(reader, ctx).map(Self::PermittedSubclasses),
            unexpected => Err(ClassFileParsingError::UnknownAttribute(
                unexpected.to_owned(),
            )),
        };
        result.and_then(|attribute| {
            if reader.position() == attribute_length as u64 {
                Ok(attribute)
            } else {
                Err(ClassFileParsingError::UnexpectedData)
            }
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ConstantValue {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let value_index = read_u16(reader)?;
        ctx.constant_pool.get_constant_value(value_index)
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for EnclosingMethod {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let class_index = read_u16(reader)?;
        let class = ctx.constant_pool.get_class_ref(class_index)?;
        let method_index = read_u16(reader)?;
        let method_name_and_desc = if method_index == 0 {
            None
        } else {
            let entry = ctx.constant_pool.get_entry_internal(method_index)?;
            let &ConstantPoolEntry::NameAndType {
                name_index,
                descriptor_index,
            } = entry
            else {
                return Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                    expected: "NameAndType",
                    found: entry.constant_kind(),
                });
            };
            let name = ctx.constant_pool.get_str(name_index)?.to_owned();
            let descriptor_str = ctx.constant_pool.get_str(descriptor_index)?;
            let descriptor = MethodDescriptor::from_str(descriptor_str)?;
            Some((name, descriptor))
        };
        Ok(EnclosingMethod {
            class,
            method_name_and_desc,
        })
    }
}
