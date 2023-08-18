use std::collections::HashMap;

use crate::{
    elements::{
        class::ClassVersion,
        class_parser::{ClassFileParsingError, ClassFileParsingResult},
        field::ConstantValue,
        references::{
            ClassReference, FieldReference, MethodReference, ModuleReference, PackageReference,
        },
    },
    utils::{read_bytes, read_bytes_vec, read_u16, read_u8},
};

#[derive(Debug)]
pub struct ConstantPool {
    entries: HashMap<u16, ConstantPoolEntry>,
    pub class_version: ClassVersion,
}

pub(crate) type ConstantPoolIndex = u16;

impl ConstantPool {
    pub fn parse<R>(reader: &mut R, class_version: ClassVersion) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let constant_pool_count = read_u16(reader)?;
        let entries = ConstantPoolEntry::parse_multiple(reader, constant_pool_count)?;

        Ok(Self {
            entries,
            class_version,
        })
    }

    pub fn get_entry(&self, index: &u16) -> ClassFileParsingResult<&ConstantPoolEntry> {
        let Some(entry) = self.entries.get(index) else { 
            return Err(ClassFileParsingError::BadConstantPoolIndex);
        };
        Ok(entry)
    }

    pub fn get_string(&self, index: &u16) -> ClassFileParsingResult<String> {
        if let ConstantPoolEntry::Utf8(string) = self.get_entry(index)? {
            Ok(string.clone())
        } else {
            Err(ClassFileParsingError::MidmatchedConstantPoolTag)
        }
    }

    pub fn get_class_ref(&self, index: &u16) -> ClassFileParsingResult<ClassReference> {
        let ConstantPoolEntry::Class { name_index } = self.get_entry(index)? else {
            return Err(ClassFileParsingError::MidmatchedConstantPoolTag);
        };
        let name = self.get_string(&name_index)?;
        Ok(ClassReference { name })
    }

    pub(crate) fn get_constant_value(
        &self,
        value_index: &u16,
    ) -> ClassFileParsingResult<ConstantValue> {
        let entry = self.get_entry(value_index)?;
        match entry {
            ConstantPoolEntry::Integer(it) => Ok(ConstantValue::Integer(*it)),
            ConstantPoolEntry::Long(it) => Ok(ConstantValue::Long(*it)),
            ConstantPoolEntry::Float(it) => Ok(ConstantValue::Float(*it)),
            ConstantPoolEntry::Double(it) => Ok(ConstantValue::Double(*it)),
            ConstantPoolEntry::String { string_index } => {
                self.get_string(string_index).map(ConstantValue::String)
            }
            _ => Err(ClassFileParsingError::MidmatchedConstantPoolTag),
        }
    }

    pub(crate) fn get_module_ref(&self, index: &u16) -> ClassFileParsingResult<ModuleReference> {
        let entry = self.get_entry(index)?;
        if let ConstantPoolEntry::Module { name_index } = entry {
            let name = self.get_string(&name_index)?;
            return Ok(ModuleReference { name });
        }
        Err(ClassFileParsingError::MidmatchedConstantPoolTag)
    }

    pub(crate) fn get_package_ref(&self, index: &u16) -> ClassFileParsingResult<PackageReference> {
        let entry = self.get_entry(index)?;
        if let ConstantPoolEntry::Package { name_index } = entry {
            let name = self.get_string(&name_index)?;
            return Ok(PackageReference { name });
        }
        Err(ClassFileParsingError::MidmatchedConstantPoolTag)
    }

    pub(crate) fn get_field_ref(&self, index: &u16) -> ClassFileParsingResult<FieldReference> {
        let entry = self.get_entry(index)?;
        if let ConstantPoolEntry::FieldRef {
            class_index,
            name_and_type_index,
        } = entry
        {
            let class = self.get_class_ref(class_index)?;
            if let ConstantPoolEntry::NameAndType {
                name_index,
                descriptor_index,
            } = self.get_entry(name_and_type_index)?
            {
                let name = self.get_string(&name_index)?;
                let descriptor = self.get_string(&descriptor_index)?;
                return Ok(FieldReference {
                    class,
                    name,
                    descriptor,
                });
            }
        }
        Err(ClassFileParsingError::MidmatchedConstantPoolTag)
    }

    fn get_name_and_type(&self, index: &u16) -> ClassFileParsingResult<(String, String)> {
        let entry = self.get_entry(index)?;
        if let ConstantPoolEntry::NameAndType {
            name_index,
            descriptor_index,
        } = entry
        {
            let name = self.get_string(&name_index)?;
            let descriptor = self.get_string(&descriptor_index)?;
            return Ok((name, descriptor));
        }
        Err(ClassFileParsingError::MidmatchedConstantPoolTag)?
    }

    pub(crate) fn get_method_ref(&self, index: &u16) -> ClassFileParsingResult<MethodReference> {
        let entry = self.get_entry(index)?;
        if let ConstantPoolEntry::MethodRef { class_index, name_and_type_index } = entry {
            let class = self.get_class_ref(class_index)?;
            let (name, descriptor) = self.get_name_and_type(name_and_type_index)?;
            return Ok(MethodReference::Class {
                class,
                name,
                descriptor,
            });
        } else if let ConstantPoolEntry::InterfaceMethodRef { class_index, name_and_type_index } = entry {
            let class = self.get_class_ref(class_index)?;
            let (name, descriptor) = self.get_name_and_type(name_and_type_index)?;
            return Ok(MethodReference::Interface {
                class,
                name,
                descriptor,
            });
        }
        Err(ClassFileParsingError::MidmatchedConstantPoolTag)
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
    fn parse_multiple<R>(reader: &mut R, count: u16) -> ClassFileParsingResult<HashMap<u16, Self>>
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

    fn parse<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_utf8<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let length = read_u16(reader)?;
        let bytes = read_bytes_vec(reader, length as usize)?;
        if let Ok(result) = String::from_utf8(bytes) {
            Ok(Self::Utf8(result))
        } else {
            Err(ClassFileParsingError::MalformedClassFile)
        }
    }

    fn parse_integer<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Integer(i32::from_be_bytes(bytes)))
    }

    fn parse_float<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Float(f32::from_be_bytes(bytes)))
    }

    fn parse_long<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Long(i64::from_be_bytes(bytes)))
    }

    fn parse_double<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bytes = read_bytes(reader)?;
        Ok(Self::Double(f64::from_be_bytes(bytes)))
    }

    fn parse_class<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Class { name_index })
    }

    fn parse_string<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let string_index = read_u16(reader)?;
        Ok(Self::String { string_index })
    }

    fn parse_field_ref<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_method_ref<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_interface_method_ref<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_name_and_type<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_method_handle<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_method_type<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let descriptor_index = read_u16(reader)?;
        Ok(Self::MethodType { descriptor_index })
    }

    fn parse_dynamic<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_invoke_dynamic<R>(reader: &mut R) -> ClassFileParsingResult<Self>
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

    fn parse_module<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Module { name_index })
    }

    fn parse_package<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Package { name_index })
    }
}
