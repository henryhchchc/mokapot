//! Class, interface, and array types.
#![doc = see_jvm_spec!(4, 4, 1)]
//!
//! # Examples
//!
//! ```
//! use mokapot::types::reference_type::ReferenceType;
//!
//! let class: ReferenceType = "java/lang/String".parse().unwrap();
//! assert_eq!(class.jls_name(), "java.lang.String");
//!
//! let array: ReferenceType = "[I".parse().unwrap();
//! assert_eq!(array.jls_name(), "int[]");
//!
//! assert_eq!("Ljava/lang/String;".parse::<ReferenceType>().unwrap(), class);
//! ```

use std::str::FromStr;

use derive_more::{Display, From};

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{JavaStr, JavaString},
    types::class_name::ClassName,
    types::{Descriptor, field_type::FieldType, method_descriptor::InvalidDescriptor},
};

/// A class, interface, or array type.
#[doc = see_jvm_spec!(4, 4, 1)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From)]
pub enum ReferenceType {
    /// A class or interface type (e.g., `java/lang/String`).
    #[display("{_0}")]
    Class(#[from] ClassName),
    /// An array of the given component type.
    #[display("{_0}")]
    Array(Box<FieldType>),
}

impl ReferenceType {
    /// Returns a JLS-style name, such as `java.lang.String[]` or `java.util.Map$Entry`.
    #[must_use]
    pub fn jls_name(&self) -> JavaString {
        match self {
            Self::Class(class_name) => class_name.jls_name(),
            Self::Array(inner) => {
                let mut name = inner.jls_name();
                name.push_str("[]");
                name
            }
        }
    }
}

impl Descriptor for ReferenceType {
    fn descriptor(&self) -> JavaString {
        let mut bytes = Vec::new();
        self.push_descriptor(&mut bytes);
        JavaString::from_cesu8(bytes).expect("reference descriptors contain valid modified UTF-8")
    }
}

impl ReferenceType {
    fn push_descriptor(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Class(class_name) => {
                bytes.push(b'L');
                bytes.extend_from_slice(class_name.as_java_str().as_bytes());
                bytes.push(b';');
            }
            Self::Array(inner) => {
                bytes.push(b'[');
                inner.push_descriptor(bytes);
            }
        }
    }
}

impl From<ReferenceType> for FieldType {
    fn from(rt: ReferenceType) -> Self {
        match rt {
            ReferenceType::Class(class_name) => class_name.into(),
            ReferenceType::Array(ft) => FieldType::Array(ft),
        }
    }
}

impl FromStr for ReferenceType {
    type Err = InvalidDescriptor;

    /// Parses an internal class name or a class or array descriptor.
    ///
    /// Accepts `java/lang/String`, `Ljava/lang/String;`, and `[I`.
    #[doc = see_jvm_spec!(4, 4, 1)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = JavaStr::from_str(s);
        Self::try_from(name.as_ref())
    }
}

impl TryFrom<&JavaStr> for ReferenceType {
    type Error = InvalidDescriptor;

    /// Parses an internal class name or a class or array descriptor.
    ///
    /// Accepts `java/lang/String`, `Ljava/lang/String;`, and `[I`.
    #[doc = see_jvm_spec!(4, 4, 1)]
    fn try_from(s: &JavaStr) -> Result<Self, Self::Error> {
        let bytes = s.as_bytes();
        if bytes.starts_with(b"[") {
            let ft = FieldType::try_from(s)?;
            match ft {
                FieldType::Array(inner) => Ok(Self::Array(inner)),
                _ => Err(InvalidDescriptor),
            }
        } else if bytes.starts_with(b"L") && bytes.ends_with(b";") && bytes.len() > 2 {
            // Lenient: strip L and ; to recover the binary name.
            // Real-world class files (e.g. from javac) sometimes
            // emit field descriptors here despite JVMS §4.4.1.
            let inner =
                JavaStr::from_cesu8(&bytes[1..bytes.len() - 1]).map_err(|_| InvalidDescriptor)?;
            let class_name = ClassName::try_from(inner).map_err(|_| InvalidDescriptor)?;
            Ok(Self::Class(class_name))
        } else {
            let class_name = ClassName::try_from(s).map_err(|_| InvalidDescriptor)?;
            Ok(Self::Class(class_name))
        }
    }
}

impl TryFrom<FieldType> for ReferenceType {
    type Error = FieldType;

    fn try_from(ft: FieldType) -> Result<Self, Self::Error> {
        match ft {
            FieldType::Object(class_name) => Ok(Self::Class(class_name)),
            FieldType::Array(inner) => Ok(Self::Array(inner)),
            base @ FieldType::Base(_) => Err(base),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_class_and_array_reference_forms() {
        let class: ReferenceType = "java/lang/String".parse().unwrap();
        assert!(matches!(class, ReferenceType::Class(_)));
        assert_eq!(class.descriptor(), "Ljava/lang/String;");

        let descriptor_class: ReferenceType = "Ljava/lang/String;".parse().unwrap();
        assert_eq!(descriptor_class, class);

        let array: ReferenceType = "[I".parse().unwrap();
        assert!(matches!(array, ReferenceType::Array(_)));
        assert_eq!(array.jls_name(), "int[]");

        // A single letter is a class name here, even if it is a primitive descriptor.
        let single_letter: ReferenceType = "I".parse().unwrap();
        assert_eq!(
            single_letter,
            ReferenceType::Class(ClassName::new("I".to_owned()).unwrap())
        );
    }

    #[test]
    fn parses_modified_utf8_class_names_and_arrays() {
        for (bytes, expected) in [
            (
                &[b'p', b'/', 0xed, 0xa0, 0x80][..],
                &[b'L', b'p', b'/', 0xed, 0xa0, 0x80, b';'][..],
            ),
            (
                &[b'[', b'L', b'p', b'/', 0xed, 0xa0, 0x80, b';'][..],
                &[b'[', b'L', b'p', b'/', 0xed, 0xa0, 0x80, b';'][..],
            ),
        ] {
            let encoded = JavaStr::from_cesu8(bytes).unwrap();
            let reference = ReferenceType::try_from(encoded).unwrap();
            assert_eq!(reference.descriptor().as_java_str().as_bytes(), expected);
        }
    }
}
