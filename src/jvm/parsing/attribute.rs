use std::str::FromStr;

use crate::jvm::{
    annotation::{Annotation, ElementValue, TypeAnnotation},
    class::{BootstrapMethod, ClassReference, EnclosingMethod, InnerClassInfo, RecordComponent},
    code::{LineNumberTableEntry, MethodBody, StackMapFrame},
    field::ConstantValue,
    method::{MethodDescriptor, MethodParameter},
    module::{Module, PackageReference},
    ClassFileParsingError,
};

use super::{
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    constant_pool::ConstantPoolEntry,
    parsing_context::ParsingContext,
    reader_utils::{read_bytes_vec, read_u16, read_u32},
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

    pub(crate) fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_idx = read_u16(reader)?;
        let name = ctx.constant_pool.get_str(name_idx)?;
        let attribute_length = read_u32(reader)?;
        let attribute_bytes = read_bytes_vec(reader, attribute_length as usize)?;
        let attr_reader = &mut std::io::Cursor::new(attribute_bytes);
        let result = match name {
            "ConstantValue" => Self::parse_constant_value(attr_reader, ctx),
            "Code" => Self::parse_code(attr_reader, ctx),
            "StackMapTable" => Self::parse_stack_map_table(attr_reader, ctx),
            "Exceptions" => Self::parse_exceptions(attr_reader, ctx),
            "InnerClasses" => Self::parse_innner_classes(attr_reader, ctx),
            "EnclosingMethod" => Self::parse_enclosing_method(attr_reader, ctx),
            "Synthetic" => Self::parse_synthetic(attr_reader, ctx),
            "Signature" => Self::parse_signature(attr_reader, ctx),
            "SourceFile" => Self::parse_source_file(attr_reader, ctx),
            "SourceDebugExtension" => Self::parse_source_debug_extension(attr_reader, ctx),
            "LineNumberTable" => Self::parse_line_no_table(attr_reader, ctx),
            "LocalVariableTable" => Self::parse_local_variable_table(attr_reader, ctx),
            "LocalVariableTypeTable" => Self::parse_local_variable_type_table(attr_reader, ctx),
            "Deprecated" => Self::parse_deprecated(attr_reader, ctx),
            "RuntimeVisibleAnnotations" => {
                Self::parse_annotations(attr_reader, ctx).map(Self::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                Self::parse_annotations(attr_reader, ctx).map(Self::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => {
                Self::parse_parameter_annotations(attr_reader, ctx)
                    .map(Self::RuntimeVisibleParameterAnnotations)
            }
            "RuntimeInvisibleParameterAnnotations" => {
                Self::parse_parameter_annotations(attr_reader, ctx)
                    .map(Self::RuntimeInvisibleParameterAnnotations)
            }
            "RuntimeVisibleTypeAnnotations" => Self::parse_type_annotations(attr_reader, ctx)
                .map(Self::RuntimeVisibleTypeAnnotations),
            "RuntimeInvisibleTypeAnnotations" => Self::parse_type_annotations(attr_reader, ctx)
                .map(Self::RuntimeInvisibleTypeAnnotations),
            "AnnotationDefault" => Self::parse_annotation_default(attr_reader, ctx),
            "BootstrapMethods" => Self::parse_bootstrap_methods(attr_reader, ctx),
            "MethodParameters" => Self::parse_method_parameters(attr_reader, ctx),
            "Module" => Self::parse_module(attr_reader, ctx),
            "ModulePackages" => Self::parse_module_packages(attr_reader, ctx),
            "ModuleMainClass" => Self::parse_module_main_class(attr_reader, ctx),
            "NestHost" => Self::parse_nest_host(attr_reader, ctx),
            "NestMembers" => Self::parse_nest_members(attr_reader, ctx),
            "Record" => Self::parse_record(attr_reader, ctx),
            "PermittedSubclasses" => Self::parse_permitted_subclasses(attr_reader, ctx),
            _ => Err(ClassFileParsingError::UnknownAttribute(name.to_owned())),
        };
        if attr_reader.position() == attribute_length as u64 {
            result
        } else {
            Err(ClassFileParsingError::UnexpectedData)
        }
    }

    fn parse_constant_value<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let value_index = read_u16(reader)?;
        let value = ctx.constant_pool.get_constant_value(value_index)?;
        Ok(Self::ConstantValue(value))
    }

    fn parse_synthetic<R>(
        _reader: &mut R,
        _ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Ok(Self::Synthetic)
    }

    fn parse_deprecated<R>(
        _reader: &mut R,
        _ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Ok(Self::Deprecated)
    }

    fn parse_enclosing_method<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let class_index = read_u16(reader)?;
        let class = ctx.constant_pool.get_class_ref(class_index)?;
        let method_index = read_u16(reader)?;
        let method_name_and_desc = if method_index == 0 {
            None
        } else {
            let entry = ctx.constant_pool.get_entry(method_index)?;
            let &ConstantPoolEntry::NameAndType {
                name_index,
                descriptor_index,
            } = entry
            else {
                return Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                    expected: "NameAndType",
                    found: entry.type_name(),
                });
            };
            let name = ctx.constant_pool.get_str(name_index)?.to_owned();
            let descriptor_str = ctx.constant_pool.get_str(descriptor_index)?;
            let descriptor = MethodDescriptor::from_str(descriptor_str)?;
            Some((name, descriptor))
        };
        Ok(Self::EnclosingMethod(EnclosingMethod {
            class,
            method_name_and_desc,
        }))
    }

    fn parse_signature<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let signature_index = read_u16(reader)?;
        let signature = ctx.constant_pool.get_str(signature_index)?.to_owned();
        Ok(Self::Signature(signature))
    }
}
