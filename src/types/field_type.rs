//! Module for handling JVM field and method type descriptors.
//!
//! This module provides types and functionality for parsing and representing JVM type
//! descriptors according to the JVM specification. It supports primitive types, object
//! references, and array types.
//!
//! - [`PrimitiveType`] represents Java primitive types like `int`, `boolean`, etc.
//! - [`FieldType`] represents any valid field type including primitives, object references, and arrays
//! 
#![doc = see_jvm_spec!(4, 3, 2)]
//!
//! # Examples
//!
//! ```rust
//! use mokapot::types::Descriptor;
//! use mokapot::types::field_type::{FieldType, PrimitiveType};
//! use std::str::FromStr;
//!
//! // Parse a primitive type descriptor
//! let int_type = FieldType::from_str("I").unwrap();
//! assert!(matches!(int_type, FieldType::Base(PrimitiveType::Int)));
//!
//! // Parse an object type descriptor
//! let string_type = FieldType::from_str("Ljava/lang/String;").unwrap();
//! assert!(matches!(string_type, FieldType::Object(_)));
//!
//! // Parse an array type descriptor
//! let int_array = FieldType::from_str("[I").unwrap();
//! assert!(matches!(int_array, FieldType::Array(_)));
//!
//! // Create and format a multi-dimensional array type
//! let matrix = FieldType::array_of(FieldType::Base(PrimitiveType::Double), 2);
//! assert_eq!(matrix.descriptor(), "[[D");
//! assert_eq!(matrix.qualified_name(), "double[][]");
//! ```
use std::str::FromStr;

use super::{Descriptor, method_descriptor::InvalidDescriptor};
use crate::{jvm::references::ClassRef, macros::see_jvm_spec};

/// A primitive type in Java.
///
/// This enum represents the 8 primitive types in Java. Each variant corresponds to a primitive type
/// and can be converted to its JVM field descriptor or displayed as its Java type name.
///
/// # Examples
/// ```
/// use mokapot::types::field_type::PrimitiveType;
///
/// // Converting to descriptor
/// assert_eq!(PrimitiveType::Int.descriptor(), 'I');
/// assert_eq!(PrimitiveType::Boolean.descriptor(), 'Z');
///
/// // Converting from descriptor
/// assert_eq!(PrimitiveType::try_from('I'), Ok(PrimitiveType::Int));
///
/// // Getting type name string
/// assert_eq!(PrimitiveType::Long.to_string(), "long");
/// ```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum PrimitiveType {
    /// The `boolean` type (descriptor: 'Z')
    #[display("boolean")]
    Boolean,
    /// The `char` type (descriptor: 'C')
    #[display("char")]
    Char,
    /// The `float` type (descriptor: 'F')
    #[display("float")]
    Float,
    /// The `double` type (descriptor: 'D')
    #[display("double")]
    Double,
    /// The `byte` type (descriptor: 'B')
    #[display("byte")]
    Byte,
    /// The `short` type (descriptor: 'S')
    #[display("short")]
    Short,
    /// The `int` type (descriptor: 'I')
    #[display("int")]
    Int,
    /// The `long` type (descriptor: 'J')
    #[display("long")]
    Long,
}

impl PrimitiveType {
    /// Returns the JVM descriptor for this type.
    #[must_use]
    pub const fn descriptor(self) -> char {
        match self {
            Self::Boolean => 'Z',
            Self::Char => 'C',
            Self::Float => 'F',
            Self::Double => 'D',
            Self::Byte => 'B',
            Self::Short => 'S',
            Self::Int => 'I',
            Self::Long => 'J',
        }
    }

    /// Returns the type tag for the `newarray` instruction.
    #[must_use]
    pub const fn new_array_type_tag(self) -> u8 {
        match self {
            PrimitiveType::Boolean => 4,
            PrimitiveType::Char => 5,
            PrimitiveType::Float => 6,
            PrimitiveType::Double => 7,
            PrimitiveType::Byte => 8,
            PrimitiveType::Short => 9,
            PrimitiveType::Int => 10,
            PrimitiveType::Long => 11,
        }
    }
}

impl TryFrom<char> for PrimitiveType {
    type Error = InvalidDescriptor;

    fn try_from(descriptor: char) -> Result<Self, Self::Error> {
        match descriptor {
            'Z' => Ok(Self::Boolean),
            'C' => Ok(Self::Char),
            'F' => Ok(Self::Float),
            'D' => Ok(Self::Double),
            'B' => Ok(Self::Byte),
            'S' => Ok(Self::Short),
            'I' => Ok(Self::Int),
            'J' => Ok(Self::Long),
            _ => Err(InvalidDescriptor),
        }
    }
}

impl FromStr for PrimitiveType {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let mut chars = descriptor.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Self::try_from(c),
            _ => Err(InvalidDescriptor),
        }
    }
}

/// A field type (non-generic) in Java.
///
/// This enum represents any valid JVM field type, which can be:
/// - A primitive type like `int` or `boolean`
/// - An object reference type like `java.lang.String`
/// - An array type of any other valid field type
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use mokapot::types::Descriptor;
/// use mokapot::types::field_type::{FieldType, PrimitiveType};
/// use mokapot::jvm::references::ClassRef;
///
/// // Create and work with different field types
/// let int_type = FieldType::Base(PrimitiveType::Int);
/// let string_type = FieldType::Object(ClassRef::new("java/lang/String"));
/// let int_array = int_type.into_array_type(); // Creates int[]
/// let string_2d_array = FieldType::array_of(string_type, 2); // Creates String[][]
///
/// // Parse from JVM field descriptors
/// let int_type = FieldType::Base(PrimitiveType::Int);
/// assert_eq!(FieldType::from_str("I").unwrap(), int_type); // int
/// assert_eq!(FieldType::from_str("[[Ljava/lang/String;").unwrap(), string_2d_array); // String[][]
///
/// // Convert to descriptors or qualified names
/// let string_type = FieldType::Object(ClassRef::new("java/lang/String"));
/// assert_eq!(int_array.descriptor(), "[I");
/// assert_eq!(string_type.qualified_name(), "java.lang.String");
/// ```
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display)]
pub enum FieldType {
    /// A primitive type.
    Base(PrimitiveType),
    /// A reference type (except arrays).
    Object(ClassRef),
    /// An array type.
    #[display("{_0}[]")]
    Array(Box<FieldType>),
}

impl FieldType {
    /// Returns the qualified name of this type.
    #[must_use]
    pub fn qualified_name(&self) -> String {
        match self {
            Self::Base(pt) => pt.to_string(),
            Self::Object(ClassRef { binary_name }) => binary_name.replace('/', "."),
            Self::Array(inner) => format!("{}[]", inner.qualified_name()),
        }
    }
}

impl Descriptor for FieldType {
    fn descriptor(&self) -> String {
        match self {
            FieldType::Base(it) => it.descriptor().to_string(),
            FieldType::Object(ClassRef { binary_name }) => {
                format!("L{binary_name};")
            }
            FieldType::Array(inner) => format!("[{}", inner.descriptor()),
        }
    }
}

impl FromStr for FieldType {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        if descriptor.chars().count() == 1 {
            PrimitiveType::from_str(descriptor).map(Into::into)
        } else if descriptor.starts_with('[') {
            let element_type_desc = &descriptor['['.len_utf8()..];
            Self::from_str(element_type_desc).map(FieldType::into_array_type)
        } else if descriptor.starts_with('L') && descriptor.ends_with(';') {
            let binary_name = &descriptor['L'.len_utf8()..(descriptor.len() - ';'.len_utf8())];
            if binary_name.is_empty() || binary_name.contains(';') {
                Err(InvalidDescriptor)
            } else {
                let class_ref = ClassRef::new(binary_name);
                Ok(Self::Object(class_ref))
            }
        } else {
            Err(InvalidDescriptor)
        }
    }
}

impl From<PrimitiveType> for FieldType {
    fn from(it: PrimitiveType) -> Self {
        Self::Base(it)
    }
}

impl FieldType {
    /// Creates an array type with the given type as its elements.
    #[must_use]
    pub fn into_array_type(self) -> Self {
        Self::Array(Box::new(self))
    }

    /// Creates an array type with the given type as its elements.
    #[must_use]
    pub fn array_of(inner: Self, dim: u8) -> Self {
        (0..dim).fold(inner, |acc, _| acc.into_array_type())
    }
}

/// A reference to a [`FieldType`].
#[derive(Debug, PartialEq, Clone)]
#[deprecated = "Use `FieldType` directly"]
pub struct TypeReference(pub FieldType);

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::tests::{arb_identifier, arb_non_array_field_type};

    use super::*;

    #[test]
    fn primitive_type_descriptor() {
        use PrimitiveType::*;
        assert_eq!(Boolean.descriptor(), 'Z');
        assert_eq!(Char.descriptor(), 'C');
        assert_eq!(Float.descriptor(), 'F');
        assert_eq!(Double.descriptor(), 'D');
        assert_eq!(Byte.descriptor(), 'B');
        assert_eq!(Short.descriptor(), 'S');
        assert_eq!(Int.descriptor(), 'I');
        assert_eq!(Long.descriptor(), 'J');
    }

    #[test]
    fn primitive_type_display() {
        use PrimitiveType::*;
        assert_eq!(Boolean.to_string(), "boolean");
        assert_eq!(Char.to_string(), "char");
        assert_eq!(Float.to_string(), "float");
        assert_eq!(Double.to_string(), "double");
        assert_eq!(Byte.to_string(), "byte");
        assert_eq!(Short.to_string(), "short");
        assert_eq!(Int.to_string(), "int");
        assert_eq!(Long.to_string(), "long");
    }

    #[test]
    fn primitive_type_from_char() {
        use PrimitiveType::*;
        assert_eq!('Z'.try_into(), Ok(Boolean));
        assert_eq!('C'.try_into(), Ok(Char));
        assert_eq!('F'.try_into(), Ok(Float));
        assert_eq!('D'.try_into(), Ok(Double));
        assert_eq!('B'.try_into(), Ok(Byte));
        assert_eq!('S'.try_into(), Ok(Short));
        assert_eq!('I'.try_into(), Ok(Int));
        assert_eq!('J'.try_into(), Ok(Long));
    }

    proptest! {

        #[test]
        fn should_reject_invalid_primitive_type(s in r"[^ZCFDBSIJ].*") {
            assert!(PrimitiveType::from_str(&s).is_err());
        }

        #[test]
        fn should_reject_invalid_primitive_type_char(
            c in r"[^ZCFDBSIJ]".prop_map(|it| it.chars().next().unwrap())
        ) {
            assert!(PrimitiveType::try_from(c).is_err());
        }

    }

    #[test]
    fn field_type_display() {
        use FieldType::{Base, Object};
        use PrimitiveType::*;
        assert_eq!(Base(Boolean).to_string(), "boolean");
        assert_eq!(Base(Char).to_string(), "char");
        assert_eq!(Base(Float).to_string(), "float");
        assert_eq!(Base(Double).to_string(), "double");
        assert_eq!(Base(Byte).to_string(), "byte");
        assert_eq!(Base(Short).to_string(), "short");
        assert_eq!(Base(Int).to_string(), "int");
        assert_eq!(Base(Long).to_string(), "long");
        assert_eq!(
            Object(ClassRef::new("java/lang/Object")).to_string(),
            "java/lang/Object"
        );
        assert_eq!(Base(Int).into_array_type().to_string(), "int[]");
        assert_eq!(
            Object(ClassRef::new("java/lang/Object"))
                .into_array_type()
                .to_string(),
            "java/lang/Object[]"
        );
    }

    proptest! {
        #[test]
        fn field_type_from_str_class(class_name in arb_identifier()) {
            let s = format!("L{class_name};");
            let expected = FieldType::Object(ClassRef::new(class_name));
            assert_eq!(s.parse(), Ok(expected));
        }

        #[test]
        fn field_type_from_str_array(
            base_type in arb_non_array_field_type(),
            dimension in 1..=u8::MAX
        ) {
            let s = format!("{}{}", "[".repeat(usize::from(dimension)), base_type.descriptor());
            let mut parsed = s.parse().expect("Failed to parse field type");
            for _ in 0..dimension {
                if let FieldType::Array(element_type) = parsed {
                    // TODO: change to the following line
                    //       when `Box::into_inner` is stable
                    //       See https://github.com/rust-lang/rust/issues/80437
                    // parsed = Box::into_inner(element_type);
                    parsed = *element_type;
                } else {
                    panic!("Expected array type, got: {parsed:?}");
                }
            }
            assert_eq!(parsed, base_type);
        }
    }

    #[test]
    fn field_type_from_str_primitive() {
        use FieldType::Base;
        use PrimitiveType::*;
        assert_eq!("Z".parse(), Ok(Base(Boolean)));
        assert_eq!("C".parse(), Ok(Base(Char)));
        assert_eq!("F".parse(), Ok(Base(Float)));
        assert_eq!("D".parse(), Ok(Base(Double)));
        assert_eq!("B".parse(), Ok(Base(Byte)));
        assert_eq!("S".parse(), Ok(Base(Short)));
        assert_eq!("I".parse(), Ok(Base(Int)));
        assert_eq!("J".parse(), Ok(Base(Long)));
        assert_eq!("Z".parse(), Ok(Base(Boolean)));
    }

    #[test]
    fn parse_primitive() {
        assert_eq!(PrimitiveType::try_from('Z'), Ok(PrimitiveType::Boolean));
        assert_eq!(PrimitiveType::try_from('C'), Ok(PrimitiveType::Char));
        assert_eq!(PrimitiveType::try_from('F'), Ok(PrimitiveType::Float));
        assert_eq!(PrimitiveType::try_from('D'), Ok(PrimitiveType::Double));
        assert_eq!(PrimitiveType::try_from('B'), Ok(PrimitiveType::Byte));
        assert_eq!(PrimitiveType::try_from('S'), Ok(PrimitiveType::Short));
        assert_eq!(PrimitiveType::try_from('I'), Ok(PrimitiveType::Int));
        assert_eq!(PrimitiveType::try_from('J'), Ok(PrimitiveType::Long));
    }

    #[test]
    fn qualified_name() {
        assert_eq!(FieldType::Base(PrimitiveType::Int).qualified_name(), "int");
        assert_eq!(
            FieldType::from_str("Ljava/lang/String;")
                .unwrap()
                .qualified_name(),
            "java.lang.String"
        );
        assert_eq!(
            FieldType::from_str("[Ljava/lang/String;")
                .unwrap()
                .qualified_name(),
            "java.lang.String[]"
        );
    }

    #[test]
    fn missing_semicolon() {
        assert!(FieldType::from_str("Ljava/lang/String").is_err());
    }

    #[test]
    fn tailing_chars() {
        assert!(FieldType::from_str("Ljava/lang/String;A").is_err());
    }

    #[test]
    fn missing_array_element() {
        assert!(FieldType::from_str("[").is_err());
    }

    #[test]
    fn invalid_array_element() {
        assert!(FieldType::from_str("[A").is_err());
    }
}
