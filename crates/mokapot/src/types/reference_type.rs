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
    pub fn jls_name(&self) -> String {
        match self {
            Self::Class(class_name) => class_name.jls_name(),
            Self::Array(inner) => format!("{}[]", inner.jls_name()),
        }
    }
}

impl Descriptor for ReferenceType {
    fn descriptor(&self) -> String {
        FieldType::from(self.clone()).descriptor()
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
        if s.starts_with('[') {
            let ft = FieldType::from_str(s)?;
            match ft {
                FieldType::Array(inner) => Ok(Self::Array(inner)),
                _ => Err(InvalidDescriptor),
            }
        } else if s.starts_with('L') && s.ends_with(';') && s.len() > 2 {
            // Lenient: strip L and ; to recover the binary name.
            // Real-world class files (e.g. from javac) sometimes
            // emit field descriptors here despite JVMS §4.4.1.
            let inner = &s[1..s.len() - 1];
            let class_name = ClassName::from_str(inner).map_err(|_| InvalidDescriptor)?;
            Ok(Self::Class(class_name))
        } else {
            let class_name = ClassName::from_str(s).map_err(|_| InvalidDescriptor)?;
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
    fn nested_class_rendering_preserves_binary_names() {
        for (input, expected) in [
            ("java/util/Map$Entry", "java.util.Map$Entry"),
            ("[Ljava/util/Map$Entry;", "java.util.Map$Entry[]"),
            ("[[Ljava/util/Map$Entry;", "java.util.Map$Entry[][]"),
        ] {
            let reference_type: ReferenceType = input.parse().unwrap();
            assert_eq!(reference_type.jls_name(), expected);
            let field_type = FieldType::from(reference_type);
            assert_eq!(field_type.jls_name(), expected);
        }
    }

    #[test]
    fn parse_class() {
        // Classes are parsed from binary names (internal form), not field descriptors
        let rt = "java/lang/String".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Class(_)));
        assert_eq!(rt.to_string(), "java/lang/String");
        assert_eq!(rt.descriptor(), "Ljava/lang/String;");
        assert_eq!(rt.jls_name(), "java.lang.String");
    }

    #[test]
    fn parse_array() {
        let rt = "[I".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Array(_)));
        assert_eq!(rt.descriptor(), "[I");
        assert_eq!(rt.jls_name(), "int[]");
    }

    #[test]
    fn parse_multidimensional() {
        let rt = "[[Ljava/lang/String;".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Array(_)));
        assert_eq!(rt.descriptor(), "[[Ljava/lang/String;");
    }

    #[test]
    fn accepts_field_descriptor_leniently() {
        // Field descriptors (L...;) are not strictly valid per JVMS §4.4.1,
        // but real-world class files (including from javac) emit them, so
        // we accept them leniently and parse them as class references.
        let rt = "Ljava/lang/String;".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Class(_)));
        assert_eq!(rt.to_string(), "java/lang/String");
        assert_eq!(rt.descriptor(), "Ljava/lang/String;");

        // Inner classes
        let rt = "Ljava/util/Map$Entry;".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Class(_)));
        assert_eq!(rt.to_string(), "java/util/Map$Entry");
    }

    #[test]
    fn single_letter_is_class_not_primitive() {
        // "I" in the constant pool is a binary name for a class named I,
        // not the primitive type int (which would be a Base FieldType).
        let rt = "I".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Class(_)));
        assert_eq!(rt.to_string(), "I");
    }
}
