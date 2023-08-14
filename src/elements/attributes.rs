use crate::utils::{read_bytes_vec, read_u16, read_u32};

use super::{
    class_file::{ClassFileParsingError, ClassReference},
    constant_pool::ConstantPool,
    fields::ConstantValue,
};

#[derive(Debug)]
pub struct AttributeInfo {
    pub(crate) attribute_name_index: u16,
    pub(crate) info: Vec<u8>,
}
impl AttributeInfo {
    pub(crate) fn parse_multiple<R>(
        reader: &mut R,
        attributes_count: u16,
    ) -> Result<Vec<AttributeInfo>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut attributes = Vec::with_capacity(attributes_count as usize);
        for _ in 0..attributes_count {
            attributes.push(Self::parse(reader)?);
        }
        Ok(attributes)
    }

    fn parse<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let attribute_name_index = read_u16(reader)?;
        let attribute_length = read_u32(reader)?;
        let info = read_bytes_vec(reader, attribute_length as usize)?;
        Ok(Self {
            attribute_name_index,
            info,
        })
    }
}

#[derive(Debug)]
pub struct ExceptionTableEntry {
    start_pc: u16,
    end_pc: u16,
    handler_pc: u16,
    catch_type: ClassReference,
}

#[derive(Debug)]
pub struct LineNumberTableEntry {
    start_pc: u16,
    line_number: u16,
}

#[derive(Debug)]
pub struct LocalVariableTableEntry {
    start_pc: u16,
    length: u16,
    name_index: u16,
    descriptor_index: u16,
    index: u16,
}

#[derive(Debug)]
pub struct StackMapTable {}

#[derive(Debug)]
pub struct Code {
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
    exception_table: Vec<ExceptionTableEntry>,
    line_number_table: Option<Vec<LineNumberTableEntry>>,
    local_variable_table: Option<Vec<LocalVariableTableEntry>>,
    stack_map_table: Option<StackMapTable>,
}

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
pub(crate) enum Attribute {
    ConstantValue(ConstantValue),
    Code(Code),
    Exceptions,
    SourceFile(String),
    LineNumberTable(Vec<LineNumberTableEntry>),
    InnerClasses,
    Synthetic,
    Deprecated,
    EnclodingMethod,
    Signature,
    SourceDebugExtension,
    LocalVariableTable,
    LocalVariableTypeTable,
    RuntimeVisibleAnnotations,
    RuntimeInvisibleAnnotations,
    RuntimeVisibleParameterAnnotations,
    RuntimeInvisibleParameterAnnotations,
    AnnotationDefault,
    StackMapTable,
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
            // "ConstantValue" => parse_constant_value(reader, constant_pool),
            "Code" => Self::parse_code(reader, constant_pool),
            // "Exceptions" => parse_exceptions(reader, constant_pool),
            // "Exceptions" => todo!(),
            "SourceFile" => Self::parse_source_file(reader, constant_pool),
            "LineNumberTable" => Self::parse_line_no_table(reader, constant_pool),
            // "InnerClasses" => todo!(),
            // "Synthetic" => todo!(),
            // "Deprecated" => todo!(),
            // "EnclodingMethod" => todo!(),
            // "Signature" => todo!(),
            // "SourceDebugExtension" => todo!(),
            // "LocalVariableTable" => todo!(),
            // "LocalVariableTypeTable" => todo!(),
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

    fn parse_code<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let max_stack = read_u16(reader)?;
        let max_locals = read_u16(reader)?;
        let code_length = read_u32(reader)?;

        let code = read_bytes_vec(reader, code_length as usize)?;

        // exception table
        let exception_table_len = read_u16(reader)?;
        let mut exception_table = Vec::with_capacity(exception_table_len as usize);
        for _ in 0..exception_table_len {
            let start_pc = read_u16(reader)?;
            let end_pc = read_u16(reader)?;
            let handler_pc = read_u16(reader)?;
            let catch_type_idx = read_u16(reader)?;

            let catch_type = constant_pool.get_class_ref(catch_type_idx)?;
            exception_table.push(ExceptionTableEntry {
                start_pc,
                end_pc,
                handler_pc,
                catch_type,
            })
        }
        let attributes = AttributeList::parse(reader, constant_pool)?;
        let mut line_number_table = None;
        let local_variable_table = None;
        let stack_map_table = None;

        for attr in attributes.into_iter() {
            match attr {
                Attribute::LineNumberTable(it) => line_number_table = Some(it),
                Attribute::LocalVariableTable => todo!(),
                Attribute::LocalVariableTypeTable => todo!(),
                Attribute::StackMapTable => todo!(),
                _ => {}
            }
        }

        Ok(Attribute::Code(Code {
            max_stack,
            max_locals,
            exception_table,
            code,
            line_number_table,
            local_variable_table,
            stack_map_table,
        }))
    }

    fn parse_line_no_table<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> Result<Attribute, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let line_number_table_len = read_u16(reader)?;
        let mut line_number_table = Vec::with_capacity(line_number_table_len as usize);
        for _ in 0..line_number_table_len {
            let start_pc = read_u16(reader)?;
            let line_number = read_u16(reader)?;
            line_number_table.push(LineNumberTableEntry {
                start_pc,
                line_number,
            });
        }
        Ok(Attribute::LineNumberTable(line_number_table))
    }
}
