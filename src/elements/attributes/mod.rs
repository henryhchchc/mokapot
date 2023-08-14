mod methods;

use crate::utils::{read_bytes_vec, read_u16, read_u32};

use self::methods::{
    Code, LineNumberTableEntry, LocalVariableTableEntry,
    LocalVariableTypeTableEntry, StackMapTableEntry,
};

use super::{
    class_file::{ClassFileParsingError, ClassReference},
    constant_pool::{ConstantPool, ConstantPoolEntry},
    fields::ConstantValue,
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
        let mut entries = Vec::with_capacity(attributes_count as usize);
        for _i in 0..attributes_count {
            let attribute = Attribute::parse(reader, constant_pool)?;
            entries.push(attribute);
        }
        Ok(Self { entries })
    }
}

#[derive(Debug)]
pub struct InnerClassInfo {
    pub inner_class: ClassReference,
    pub outer_class: Option<ClassReference>,
    pub inner_name: String,
    pub inner_class_access_flags: u16,
}

#[derive(Debug)]
pub(crate) enum Attribute {
    ConstantValue(ConstantValue),
    Code(Code),
    Exceptions(Vec<ClassReference>),
    SourceFile(String),
    LineNumberTable(Vec<LineNumberTableEntry>),
    InnerClasses(Vec<InnerClassInfo>),
    Synthetic,
    Deprecated,
    EnclosingMethod {
        class: ClassReference,
        method_name_and_desc: Option<(String, String)>,
    },
    Signature(String),
    SourceDebugExtension(Vec<u8>),
    LocalVariableTable(Vec<LocalVariableTableEntry>),
    LocalVariableTypeTable(Vec<LocalVariableTypeTableEntry>),
    RuntimeVisibleAnnotations,
    RuntimeInvisibleAnnotations,
    RuntimeVisibleParameterAnnotations,
    RuntimeInvisibleParameterAnnotations,
    AnnotationDefault,
    StackMapTable(Vec<StackMapTableEntry>),
    BootstrapMethods,
    RuntimeVisibleTypeAnnotations,
    RuntimeInvisibleTypeAnnotations,
    MethodParameters,
    Module,
    ModulePackages,
    ModuleMainClass,
    NestHost,
    NestMembers,
    Record,
    PermittedSubclasses,
}

impl Attribute {
    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_idx = read_u16(reader)?;
        let name = constant_pool.get_string(name_idx)?;
        match name.as_str() {
            "ConstantValue" => Self::parse_constant_value(reader, constant_pool),
            "Code" => Self::parse_code(reader, constant_pool),
            "Exceptions" => Self::parse_exceptions(reader, constant_pool),
            "InnerClasses" => Self::parse_innner_classes(reader, constant_pool),
            "Synthetic" => Self::parse_synthetic(reader, constant_pool),
            "Deprecated" => Self::parse_deprecated(reader, constant_pool),
            "EnclosingMethod" => Self::parse_enclosing_method(reader, constant_pool),
            "Signature" => Self::parse_signature(reader, constant_pool),
            "SourceFile" => Self::parse_source_file(reader, constant_pool),
            "SourceDebugExtension" => Self::parse_source_debug_extension(reader, constant_pool),
            "LineNumberTable" => Self::parse_line_no_table(reader, constant_pool),
            "LocalVariableTable" => Self::parse_local_variable_table(reader, constant_pool),
            "LocalVariableTypeTable" => {
                Self::parse_local_variable_type_table(reader, constant_pool)
            }
            // "RuntimeVisibleAnnotations" => todo!(),
            // "RuntimeInvisibleAnnotations" => todo!(),
            // "RuntimeVisibleParameterAnnotations" => todo!(),
            // "RuntimeInvisibleParameterAnnotations" => todo!(),
            // "AnnotationDefault" => todo!(),
            // "StackMapTable" => todo!(),
            // "BootstrapMethods" => todo!(),
            // "RuntimeVisibleTypeAnnotations" => todo!(),
            // "RuntimeInvisibleTypeAnnotations" => todo!(),
            // "MethodParameters" => todo!(),
            // "Module" => todo!(),
            // "ModulePackages" => todo!(),
            // "ModuleMainClass" => todo!(),
            // "NestHost" => todo!(),
            // "NestMembers" => todo!(),
            // "Record" => todo!(),
            // "PermittedSubclasses" => todo!(),
            _ => Err(ClassFileParsingError::UnknownAttributeName(name)),
        }
    }

    fn check_attribute_length<R>(reader: &mut R, expected: u32) -> Result<(), ClassFileParsingError>
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

    fn parse_source_file<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let sourcefile_index = read_u16(reader)?;
        let file_name = constant_pool.get_string(sourcefile_index)?;
        Ok(Self::SourceFile(file_name))
    }

    fn parse_exceptions<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let number_of_exceptions = read_u16(reader)?;
        let mut exceptions = Vec::with_capacity(number_of_exceptions as usize);
        for _ in 0..number_of_exceptions {
            let exception_index = read_u16(reader)?;
            let exception = constant_pool.get_class_ref(exception_index)?;
            exceptions.push(exception);
        }
        Ok(Attribute::Exceptions(exceptions))
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
        let value = constant_pool.get_constant_value(value_index)?;
        Ok(Attribute::ConstantValue(value))
    }

    fn parse_innner_classes<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let number_of_classes = read_u16(reader)?;
        let mut classes = Vec::with_capacity(number_of_classes as usize);
        for _ in 0..number_of_classes {
            let inner_class_info_index = read_u16(reader)?;
            let inner_class = constant_pool.get_class_ref(inner_class_info_index)?;
            let outer_class_info_index = read_u16(reader)?;
            let outer_class = if outer_class_info_index == 0 {
                None
            } else {
                let the_class = constant_pool.get_class_ref(outer_class_info_index)?;
                Some(the_class)
            };
            let inner_name_index = read_u16(reader)?;
            let inner_name = constant_pool.get_string(inner_name_index)?;
            let inner_class_access_flags = read_u16(reader)?;
            classes.push(InnerClassInfo {
                inner_class,
                outer_class,
                inner_name,
                inner_class_access_flags,
            });
        }
        Ok(Attribute::InnerClasses(classes))
    }

    fn parse_synthetic<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 0)?;
        Ok(Attribute::Synthetic)
    }

    fn parse_deprecated<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 0)?;
        Ok(Attribute::Deprecated)
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
        let class = constant_pool.get_class_ref(class_index)?;
        let method_index = read_u16(reader)?;
        let method_name_and_desc = if method_index == 0 {
            None
        } else {
            let ConstantPoolEntry::NameAndType{ name_index, descriptor_index } = constant_pool.get_entry(method_index)? else {
                return Err(ClassFileParsingError::MidmatchedConstantPoolTag);
            };
            let name = constant_pool.get_string(*name_index)?;
            let descriptor = constant_pool.get_string(*descriptor_index)?;
            Some((name, descriptor))
        };
        Ok(Attribute::EnclosingMethod {
            class,
            method_name_and_desc,
        })
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
        let signature = constant_pool.get_string(signature_index)?;
        Ok(Attribute::Signature(signature))
    }

    fn parse_source_debug_extension<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let attribute_length = read_u32(reader)?;
        let debug_extension = read_bytes_vec(reader, attribute_length as usize)?;
        Ok(Attribute::SourceDebugExtension(debug_extension))
    }
}
