//! JVM reference type — a class, interface, or array type.
//!
//! A `ReferenceType` is the semantic model of what `CONSTANT_Class_info` represents
//! ([JVMS §4.4.1]). It is either a class/interface name in internal form or an
//! array type descriptor. It is a strict subset of [`FieldType`] — it excludes
//! primitive [`Base`](crate::types::field_type::FieldType::Base) types.
//!
//! # Examples
//!
//! ```
//! use std::str::FromStr;
//! use mokapot::types::reference_type::ReferenceType;
//! use mokapot::jvm::references::ClassRef;
//!
//! // Class types are parsed from binary names (internal form)
//! let string_type = ReferenceType::from_str("java/lang/String").unwrap();
//! assert!(matches!(string_type, ReferenceType::Class(_)));
//!
//! // Array types are parsed from field descriptors
//! let int_array = ReferenceType::from_str("[I").unwrap();
//! assert!(matches!(int_array, ReferenceType::Array(_)));
//!
//! // Field descriptors (L...;) are accepted leniently and parsed as classes
//! let class_type = ReferenceType::from_str("Ljava/lang/String;").unwrap();
//! assert!(matches!(class_type, ReferenceType::Class(_)));
//! assert_eq!(class_type.to_string(), "java/lang/String");
//! ```

use std::{fmt, str::FromStr};

use crate::{
    intrinsics::see_jvm_spec,
    jvm::references::ClassRef,
    types::{Descriptor, field_type::FieldType, method_descriptor::InvalidDescriptor},
};

/// A class, interface, or array type — exactly what `CONSTANT_Class_info` represents.
///
/// This is a strict subset of [`FieldType`]: it excludes primitive base types
/// since `CONSTANT_Class_info` never references those.
#[doc = see_jvm_spec!(4, 4, 1)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceType {
    /// A class or interface type (e.g., `java/lang/String`).
    Class(ClassRef),
    /// An array type (e.g., `[I`, `[Ljava/lang/Object;`).
    Array(Box<FieldType>),
}

impl ReferenceType {
    /// Returns the qualified name of this type (`.`-separated form for classes).
    #[must_use]
    pub fn qualified_name(&self) -> String {
        match self {
            Self::Class(class_ref) => class_ref.0.to_qualified_name(),
            Self::Array(inner) => format!("{}[]", inner.qualified_name()),
        }
    }
}

impl Descriptor for ReferenceType {
    fn descriptor(&self) -> String {
        FieldType::from(self.clone()).descriptor()
    }
}

impl fmt::Display for ReferenceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Class(class_ref) => write!(f, "{class_ref}"),
            Self::Array(inner) => write!(f, "{inner}"),
        }
    }
}

impl From<ClassRef> for ReferenceType {
    fn from(class_ref: ClassRef) -> Self {
        Self::Class(class_ref)
    }
}

impl From<ReferenceType> for FieldType {
    fn from(rt: ReferenceType) -> Self {
        match rt {
            ReferenceType::Class(cr) => FieldType::Object(cr),
            ReferenceType::Array(ft) => FieldType::Array(ft),
        }
    }
}

impl FromStr for ReferenceType {
    type Err = InvalidDescriptor;

    /// Parses a `ReferenceType` from its constant-pool string representation.
    ///
    /// A `CONSTANT_Class_info` stores either a binary name (for
    /// classes/interfaces) or an array descriptor (for array types).
    ///
    /// - Strings starting with `[` are parsed as array type descriptors.
    ///   The outer `Array` wrapper in the resulting `FieldType` is stripped
    ///   since `ReferenceType::Array` stores only the component type.
    /// - Strings starting with `L` and ending with `;` (field descriptor
    ///   form) are accepted leniently: the `L` and `;` are stripped and
    ///   the inner string is parsed as a binary name. Although the JVM
    ///   specification only permits binary names in `CONSTANT_Class_info`,
    ///   some class file generators (including `javac`) emit field descriptor
    ///   form, and the reference implementation accepts it.
    /// - All other strings are parsed as binary names (internal form) via
    ///   [`ClassRef::from_str`].
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
            let cr = ClassRef::from_str(inner).map_err(|_| InvalidDescriptor)?;
            Ok(Self::Class(cr))
        } else {
            let cr = ClassRef::from_str(s).map_err(|_| InvalidDescriptor)?;
            Ok(Self::Class(cr))
        }
    }
}

impl TryFrom<FieldType> for ReferenceType {
    type Error = FieldType;

    fn try_from(ft: FieldType) -> Result<Self, Self::Error> {
        match ft {
            FieldType::Object(cr) => Ok(Self::Class(cr)),
            FieldType::Array(inner) => Ok(Self::Array(inner)),
            base @ FieldType::Base(_) => Err(base),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_class() {
        // Classes are parsed from binary names (internal form), not field descriptors
        let rt = "java/lang/String".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Class(_)));
        assert_eq!(rt.to_string(), "java/lang/String");
        assert_eq!(rt.descriptor(), "Ljava/lang/String;");
        assert_eq!(rt.qualified_name(), "java.lang.String");
    }

    #[test]
    fn parse_array() {
        let rt = "[I".parse::<ReferenceType>().unwrap();
        assert!(matches!(rt, ReferenceType::Array(_)));
        assert_eq!(rt.descriptor(), "[I");
        assert_eq!(rt.qualified_name(), "int[]");
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
