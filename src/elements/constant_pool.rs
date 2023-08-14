use std::collections::HashMap;

use crate::utils::{read_bytes, read_u16, read_u8};

use super::{class_file::{ClassFileParsingError, ClassReference}, fields::ConstantValue};


#[derive(Debug)]
pub struct ConstantPool {
    entries: HashMap<u16, ConstantPoolEntry>,
}

impl ConstantPool {
    pub fn parse<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let constant_pool_count = read_u16(reader)?;
        let entries = ConstantPoolEntry::parse_multiple(reader, constant_pool_count)?;

        Ok(Self { entries })
    }

    pub fn get_entry(&self, index: impl Into<u16>) -> Result<&ConstantPoolEntry, ClassFileParsingError> {
        let Some(entry) = self.entries.get(&index.into()) else { 
            return Err(ClassFileParsingError::BadConstantPoolIndex);
        };
        Ok(entry)
    }

    pub fn get_string(&self, index: impl Into<u16>) -> Result<String, ClassFileParsingError> {
        if let ConstantPoolEntry::Utf8(string) = self.get_entry(index)? {
            Ok(string.clone())
        } else {
            Err(ClassFileParsingError::MidmatchedConstantPoolTag)
        }
    }

    pub fn get_class_ref(&self, index: impl Into<u16>) -> Result<ClassReference, ClassFileParsingError> {
        let ConstantPoolEntry::Class { name_index } = self.get_entry(index)? else {
            return Err(ClassFileParsingError::MidmatchedConstantPoolTag);
        };
        let name = self.get_string(*name_index)?;
        Ok(ClassReference { name })
    }

    pub(crate) fn get_constant_value(&self, value_index: u16) -> Result<ConstantValue, ClassFileParsingError> {
        let entry = self.get_entry(value_index)?;
        match entry {
            ConstantPoolEntry::Integer(it) => Ok(ConstantValue::Integer(*it)),
            ConstantPoolEntry::Long(it) => Ok(ConstantValue::Long(*it)),
            ConstantPoolEntry::Float(it) => Ok(ConstantValue::Float(*it)),
            ConstantPoolEntry::Double(it) => Ok(ConstantValue::Double(*it)),
            ConstantPoolEntry::String { string_index } => self.get_string(*string_index).map(ConstantValue::String),
            _ => Err(ClassFileParsingError::MidmatchedConstantPoolTag)
        }
    }

}

#[derive(Debug, Clone)]
pub enum ConstantPoolEntry {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class {
        name_index: u16,
    },
    String {
        string_index: u16,
    },
    FieldRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    MethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    InterfaceMethodRef {
        class_index: u16,
        name_and_type_index: u16,
    },
    NameAndType {
        name_index: u16,
        descriptor_index: u16,
    },
    MethodHandle {
        reference_kind: u8,
        reference_index: u16,
    },
    MethodType {
        descriptor_index: u16,
    },
    Dynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
    InvokeDynamic {
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
    Module {
        name_index: u16,
    },
    Package {
        name_index: u16,
    },
}

impl ConstantPoolEntry {
    fn parse_multiple<R>(
        reader: &mut R,
        count: u16,
    ) -> Result<HashMap<u16, Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut counter: u16 = 1;
        let mut result = HashMap::with_capacity(count as usize);
        while counter < count {
            let entry = Self::parse(reader)?;
            let increment = match entry {
                ConstantPoolEntry::Long(_) | ConstantPoolEntry::Double(_) => 2,
                _ => 1,
            };
            result.insert(counter, entry);
            counter += increment;
        }
        Ok(result)
    }

    fn parse<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let tag = read_u8(reader)?;
        match tag {
            1 => Self::parse_utf8(reader),
            3 => Self::parse_integer(reader),
            4 => Self::parse_float(reader),
            5 => Self::parse_long(reader),
            6 => Self::parse_double(reader),
            7 => Self::parse_class(reader),
            8 => Self::parse_string(reader),
            9 => Self::parse_field_ref(reader),
            10 => Self::parse_method_ref(reader),
            11 => Self::parse_interface_method_ref(reader),
            12 => Self::parse_name_and_type(reader),
            15 => Self::parse_method_handle(reader),
            16 => Self::parse_method_type(reader),
            17 => Self::parse_dynamic(reader),
            18 => Self::parse_invoke_dynamic(reader),
            19 => Self::parse_module(reader),
            20 => Self::parse_package(reader),
            _ => Err(ClassFileParsingError::MalformedClassFile),
        }
    }

    fn parse_utf8<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let length = read_u16(reader)?;
        let mut bytes = Vec::with_capacity(length as usize);
        for _ in 0..length {
            bytes.push(read_u8(reader)?);
        }
        if let Ok(result) = String::from_utf8(bytes) {
            Ok(Self::Utf8(result))
        } else {
            Err(ClassFileParsingError::MalformedClassFile)
        }
    }

    fn parse_integer<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Integer(i32::from_be_bytes(bytes)))
    }

    fn parse_float<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Float(f32::from_be_bytes(bytes)))
    }

    fn parse_long<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Long(i64::from_be_bytes(bytes)))
    }

    fn parse_double<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Double(f64::from_be_bytes(bytes)))
    }

    fn parse_class<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Class { name_index })
    }

    fn parse_string<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let string_index = read_u16(reader)?;
        Ok(Self::String { string_index })
    }

    fn parse_field_ref<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let class_index = read_u16(reader)?;
        let name_and_type_index = read_u16(reader)?;
        Ok(Self::FieldRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_method_ref<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let class_index = read_u16(reader)?;
        let name_and_type_index = read_u16(reader)?;
        Ok(Self::MethodRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_interface_method_ref<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let class_index = read_u16(reader)?;
        let name_and_type_index = read_u16(reader)?;
        Ok(Self::InterfaceMethodRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_name_and_type<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        let descriptor_index = read_u16(reader)?;
        Ok(Self::NameAndType {
            name_index,
            descriptor_index,
        })
    }

    fn parse_method_handle<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let reference_kind = read_u8(reader)?;
        let reference_index = read_u16(reader)?;
        Ok(Self::MethodHandle {
            reference_kind,
            reference_index,
        })
    }

    fn parse_method_type<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let descriptor_index = read_u16(reader)?;
        Ok(Self::MethodType { descriptor_index })
    }

    fn parse_dynamic<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bootstrap_method_attr_index = read_u16(reader)?;
        let name_and_type_index = read_u16(reader)?;
        Ok(Self::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    fn parse_invoke_dynamic<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bootstrap_method_attr_index = read_u16(reader)?;
        let name_and_type_index = read_u16(reader)?;
        Ok(Self::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    fn parse_module<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Module { name_index })
    }

    fn parse_package<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Package { name_index })
    }
}
