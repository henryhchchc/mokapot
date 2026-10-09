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
use crate::{intrinsics::see_jvm_spec, types::class_name::ClassName};

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

    /// Parses a primitive type from the beginning of `input`, advancing it past the parsed
    /// descriptor character on success.
    fn parse_prefix(input: &mut &str) -> Result<Self, InvalidDescriptor> {
        let first_char = input.chars().next().ok_or(InvalidDescriptor)?;
        Self::try_from(first_char).inspect(|_| *input = &input[first_char.len_utf8()..])
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

    fn from_str(mut descriptor: &str) -> Result<Self, Self::Err> {
        let pt = Self::parse_prefix(&mut descriptor)?;
        if descriptor.is_empty() {
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
    pub fn jls_name(&self) -> String {
        match self {
            Self::Base(pt) => pt.to_string(),
            Self::Object(class_name) => class_name.jls_name(),
            Self::Array(inner) => format!("{}[]", inner.jls_name()),
        }
    }

    /// Parses a field type from the beginning of `input`, advancing it past the parsed type.
    pub(crate) fn parse_prefix(input: &mut &str) -> Result<Self, InvalidDescriptor> {
        if let Ok(pt) = PrimitiveType::parse_prefix(input) {
            Ok(Self::Base(pt))
        } else if let Some(mut rest) = input.strip_prefix('[') {
            Self::parse_prefix(&mut rest)
                .map(FieldType::into_array_type)
                .inspect(|_| *input = rest)
        } else if let Some(rest) = input.strip_prefix('L') {
            let (class_name, after_semi) = rest.split_once(';').ok_or(InvalidDescriptor)?;
            if class_name.is_empty() {
                Err(InvalidDescriptor)
            } else {
                let class_name: ClassName = class_name.parse().map_err(|_| InvalidDescriptor)?;
                *input = after_semi;
                Ok(class_name.into())
            }
        } else {
            Err(InvalidDescriptor)
        }
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
    fn descriptor(&self) -> String {
        match self {
            FieldType::Base(it) => it.descriptor().to_string(),
            FieldType::Object(class_name) => {
                format!("L{class_name};")
            }
            FieldType::Array(inner) => format!("[{}", inner.descriptor()),
        }
    }
}

impl FromStr for FieldType {
    type Err = InvalidDescriptor;

    fn from_str(mut descriptor: &str) -> Result<Self, Self::Err> {
        let ty = Self::parse_prefix(&mut descriptor)?;
        if descriptor.is_empty() {
            Ok(ty)
        } else {
            Err(InvalidDescriptor)
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::types::class_name::ClassName;

    impl Arbitrary for FieldType {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            let non_array = prop_oneof![
                any::<PrimitiveType>().prop_map(FieldType::Base),
                any::<ClassName>().prop_map(FieldType::from),
            ];
            let dimensions = prop_oneof![Just(0_u8), 1..=u8::MAX];
            (non_array, dimensions)
                .prop_map(|(inner, dim)| FieldType::array_of(inner, dim))
                .boxed()
        }
    }

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
    }

    proptest! {
        #[test]
        fn rejects_invalid_primitive_prefix(
            c in r"[^ZCFDBSIJ]",
            suffix in any::<String>(),
        ) {
            let ch = c.chars().next().unwrap();
            prop_assert_eq!(PrimitiveType::try_from(ch), Err(InvalidDescriptor));
            let descriptor = format!("{c}{suffix}");
            let mut input = descriptor.as_str();
            prop_assert_eq!(PrimitiveType::parse_prefix(&mut input), Err(InvalidDescriptor));
            prop_assert_eq!(input, descriptor.as_str());
            prop_assert_eq!(descriptor.parse::<PrimitiveType>(), Err(InvalidDescriptor));
        }

        #[test]
        fn primitive_prefix_preserves_suffix(
            ty in any::<PrimitiveType>(),
            suffix in any::<String>(),
        ) {
            let descriptor = format!("{}{suffix}", ty.descriptor());
            let mut input = descriptor.as_str();
            prop_assert_eq!(PrimitiveType::parse_prefix(&mut input), Ok(ty));
            prop_assert_eq!(input, suffix.as_str());
            if !suffix.is_empty() {
                prop_assert_eq!(descriptor.parse::<PrimitiveType>(), Err(InvalidDescriptor));
            }
        }

        #[test]
        fn field_names_descriptors_and_prefixes(
            base in prop_oneof![
                any::<PrimitiveType>().prop_map(FieldType::Base),
                any::<ClassName>().prop_map(FieldType::Object),
            ],
            dim in prop_oneof![Just(0_u8), Just(1_u8), 2..=u8::MAX],
            suffix in any::<String>(),
        ) {
            let (base_descriptor, base_name, base_jls_name) = match &base {
                FieldType::Base(ty) => (ty.descriptor().to_string(), ty.to_string(), ty.to_string()),
                FieldType::Object(name) => (
                    format!("L{name};"),
                    name.to_string(),
                    name.as_str().replace('/', "."),
                ),
                FieldType::Array(_) => unreachable!(),
            };
            let category = if dim == 0 && matches!(base, FieldType::Base(PrimitiveType::Long | PrimitiveType::Double)) {
                ValueCategory::Category2
            } else {
                ValueCategory::Category1
            };
            let field_type = FieldType::array_of(base, dim);
            prop_assert_eq!(field_type.value_category(), category);
            let descriptor = format!("{}{base_descriptor}", "[".repeat(usize::from(dim)));
            let brackets = "[]".repeat(usize::from(dim));
            prop_assert_eq!(field_type.to_string(), format!("{base_name}{brackets}"));
            prop_assert_eq!(field_type.jls_name(), format!("{base_jls_name}{brackets}"));
            prop_assert_eq!(field_type.descriptor(), descriptor.as_str());
            prop_assert_eq!(descriptor.parse::<FieldType>(), Ok(field_type.clone()));
            let with_suffix = format!("{descriptor}{suffix}");
            let mut input = with_suffix.as_str();
            prop_assert_eq!(FieldType::parse_prefix(&mut input), Ok(field_type));
            prop_assert_eq!(input, suffix.as_str());
            if !suffix.is_empty() {
                prop_assert_eq!(with_suffix.parse::<FieldType>(), Err(InvalidDescriptor));
            }
        }

        #[test]
        fn missing_object_semicolon_is_rejected_at_every_array_depth(
            class_name in any::<ClassName>(),
            dim in any::<u8>(),
        ) {
            let descriptor = format!("{}L{class_name}", "[".repeat(usize::from(dim)));
            let mut input = descriptor.as_str();
            prop_assert_eq!(FieldType::parse_prefix(&mut input), Err(InvalidDescriptor));
            prop_assert_eq!(input, descriptor.as_str());
            prop_assert_eq!(descriptor.parse::<FieldType>(), Err(InvalidDescriptor));
        }

        #[test]
        fn invalid_elements_are_rejected_at_every_array_depth(
            element in prop_oneof![
                Just(String::new()),
                "[^ZCFDBSIJL\\[]",
                Just("L;".to_owned()),
                Just("L/foo;".to_owned()),
                Just("Lfoo/;".to_owned()),
                Just("Lfoo//bar;".to_owned()),
                Just("Lfoo.bar;".to_owned()),
                Just("Lfoo[bar;".to_owned()),
            ],
            dim in any::<u8>(),
        ) {
            let descriptor = format!("{}{element}", "[".repeat(usize::from(dim)));
            let mut input = descriptor.as_str();
            prop_assert_eq!(FieldType::parse_prefix(&mut input), Err(InvalidDescriptor));
            prop_assert_eq!(input, descriptor.as_str());
            prop_assert_eq!(descriptor.parse::<FieldType>(), Err(InvalidDescriptor));
        }
    }

    #[test]
    fn rejects_empty_void_and_incomplete_field_descriptors() {
        for descriptor in ["", "V", "[", "[V", "[A", "L;"] {
            assert_eq!(descriptor.parse::<FieldType>(), Err(InvalidDescriptor));
        }
    }

    #[test]
    fn empty_primitive_descriptor_is_rejected() {
        let mut input = "";
        assert_eq!(
            PrimitiveType::parse_prefix(&mut input),
            Err(InvalidDescriptor)
        );
        assert_eq!(input, "");
        assert_eq!(input.parse::<PrimitiveType>(), Err(InvalidDescriptor));
    }
}
