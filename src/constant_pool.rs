use crate::class_file::{read_u16, read_u32, ClassFileParsingError};

pub enum ConstantPoolInfo {
    Padding,
    Utf8 {
        tag: u8,
        length: u16,
        bytes: Vec<u8>,
    },
    Integer {
        tag: u8,
        bytes: u32,
    },
    Float {
        tag: u8,
        bytes: u32,
    },
    Long {
        tag: u8,
        high_bytes: u32,
        low_bytes: u32,
    },
    Double {
        tag: u8,
        high_bytes: u32,
        low_bytes: u32,
    },
    Class {
        tag: u8,
        name_index: u16,
    },
    String {
        tag: u8,
        string_index: u16,
    },
    FieldRef {
        tag: u8,
        class_index: u16,
        name_and_type_index: u16,
    },
    MethodRef {
        tag: u8,
        class_index: u16,
        name_and_type_index: u16,
    },
    InterfaceMethodRef {
        tag: u8,
        class_index: u16,
        name_and_type_index: u16,
    },
    NameAndType {
        tag: u8,
        name_index: u16,
        descriptor_index: u16,
    },
    MethodHandle {
        tag: u8,
        reference_kind: u8,
        reference_index: u16,
    },
    MethodType {
        tag: u8,
        descriptor_index: u16,
    },
    Dynamic {
        tag: u8,
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
    InvokeDynamic {
        tag: u8,
        bootstrap_method_attr_index: u16,
        name_and_type_index: u16,
    },
    Module {
        tag: u8,
        name_index: u16,
    },
    Package {
        tag: u8,
        name_index: u16,
    },
}

fn read_u8<R>(reader: &mut R) -> Result<u8, ClassFileParsingError>
where
    R: std::io::Read,
{
    let mut buf: [u8; 1] = [0];
    if reader.read_exact(&mut buf).is_err() {
        return Err(ClassFileParsingError::MalformedClassFile);
    }
    Ok(buf[0])
}

impl ConstantPoolInfo {
    pub fn parse_multiple<R>(reader: &mut R, count: u16) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut result = Vec::with_capacity(count as usize);
        result.push(Self::Padding);
        for _ in 1..count - 1 {
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
        Ok(Self::Utf8 {
            tag: 1,
            length,
            bytes,
        })
    }

    fn parse_integer<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_u32(reader)?;
        Ok(Self::Integer { tag: 3, bytes })
    }

    fn parse_float<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bytes = read_u32(reader)?;
        Ok(Self::Float { tag: 4, bytes })
    }

    fn parse_long<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let high_bytes = read_u32(reader)?;
        let low_bytes = read_u32(reader)?;
        Ok(Self::Long {
            tag: 5,
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
            tag: 6,
            high_bytes,
            low_bytes,
        })
    }

    fn parse_class<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Class { tag: 7, name_index })
    }

    fn parse_string<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let string_index = read_u16(reader)?;
        Ok(Self::String {
            tag: 8,
            string_index,
        })
    }

    fn parse_field_ref<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let class_index = read_u16(reader)?;
        let name_and_type_index = read_u16(reader)?;
        Ok(Self::FieldRef {
            tag: 9,
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
            tag: 10,
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
            tag: 11,
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
            tag: 12,
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
            tag: 15,
            reference_kind,
            reference_index,
        })
    }

    fn parse_method_type<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let descriptor_index = read_u16(reader)?;
        Ok(Self::MethodType {
            tag: 16,
            descriptor_index,
        })
    }

    fn parse_dynamic<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let bootstrap_method_attr_index = read_u16(reader)?;
        let name_and_type_index = read_u16(reader)?;
        Ok(Self::Dynamic {
            tag: 17,
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
            tag: 18,
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    fn parse_module<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Module {
            tag: 19,
            name_index,
        })
    }

    fn parse_package<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let name_index = read_u16(reader)?;
        Ok(Self::Package {
            tag: 20,
            name_index,
        })
    }
}
