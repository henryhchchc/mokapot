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
        let attributes = parse_repeated(bytes, attributes_count, <&AttributeInfo>::decode)?;

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
    use crate::jvm::{Class, class::Version, errors::ParseErrorKind};

    fn assert_version_matches_spec(major: u16, minor: u16) {
        let valid = major == 45
            || ((46..=Version::LATEST.major()).contains(&major)
                && (minor == 0 || major >= 56 && minor == u16::MAX));
        let expected = if valid {
            Ok((major, minor, major >= 56 && minor == u16::MAX))
        } else {
            Err(ParseErrorKind::Malformed)
        };
        let actual = Version::from_versions(major, minor)
            .map(|version| {
                (
                    version.major(),
                    version.minor(),
                    version.is_preview_enabled(),
                )
            })
            .map_err(|error| error.kind());
        assert_eq!(actual, expected, "version {major}.{minor}");
    }

    #[test]
    fn class_version_boundaries_match_spec() {
        for major in [
            44,
            45,
            46,
            55,
            56,
            Version::LATEST.major(),
            Version::LATEST.major() + 1,
        ] {
            for minor in [0, 1, u16::MAX - 1, u16::MAX] {
                assert_version_matches_spec(major, minor);
            }
        }
    }

    #[test]
    fn class_bytes_reject_invalid_magic() {
        let error = Class::from_bytes(&[0, 0, 0, 0]).unwrap_err();
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
        let class = Class::from_bytes(&bytes).unwrap();
        assert_eq!(class.name().as_java_str(), "java/lang/Object");
        assert!(class.super_class().is_none());
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
    fn rejects_unavailable_attribute_payload() {
        let mut bytes = object_bytes();
        bytes.pop();
        bytes.push(1); // attributes count
        bytes.extend([0, 1, 0xff, 0xff, 0xff, 0xff]);
        let error = Class::from_bytes(&bytes).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }
}
