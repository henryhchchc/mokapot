use crate::utils::{read_u16, read_u32};

use super::{
    attributes::{Attribute, AttributeList},
    constant_pool::{ConstantPool},
    fields::{Field, FieldInfo},
};

#[derive(Debug)]
pub struct ClassFile {
    version: ClassFileVersion,
    constant_pool: ConstantPool,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<FieldInfo>,
    methods: Vec<MethodInfo>,
    attributes: AttributeList,
}

pub struct Class {
    pub version: ClassFileVersion,
    pub access_flags: u16,
    pub this_class: ClassReference,
    pub super_class: ClassReference,
    pub interfaces: Vec<ClassReference>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub source_file: Option<String>,
}

impl ClassFile {
    pub fn to_class(self) -> Result<Class, ClassFileParsingError> {
        let this_class = self.constant_pool.get_class_ref(self.this_class)?;
        let super_class = self.constant_pool.get_class_ref(self.super_class)?;
        let interfaces = self
            .interfaces
            .iter()
            .map(|i| self.constant_pool.get_class_ref(*i))
            .collect::<Result<Vec<_>, ClassFileParsingError>>()?;
        let fields = self
            .fields
            .iter()
            .map(|f| f.to_field(&self.constant_pool))
            .collect::<Result<Vec<Field>, ClassFileParsingError>>()?;
        let methods = self
            .methods
            .iter()
            .map(|m| m.to_method(&self.constant_pool))
            .collect::<Result<Vec<Method>, ClassFileParsingError>>()?;

        let mut source_file = None;
        for attr in self.attributes.into_iter() {
            match attr {
                Attribute::SourceFile(file_name) => source_file = Some(file_name.clone()),
                _ => {}
            }
        }
        Ok(Class {
            version: self.version,
            access_flags: self.access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            source_file,
        })
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

pub struct Method {
    pub access_flags: u16,
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug)]
pub struct MethodInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: AttributeList,
}

impl MethodInfo {
    pub(crate) fn to_method(
        &self,
        constant_pool: &ConstantPool,
    ) -> Result<Method, ClassFileParsingError> {
        let access_flags = self.access_flags;
        let name = constant_pool.get_string(self.name_index)?;
        let descriptor = constant_pool.get_string(self.descriptor_index)?;
        Ok(Method {
            access_flags,
            name,
            descriptor,
        })
    }

    fn parse_multiple<R>(
        reader: &mut R,
        methods_count: u16,
        constant_pool: &ConstantPool,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut methods = Vec::with_capacity(methods_count as usize);
        for _ in 0..methods_count {
            methods.push(Self::parse(reader, constant_pool)?);
        }
        Ok(methods)
    }

    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let access_flags = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let descriptor_index = read_u16(reader)?;
        let attributes = AttributeList::parse(reader, constant_pool)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
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
        let constant_pool = ConstantPool::parse(reader)?;
        let access_flags = read_u16(reader)?;
        let this_class = read_u16(reader)?;
        let super_class = read_u16(reader)?;
        let interfaces_count = read_u16(reader)?;
        let mut interfaces: Vec<u16> = Vec::with_capacity(interfaces_count as usize);
        for _ in 0..interfaces_count {
            interfaces.push(read_u16(reader)?);
        }
        let fields_count = read_u16(reader)?;
        let fields = FieldInfo::parse_multiple(reader, fields_count, &constant_pool)?;
        let methods_count = read_u16(reader)?;
        let methods = MethodInfo::parse_multiple(reader, methods_count, &constant_pool)?;
        let attributes = AttributeList::parse(reader, &constant_pool)?;
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

#[derive(Debug, PartialEq, Eq)]
pub struct ClassReference {
    pub name: String,
}

#[derive(Debug)]
pub enum ClassFileParsingError {
    MalformedClassFile,
    MidmatchedConstantPoolTag,
    BadConstantPoolIndex,
    UnknownAttributeName(String),
    InvalidAttributeLength { expected: u32, actual: u32 },
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
