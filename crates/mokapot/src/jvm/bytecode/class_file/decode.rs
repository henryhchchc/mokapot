use zerocopy::byteorder::big_endian::{U16, U32};

use super::{ClassFile, JAVA_CLASS_MAGIC};
use crate::jvm::bytecode::{
    FromBytecode, ParseError, attribute::AttributeInfo, constant_pool::RawConstantPool,
    decoder::read_value, field_info::FieldInfo, method_info::MethodInfo,
};

impl ClassFile {
    pub fn parse(mut bytes: &[u8]) -> Result<Self, ParseError> {
        let class = Self::from_bytes(&mut bytes)?;
        if !bytes.is_empty() {
            return Err(ParseError::malform(
                "Extra data at the end of the class file",
            ));
        }
        Ok(class)
    }
}

impl FromBytecode for ClassFile {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let magic: u32 = read_value::<U32>(bytes)?.get();
        if magic != JAVA_CLASS_MAGIC {
            return Err(ParseError::malform("This is not a Java class file"));
        }
        let minor_version = read_value::<U16>(bytes)?.get();
        let major_version = read_value::<U16>(bytes)?.get();
        let constant_pool_count = read_value::<U16>(bytes)?.get();
        let constant_pool = RawConstantPool::parse(bytes, constant_pool_count)?;
        let access_flags = read_value::<U16>(bytes)?.get();
        let this_class = read_value::<U16>(bytes)?.get();
        let super_class = read_value::<U16>(bytes)?.get();
        let interfaces_count: u16 = read_value::<U16>(bytes)?.get();
        let interfaces = (0..interfaces_count)
            .map(|_| read_value::<U16>(bytes).map(u16::from))
            .collect::<Result<_, ParseError>>()?;
        let fields_count: u16 = read_value::<U16>(bytes)?.get();
        let fields = (0..fields_count)
            .map(|_| FieldInfo::from_bytes(bytes))
            .collect::<Result<_, ParseError>>()?;
        let methods_count: u16 = read_value::<U16>(bytes)?.get();
        let methods = (0..methods_count)
            .map(|_| MethodInfo::from_bytes(bytes))
            .collect::<Result<_, ParseError>>()?;
        let attributes_count: u16 = read_value::<U16>(bytes)?.get();
        let attributes = (0..attributes_count)
            .map(|_| AttributeInfo::from_bytes(bytes))
            .collect::<Result<_, ParseError>>()?;

        Ok(Self {
            minor_version,
            major_version,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::{Class, bytecode::ParseErrorKind};

    #[test]
    fn rejects_invalid_magic() {
        let reader = &[0, 0, 0, 0][..];
        let error = ClassFile::parse(reader).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }

    fn object_bytes() -> Vec<u8> {
        let mut bytes = vec![
            0xca, 0xfe, 0xba, 0xbe, // magic
            0, 0, 0, 45, // version
            0, 3, // constant pool count
            1, 0, 16, // UTF-8 name
        ];
        bytes.extend(b"java/lang/Object");
        bytes.extend([
            7, 0, 1, // class name index
            0, 0x21, // access flags
            0, 2, // this class
            0, 0, // super class
            0, 0, // interfaces count
            0, 0, // fields count
            0, 0, // methods count
            0, 0, // attributes count
        ]);
        bytes
    }

    #[test]
    fn rejects_every_truncated_prefix() {
        let bytes = object_bytes();
        Class::from_bytes(&bytes).unwrap();
        for end in 0..bytes.len() {
            let error = Class::from_bytes(&bytes[..end]).unwrap_err();
            assert_eq!(error.kind(), ParseErrorKind::Malformed, "prefix {end}");
        }
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = object_bytes();
        bytes.push(0);
        let error = Class::from_bytes(&bytes).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }

    #[test]
    fn parses_unaligned_bytes_and_owns_the_result() {
        let class = {
            let mut bytes = vec![0xff];
            bytes.extend(object_bytes());
            Class::from_bytes(&bytes[1..]).unwrap()
        };
        assert_eq!(class.name, "java/lang/Object");
        assert!(class.super_class.is_none());
    }

    #[test]
    fn rejects_unavailable_attribute_payload() {
        let mut bytes = object_bytes();
        bytes.pop();
        bytes.push(1); // attributes count
        bytes.extend([0, 1, 0xff, 0xff, 0xff, 0xff]);
        let error = Class::from_bytes(&bytes).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }
}
