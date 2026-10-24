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
//! assert_eq!(matrix.jls_name(), "double[][]");
//! ```
use std::str::FromStr;

use derive_more::{Display, From};

use super::{Descriptor, method_descriptor::InvalidDescriptor};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{JavaStr, JavaString},
    types::class_name::ClassName,
};

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
#[doc = see_jvm_spec!(2, 3)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Display)]
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

    /// Parses a primitive type from the beginning of `input`, advancing it past the parsed
    /// descriptor character on success.
    fn parse_prefix(input: &mut &[u8]) -> Result<Self, InvalidDescriptor> {
        let (&first_byte, rest) = input.split_first().ok_or(InvalidDescriptor)?;
        Self::try_from(char::from(first_byte)).inspect(|_| *input = rest)
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
        let mut bytes = descriptor.as_bytes();
        let pt = Self::parse_prefix(&mut bytes)?;
        if bytes.is_empty() {
            Ok(pt)
        } else {
            Err(InvalidDescriptor)
        }
    }
}

/// The category of a JVM value, which determines the number of slots it occupies.
///
/// Most values are of [`Category1`](Self::Category1), while `long` and `double` values are of [`Category2`](Self::Category2).
#[doc = see_jvm_spec!(2, 11, 1)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum ValueCategory {
    /// A value that occupies one slot.
    Category1,
    /// A value that occupies two slots.
    Category2,
}

impl ValueCategory {
    /// Returns the number of slots occupied by a value of this category.
    #[must_use]
    pub const fn slot_count(self) -> usize {
        match self {
            Self::Category1 => 1,
            Self::Category2 => 2,
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
///
/// // Create and work with different field types
/// let int_type = FieldType::Base(PrimitiveType::Int);
/// let string_type = FieldType::from_str("Ljava/lang/String;").unwrap();
/// let int_array = int_type.into_array_type(); // Creates int[]
/// let string_2d_array = FieldType::array_of(string_type, 2); // Creates String[][]
///
/// // Parse from JVM field descriptors
/// let int_type = FieldType::Base(PrimitiveType::Int);
/// assert_eq!(FieldType::from_str("I").unwrap(), int_type); // int
/// assert_eq!(FieldType::from_str("[[Ljava/lang/String;").unwrap(), string_2d_array); // String[][]
///
/// // Convert to descriptors or JLS-style names
/// assert_eq!(int_array.descriptor(), "[I");
/// let string_type = FieldType::from_str("Ljava/lang/String;").unwrap();
/// assert_eq!(string_type.jls_name(), "java.lang.String");
/// ```
///
#[doc = see_jvm_spec!(4, 3, 2)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display, From)]
pub enum FieldType {
    /// A primitive type.
    Base(#[from] PrimitiveType),
    /// A reference type (except arrays).
    Object(#[from] ClassName),
    /// An array of the given component type.
    #[display("{_0}[]")]
    Array(Box<FieldType>),
}

impl FieldType {
    /// Returns a JLS-style name, such as `int[]` or `java.util.Map$Entry`.
    #[must_use]
    pub fn jls_name(&self) -> JavaString {
        match self {
            Self::Base(pt) => JavaString::from(pt.to_string()),
            Self::Object(class_name) => class_name.jls_name(),
            Self::Array(inner) => {
                let mut name = inner.jls_name();
                name.push_str("[]");
                name
            }
        }
    }

    /// Parses a field type from the beginning of `input`, advancing it past the parsed type.
    pub(crate) fn parse_prefix(input: &mut &JavaStr) -> Result<Self, InvalidDescriptor> {
        let (tag, mut rest) = input.split_first_ascii().ok_or(InvalidDescriptor)?;
        let field_type = match tag {
            b'[' => Self::parse_prefix(&mut rest)?.into_array_type(),
            b'L' => {
                let (class_name, after_semi) =
                    rest.split_once_ascii(b';').ok_or(InvalidDescriptor)?;
                let class_name = ClassName::try_from(class_name).map_err(|_| InvalidDescriptor)?;
                rest = after_semi;
                class_name.into()
            }
            _ => Self::Base(PrimitiveType::try_from(char::from(tag))?),
        };
        *input = rest;
        Ok(field_type)
    }

    /// Creates an array type with the given type as its elements.
    #[must_use]
    pub fn into_array_type(self) -> Self {
        Self::Array(Box::new(self))
    }

    /// Creates an array type with the given type as its elements.
    ///
    /// ```
    /// use mokapot::types::{Descriptor, field_type::{FieldType, PrimitiveType}};
    ///
    /// let array = FieldType::array_of(PrimitiveType::Int.into(), 2);
    /// assert_eq!(array.descriptor(), "[[I");
    /// assert_eq!(array.jls_name(), "int[][]");
    /// ```
    #[must_use]
    pub fn array_of(inner: Self, dim: u8) -> Self {
        (0..dim).fold(inner, |acc, _| acc.into_array_type())
    }

    /// Returns the category of the values of this type.
    #[must_use]
    pub const fn value_category(&self) -> ValueCategory {
        match self {
            Self::Base(PrimitiveType::Long | PrimitiveType::Double) => ValueCategory::Category2,
            _ => ValueCategory::Category1,
        }
    }
}

impl Descriptor for FieldType {
    fn descriptor(&self) -> JavaString {
        let mut bytes = Vec::new();
        self.push_descriptor(&mut bytes);
        JavaString::from_cesu8(bytes).expect("type descriptors contain valid modified UTF-8")
    }
}

impl FieldType {
    pub(super) fn push_descriptor(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Base(it) => bytes.push(it.descriptor() as u8),
            Self::Object(class_name) => {
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

impl TryFrom<&JavaStr> for FieldType {
    type Error = InvalidDescriptor;

    fn try_from(descriptor: &JavaStr) -> Result<Self, Self::Error> {
        let mut rest = descriptor;
        let field_type = Self::parse_prefix(&mut rest)?;
        if rest.as_bytes().is_empty() {
            Ok(field_type)
        } else {
            Err(InvalidDescriptor)
        }
    }
}

impl FromStr for FieldType {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let desc_jstr = JavaStr::from_str(descriptor);
        Self::try_from(desc_jstr.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_type_mappings() {
        use PrimitiveType::*;
        use ValueCategory::*;

        for (ty, descriptor, name, tag, category) in [
            (Boolean, 'Z', "boolean", 4, Category1),
            (Char, 'C', "char", 5, Category1),
            (Float, 'F', "float", 6, Category1),
            (Double, 'D', "double", 7, Category2),
            (Byte, 'B', "byte", 8, Category1),
            (Short, 'S', "short", 9, Category1),
            (Int, 'I', "int", 10, Category1),
            (Long, 'J', "long", 11, Category2),
        ] {
            assert_eq!(ty.descriptor(), descriptor);
            assert_eq!(ty.to_string(), name);
            assert_eq!(ty.new_array_type_tag(), tag);
            assert_eq!(PrimitiveType::try_from(descriptor), Ok(ty));
            assert_eq!(descriptor.to_string().parse::<PrimitiveType>(), Ok(ty));
            let field_type = FieldType::Base(ty);
            assert_eq!(descriptor.to_string().parse(), Ok(field_type.clone()));
            assert_eq!(field_type.value_category(), category);
        }
        assert_eq!(Category1.slot_count(), 1);
        assert_eq!(Category2.slot_count(), 2);

        for invalid in ['A', 'V', '[', '\0', 'é'] {
            assert_eq!(PrimitiveType::try_from(invalid), Err(InvalidDescriptor));
            let descriptor = JavaString::from(invalid.to_string());
            let mut input = descriptor.as_java_str().as_bytes();
            assert_eq!(
                PrimitiveType::parse_prefix(&mut input),
                Err(InvalidDescriptor)
            );
            assert_eq!(input, descriptor.as_java_str().as_bytes());
        }

        let mut input = b"I!".as_slice();
        assert_eq!(PrimitiveType::parse_prefix(&mut input), Ok(Int));
        assert_eq!(input, b"!");
    }

    #[test]
    fn parses_prefix_at_array_depth_boundaries_and_rejects_invalid_elements() {
        for depth in [0_u8, 1, u8::MAX] {
            let descriptor = format!("{}I!", "[".repeat(usize::from(depth)));
            let encoded = JavaString::from(descriptor);
            let mut input = encoded.as_java_str();
            let parsed = FieldType::parse_prefix(&mut input).unwrap();
            assert_eq!(
                parsed,
                FieldType::array_of(PrimitiveType::Int.into(), depth)
            );
            assert_eq!(input, "!");
        }

        for descriptor in ["Lfoo", "[Lfoo", "Lfoo//bar;", "[Lfoo.bar;", "[[L;"] {
            let encoded = JavaString::from(descriptor.to_owned());
            let mut input = encoded.as_java_str();
            assert_eq!(FieldType::parse_prefix(&mut input), Err(InvalidDescriptor));
            assert_eq!(input, encoded.as_java_str());
        }
    }

    #[test]
    fn empty_primitive_and_field_descriptors_are_rejected() {
        let mut input = b"".as_slice();
        assert_eq!(
            PrimitiveType::parse_prefix(&mut input),
            Err(InvalidDescriptor)
        );
        assert_eq!(input, b"");
        assert_eq!("".parse::<PrimitiveType>(), Err(InvalidDescriptor));
        assert_eq!("".parse::<FieldType>(), Err(InvalidDescriptor));
    }

    #[test]
    fn parses_and_emits_modified_utf8_class_names() {
        let bytes = [b'L', b'p', b'/', 0xed, 0xa0, 0x80, b';'];
        let descriptor = JavaStr::from_cesu8(&bytes).unwrap();
        let field_type = FieldType::try_from(descriptor).unwrap();
        let FieldType::Object(class_name) = &field_type else {
            panic!("descriptor is an object type");
        };
        assert_eq!(
            class_name.as_java_str().as_bytes(),
            &[b'p', b'/', 0xed, 0xa0, 0x80]
        );
        assert_eq!(field_type.descriptor().as_java_str().as_bytes(), bytes);
    }
}
