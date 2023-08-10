use crate::{
    utils::{read_u16, read_u32, read_u8},
};

use super::class_file::ClassFileParsingError;

#[derive(Debug, Clone)]
pub enum ConstantPoolInfo {
    Utf8 {
        bytes: Vec<u8>,
    },
    Integer {
        bytes: u32,
    },
    Float {
        bytes: u32,
    },
    Long {
        high_bytes: u32,
        low_bytes: u32,
    },
    Double {
        high_bytes: u32,
        low_bytes: u32,
    },
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

impl ConstantPoolInfo {
    pub fn parse_multiple<R>(reader: &mut R, count: u16) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut result = Vec::with_capacity(count as usize);
        for _ in 0..count {
            result.push(Self::parse(reader)?);
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
        Ok(Self::Utf8 { bytes })
    }

    fn parse_integer<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_u32(reader)?;
        Ok(Self::Integer { bytes })
    }

    fn parse_float<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_u32(reader)?;
        Ok(Self::Float { bytes })
    }

    fn parse_long<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let high_bytes = read_u32(reader)?;
        let low_bytes = read_u32(reader)?;
        Ok(Self::Long {
            high_bytes,
            low_bytes,
        })
    }

    fn parse_double<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let high_bytes = read_u32(reader)?;
        let low_bytes = read_u32(reader)?;
        Ok(Self::Double {
            high_bytes,
            low_bytes,
        })
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
