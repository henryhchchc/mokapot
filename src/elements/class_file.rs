use crate::utils::{read_bytes, read_u16, read_u32};

use super::constant_pool::ConstantPoolInfo;

#[derive(Debug)]
pub struct ClassFile {
    version: ClassFileVersion,
    constant_pool: Vec<ConstantPoolInfo>,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<FieldInfo>,
    methods: Vec<MethodInfo>,
    attributes: Vec<AttributeInfo>,
}

pub struct Class {
    pub version: ClassFileVersion,
    pub access_flags: u16,
    pub binary_name: String,
    pub super_class_binary_name: String,
    pub interface_binary_names: Vec<String>,
}

impl ClassFile {
    pub fn to_class(&self) -> Result<Class, ClassFileParsingError> {
        let binary_name = self.get_constant_pool_string(&self.this_class)?;
        let super_class_binary_name = self.get_constant_pool_string(&self.super_class)?;
        let interfaces = self.interfaces.iter().map(|i| self.get_constant_pool_string(i)).collect::<Result<Vec<String>, ClassFileParsingError>>()?;
        Ok(Class {
            version: self.version,
            access_flags: self.access_flags,
            binary_name,
            super_class_binary_name,
            interface_binary_names: interfaces,
        })
    }

    fn get_constant_pool_string(
        &self,
        class_info_idx: &u16,
    ) -> Result<String, ClassFileParsingError> {
        if let Some(ConstantPoolInfo::Class { name_index }) =
            &self.constant_pool_entry(class_info_idx)
        {
            if let Some(ConstantPoolInfo::Utf8 { bytes }) = &self.constant_pool_entry(name_index) {
                return String::from_utf8(bytes.clone())
                    .map_err(|_| ClassFileParsingError::MalformedClassFile);
            }
        }
        Err(ClassFileParsingError::MidmatchedConstantPoolTag)
    }

    fn constant_pool_entry(&self, index: &u16) -> Option<&ConstantPoolInfo> {
        self.constant_pool.get((index - 1) as usize)
    }
}

#[derive(Debug, Copy, Clone)]
/// The version of a class file.
pub struct ClassFileVersion {
    /// The major version number.
    pub major: u16,
    /// the minor version number.
    pub minor: u16,
}

impl ClassFileVersion {
    /// Returns `true` if this class file is compiled with `--enable-preview`.
    pub fn is_preview_enabled(&self) -> bool {
        self.minor == 65535
    }
    fn parse<R>(reader: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let minor = read_u16(reader)?;
        let major = read_u16(reader)?;
        Ok(Self { major, minor })
    }
}

#[derive(Debug)]
pub struct FieldInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<AttributeInfo>,
}
impl FieldInfo {
    fn parse_multiple<R>(
        reader: &mut R,
        fields_count: u16,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut fields = Vec::with_capacity(fields_count as usize);
        for _ in 0..fields_count {
            fields.push(Self::parse(reader)?);
        }
        Ok(fields)
    }

    fn parse<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let access_flags = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let descriptor_index = read_u16(reader)?;
        let attributes_count = read_u16(reader)?;
        let attributes = AttributeInfo::parse_multiple(reader, attributes_count)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

#[derive(Debug)]
pub struct MethodInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<AttributeInfo>,
}
impl MethodInfo {
    fn parse_multiple<R>(
        reader: &mut R,
        methods_count: u16,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut methods = Vec::with_capacity(methods_count as usize);
        for _ in 0..methods_count {
            methods.push(Self::parse(reader)?);
        }
        Ok(methods)
    }

    fn parse<R>(reader: &mut R) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let access_flags = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let descriptor_index = read_u16(reader)?;
        let attributes_count = read_u16(reader)?;
        let attributes = AttributeInfo::parse_multiple(reader, attributes_count)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}
#[derive(Debug)]
pub struct AttributeInfo {
    attribute_name_index: u16,
    info: Vec<u8>,
}
impl AttributeInfo {
    fn parse_multiple<R>(
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
        let info = read_bytes(reader, attribute_length as usize)?;
        Ok(Self {
            attribute_name_index,
            info,
        })
    }
}

const JAVA_CLASS_MAIGC: u32 = 0xCAFEBABE;

impl ClassFile {
    pub fn parse<R>(reader: &mut R) -> Result<ClassFile, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let magic = read_u32(reader)?;
        if magic != JAVA_CLASS_MAIGC {
            return Err(ClassFileParsingError::MalformedClassFile);
        }
        let version = ClassFileVersion::parse(reader)?;
        let constant_pool_count = read_u16(reader)?;
        // Constant pool is indexed from 1, so we need to subtract 1
        let constant_pool = ConstantPoolInfo::parse_multiple(reader, constant_pool_count - 1)?;
        let access_flags = read_u16(reader)?;
        let this_class = read_u16(reader)?;
        let super_class = read_u16(reader)?;
        let interfaces_count = read_u16(reader)?;
        let mut interfaces: Vec<u16> = Vec::with_capacity(interfaces_count as usize);
        for _ in 0..interfaces_count {
            interfaces.push(read_u16(reader)?);
        }
        let fields_count = read_u16(reader)?;
        let fields = FieldInfo::parse_multiple(reader, fields_count)?;
        let methods_count = read_u16(reader)?;
        let methods = MethodInfo::parse_multiple(reader, methods_count)?;
        let attributes_count = read_u16(reader)?;
        let attributes = AttributeInfo::parse_multiple(reader, attributes_count)?;
        Ok(ClassFile {
            version,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        })
    }
}

#[derive(Debug)]
pub enum ClassFileParsingError {
    MalformedClassFile,
    MidmatchedConstantPoolTag,
}

impl From<std::io::Error> for ClassFileParsingError {
    fn from(_value: std::io::Error) -> Self {
        ClassFileParsingError::MalformedClassFile
    }
}

impl std::fmt::Display for ClassFileParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oops")
    }
}

impl std::error::Error for ClassFileParsingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}
