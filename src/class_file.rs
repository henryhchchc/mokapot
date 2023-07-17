use crate::constant_pool::ConstantPoolInfo;

pub struct ClassFile {
    magic: u32,
    major_version: u16,
    minor_version: u16,
    constant_pool_count: u16,
    constant_pool: Vec<ConstantPoolInfo>,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces_count: u16,
    interfaces: Vec<u16>,
    fields_count: u16,
    fields: Vec<FieldInfo>,
    methods_count: u16,
    methods: Vec<MethodInfo>,
    attributes_count: u16,
    attributes: Vec<AttributeInfo>,
}

const JAVA_CLASS_MAIGC: u32 = 0xCAFEBABE;

/// Declared `public`; may be accessed from outside its package.
const ACC_PUBLIC: u16 = 0x0001;
/// Declared `final`; no subclasses allowed.
const ACC_FINAL: u16 = 0x0010;
/// Treat superclass methods specially when invoked by the invokespecial instruction.
const ACC_SUPER: u16 = 0x0020;
/// Is an interface, not a class.
const ACC_INTERFACE: u16 = 0x0200;
/// Declared `abstract`; must not be instantiated.
const ACC_ABSTRACT: u16 = 0x0400;
/// Declared synthetic; not present in the source code.
const ACC_SYNTHETIC: u16 = 0x1000;
/// Declared as an annotation interface.
const ACC_ANNOTATION: u16 = 0x2000;
/// Declared as an enum class.
const ACC_ENUM: u16 = 0x4000;
/// Is a module, not a class or interface.
const ACC_MODULE: u16 = 0x8000;

pub struct FieldInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes_count: u16,
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
            attributes_count,
            attributes,
        })
    }
}
pub struct MethodInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes_count: u16,
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
            attributes_count,
            attributes,
        })
    }
}
pub struct AttributeInfo {
    attribute_name_index: u16,
    attribute_length: u32,
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
            attribute_length,
            info,
        })
    }
}

impl ClassFile {
    pub fn parse<R>(reader: &mut R) -> Result<ClassFile, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        check_magic(reader)?;
        let major_version = read_u16(reader)?;
        let minor_version = read_u16(reader)?;
        let constant_pool_count = read_u16(reader)?;
        let constant_pool = ConstantPoolInfo::parse_multiple(reader, constant_pool_count)?;
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
            magic: JAVA_CLASS_MAIGC,
            major_version,
            minor_version,
            constant_pool_count,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces_count,
            interfaces,
            fields_count,
            fields,
            methods_count,
            methods,
            attributes_count,
            attributes,
        })
    }
}

fn check_magic<R>(reader: &mut R) -> Result<(), ClassFileParsingError>
where
    R: std::io::Read,
{
    let mut magic_buf = [0u8; 4];
    if reader.read_exact(&mut magic_buf).is_err() {
        return Err(ClassFileParsingError::NotAClassFile);
    }
    let magic = u32::from_be_bytes(magic_buf);
    if magic != JAVA_CLASS_MAIGC {
        return Err(ClassFileParsingError::NotAClassFile);
    }
    Ok(())
}

pub fn read_u32<R>(reader: &mut R) -> Result<u32, ClassFileParsingError>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 4];
    if reader.read_exact(&mut buf).is_err() {
        return Err(ClassFileParsingError::MalformedClassFile);
    }
    Ok(u32::from_be_bytes(buf))
}

pub fn read_u16<R>(reader: &mut R) -> Result<u16, ClassFileParsingError>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 2];
    if reader.read_exact(&mut buf).is_err() {
        return Err(ClassFileParsingError::MalformedClassFile);
    }
    Ok(u16::from_be_bytes(buf))
}

pub fn read_bytes<R>(reader: &mut R, len: usize) -> Result<Vec<u8>, ClassFileParsingError>
where
    R: std::io::Read,
{
    let mut buf = vec![0u8; len];
    if reader.read_exact(&mut buf).is_err() {
        return Err(ClassFileParsingError::MalformedClassFile);
    }
    Ok(buf)
}

#[derive(Debug)]
pub enum ClassFileParsingError {
    NotAClassFile,
    MalformedClassFile,
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
