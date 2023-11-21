use crate::{
    elements::{
        annotation::{Annotation, ElementValue, TypeAnnotation},
        class::{BootstrapMethod, EnclosingMethod, InnerClassInfo, RecordComponent},
        field::ConstantValue,
        instruction::{LineNumberTableEntry, MethodBody, StackMapFrame},
        method::MethodParameter,
        module::Module,
        references::{ClassReference, PackageReference},
    },
    errors::ClassFileParsingError,
    reader_utils::{read_u16, read_u32},
};

use super::{
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    constant_pool::ConstantPoolEntry,
    parsing_context::ParsingContext,
};

#[derive(Debug)]
pub(crate) struct AttributeList {
    entries: Vec<Attribute>,
}

impl IntoIterator for AttributeList {
    type Item = Attribute;

    type IntoIter = <Vec<Attribute> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl AttributeList {
    pub(crate) fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let attributes_count = read_u16(reader)?;
        let entries = (0..attributes_count)
            .map(|_| Attribute::parse(reader, ctx))
            .collect::<Result<_, ClassFileParsingError>>()?;
        Ok(Self { entries })
    }
}

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
    pub fn name<'a>(&self) -> &'a str {
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

    fn parse<R>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_idx = read_u16(reader)?;
        let name = ctx.constant_pool.get_str(name_idx)?;
        match name {
            "ConstantValue" => Self::parse_constant_value(reader, ctx),
            "Code" => Self::parse_code(reader, ctx),
            "StackMapTable" => Self::parse_stack_map_table(reader, ctx),
            "Exceptions" => Self::parse_exceptions(reader, ctx),
            "InnerClasses" => Self::parse_innner_classes(reader, ctx),
            "EnclosingMethod" => Self::parse_enclosing_method(reader, ctx),
            "Synthetic" => Self::parse_synthetic(reader, ctx),
            "Signature" => Self::parse_signature(reader, ctx),
            "SourceFile" => Self::parse_source_file(reader, ctx),
            "SourceDebugExtension" => Self::parse_source_debug_extension(reader, ctx),
            "LineNumberTable" => Self::parse_line_no_table(reader, ctx),
            "LocalVariableTable" => Self::parse_local_variable_table(reader, ctx),
            "LocalVariableTypeTable" => Self::parse_local_variable_type_table(reader, ctx),
            "Deprecated" => Self::parse_deprecated(reader, ctx),
            "RuntimeVisibleAnnotations" => {
                let attribute_length = read_u32(reader)?;
                Self::parse_annotations(reader, ctx, Some(attribute_length))
                    .map(Self::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                let attribute_length = read_u32(reader)?;
                Self::parse_annotations(reader, ctx, Some(attribute_length))
                    .map(Self::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => Self::parse_parameter_annotations(reader, ctx)
                .map(Self::RuntimeVisibleParameterAnnotations),
            "RuntimeInvisibleParameterAnnotations" => {
                Self::parse_parameter_annotations(reader, ctx)
                    .map(Self::RuntimeInvisibleParameterAnnotations)
            }
            "RuntimeVisibleTypeAnnotations" => {
                Self::parse_type_annotations(reader, ctx).map(Self::RuntimeVisibleTypeAnnotations)
            }
            "RuntimeInvisibleTypeAnnotations" => {
                Self::parse_type_annotations(reader, ctx).map(Self::RuntimeInvisibleTypeAnnotations)
            }
            "AnnotationDefault" => Self::parse_annotation_default(reader, ctx),
            "BootstrapMethods" => Self::parse_bootstrap_methods(reader, ctx),
            "MethodParameters" => Self::parse_method_parameters(reader, ctx),
            "Module" => Self::parse_module(reader, ctx),
            "ModulePackages" => Self::parse_module_packages(reader, ctx),
            "ModuleMainClass" => Self::parse_module_main_class(reader, ctx),
            "NestHost" => Self::parse_nest_host(reader, ctx),
            "NestMembers" => Self::parse_nest_members(reader, ctx),
            "Record" => Self::parse_record(reader, ctx),
            "PermittedSubclasses" => Self::parse_permitted_subclasses(reader, ctx),
            _ => Err(ClassFileParsingError::UnknownAttribute(name.to_owned())),
        }
    }

    pub fn check_attribute_length<R>(
        reader: &mut R,
        expected: u32,
    ) -> Result<(), ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let attribute_length = read_u32(reader)?;
        if attribute_length != expected {
            return Err(ClassFileParsingError::InvalidAttributeLength {
                expected,
                actual: attribute_length,
            });
        }
        Ok(())
    }

    fn parse_constant_value<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let value_index = read_u16(reader)?;
        let value = ctx.constant_pool.get_constant_value(value_index)?;
        Ok(Self::ConstantValue(value))
    }

    fn parse_synthetic<R>(
        reader: &mut R,
        _ctx: &ParsingContext,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 0)?;
        Ok(Self::Synthetic)
    }

    fn parse_deprecated<R>(
        reader: &mut R,
        _ctx: &ParsingContext,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 0)?;
        Ok(Self::Deprecated)
    }

    fn parse_enclosing_method<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 4)?;
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
            let descriptor = ctx.constant_pool.get_str(descriptor_index)?.to_owned();
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
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let signature_index = read_u16(reader)?;
        let signature = ctx.constant_pool.get_str(signature_index)?.to_owned();
        Ok(Self::Signature(signature))
    }
}
