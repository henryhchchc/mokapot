use super::{ClassFile, JAVA_CLASS_MAGIC};
use crate::jvm::class_file::{
    Decode, ParseError,
    attribute::AttributeInfo,
    constant_pool::RawConstantPool,
    decoder::{borrow_slice, decode_value, parse_repeated},
    field::FieldInfo,
    method::MethodInfo,
};

impl<'a> ClassFile<'a> {
    pub fn parse_complete(mut bytes: &'a [u8]) -> Result<Self, ParseError> {
        let class = Self::decode(&mut bytes)?;
        if !bytes.is_empty() {
            return Err(ParseError::malform(
                "Extra data at the end of the class file",
            ));
        }
        Ok(class)
    }
}

impl<'a> Decode<'a> for ClassFile<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let magic: u32 = decode_value(bytes)?;
        if magic != JAVA_CLASS_MAGIC {
            return Err(ParseError::malform("This is not a Java class file"));
        }
        let minor_version = decode_value(bytes)?;
        let major_version = decode_value(bytes)?;
        let constant_pool_count: u16 = decode_value(bytes)?;
        let constant_pool = RawConstantPool::parse_prefix(bytes, constant_pool_count)?;
        let access_flags = decode_value(bytes)?;
        let this_class = decode_value(bytes)?;
        let super_class = decode_value(bytes)?;
        let interfaces_count: u16 = decode_value(bytes)?;
        let interfaces = borrow_slice(bytes, usize::from(interfaces_count))?;
        let fields_count: u16 = decode_value(bytes)?;
        let fields = parse_repeated(bytes, fields_count, FieldInfo::decode)?;
        let methods_count: u16 = decode_value(bytes)?;
        let methods = parse_repeated(bytes, methods_count, MethodInfo::decode)?;
        let attributes_count: u16 = decode_value(bytes)?;
        let attributes = parse_repeated(bytes, attributes_count, AttributeInfo::decode)?;

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
    use crate::jvm::{Class, errors::ParseErrorKind};

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
