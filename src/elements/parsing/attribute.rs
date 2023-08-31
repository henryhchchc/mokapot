use crate::{
    elements::{
        annotation::{Annotation, ElementValue, TypeAnnotation},
        class::{BootstrapMethod, EnclosingMethod, InnerClassInfo, RecordComponent},
        field::ConstantValue,
        method::{
            LineNumberTableEntry, LocalVariableDescAttr, LocalVariableTypeAttr, MethodBody,
            MethodParameter, StackMapFrame,
        },
        module::Module,
        references::{ClassReference, PackageReference},
    },
    utils::{read_u16, read_u32},
};

use super::{
    constant_pool::{ConstantPool, ConstantPoolEntry},
    error::ClassFileParsingError,
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
        constant_pool: &ConstantPool,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let attributes_count = read_u16(reader)?;
        let entries = (0..attributes_count)
            .map(|_| Attribute::parse(reader, constant_pool))
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
    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_idx = read_u16(reader)?;
        let name = constant_pool.get_string(&name_idx)?;
        match name.as_str() {
            "ConstantValue" => Self::parse_constant_value(reader, constant_pool),
            "Code" => Self::parse_code(reader, constant_pool),
            "StackMapTable" => Self::parse_stack_map_table(reader, constant_pool),
            "Exceptions" => Self::parse_exceptions(reader, constant_pool),
            "InnerClasses" => Self::parse_innner_classes(reader, constant_pool),
            "EnclosingMethod" => Self::parse_enclosing_method(reader, constant_pool),
            "Synthetic" => Self::parse_synthetic(reader, constant_pool),
            "Signature" => Self::parse_signature(reader, constant_pool),
            "SourceFile" => Self::parse_source_file(reader, constant_pool),
            "SourceDebugExtension" => Self::parse_source_debug_extension(reader, constant_pool),
            "LineNumberTable" => Self::parse_line_no_table(reader, constant_pool),
            "LocalVariableTable" => Self::parse_local_variable_table(reader, constant_pool),
            "LocalVariableTypeTable" => {
                Self::parse_local_variable_type_table(reader, constant_pool)
            }
            "Deprecated" => Self::parse_deprecated(reader, constant_pool),
            "RuntimeVisibleAnnotations" => {
                let attribute_length = read_u32(reader)?;
                Self::parse_annotations(reader, constant_pool, Some(attribute_length))
                    .map(Self::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                let attribute_length = read_u32(reader)?;
                Self::parse_annotations(reader, constant_pool, Some(attribute_length))
                    .map(Self::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => {
                Self::parse_parameter_annotations(reader, constant_pool)
                    .map(Self::RuntimeVisibleParameterAnnotations)
            }
            "RuntimeInvisibleParameterAnnotations" => {
                Self::parse_parameter_annotations(reader, constant_pool)
                    .map(Self::RuntimeInvisibleParameterAnnotations)
            }
            "RuntimeVisibleTypeAnnotations" => Self::parse_type_annotations(reader, constant_pool)
                .map(Self::RuntimeVisibleTypeAnnotations),
            "RuntimeInvisibleTypeAnnotations" => {
                Self::parse_type_annotations(reader, constant_pool)
                    .map(Self::RuntimeInvisibleTypeAnnotations)
            }
            "AnnotationDefault" => Self::parse_annotation_default(reader, constant_pool),
            "BootstrapMethods" => Self::parse_bootstrap_methods(reader, constant_pool),
            "MethodParameters" => Self::parse_method_parameters(reader, constant_pool),
            "Module" => Self::parse_module(reader, constant_pool),
            "ModulePackages" => Self::parse_module_packages(reader, constant_pool),
            "ModuleMainClass" => Self::parse_module_main_class(reader, constant_pool),
            "NestHost" => Self::parse_nest_host(reader, constant_pool),
            "NestMembers" => Self::parse_nest_members(reader, constant_pool),
            "Record" => Self::parse_record(reader, constant_pool),
            "PermittedSubclasses" => Self::parse_permitted_subclasses(reader, constant_pool),
            _ => Err(ClassFileParsingError::UnknownAttribute(name)),
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
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let value_index = read_u16(reader)?;
        let value = constant_pool.get_constant_value(&value_index)?;
        Ok(Self::ConstantValue(value))
    }

    fn parse_synthetic<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 0)?;
        Ok(Self::Synthetic)
    }

    fn parse_deprecated<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 0)?;
        Ok(Self::Deprecated)
    }

    fn parse_enclosing_method<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 4)?;
        let class_index = read_u16(reader)?;
        let class = constant_pool.get_class_ref(&class_index)?;
        let method_index = read_u16(reader)?;
        let method_name_and_desc = if method_index == 0 {
            None
        } else {
            let entry = constant_pool.get_entry(&method_index)?;
            let ConstantPoolEntry::NameAndType{ name_index, descriptor_index } = entry else {
                return Err(ClassFileParsingError::MismatchedConstantPoolEntryType{expected: "NameAndType", found: entry.type_name()});
            };
            let name = constant_pool.get_string(name_index)?;
            let descriptor = constant_pool.get_string(descriptor_index)?;
            Some((name, descriptor))
        };
        Ok(Self::EnclosingMethod(EnclosingMethod {
            class,
            method_name_and_desc,
        }))
    }

    fn parse_signature<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let signature_index = read_u16(reader)?;
        let signature = constant_pool.get_string(&signature_index)?;
        Ok(Self::Signature(signature))
    }
}
