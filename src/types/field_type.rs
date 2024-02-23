//! JVM non-generic type system
use std::{fmt::Display, str::FromStr};

use itertools::Itertools;

use crate::jvm::{class::ClassReference, method::InvalidDescriptor};

/// A primitive type in Java.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum PrimitiveType {
    /// The `boolean` type.
    Boolean,
    /// The `char` type.
    Char,
    /// The `float` type.
    Float,
    /// The `double` type.
    Double,
    /// The `byte` type.
    Byte,
    /// The `short` type.
    Short,
    /// The `int` type.
    Int,
    /// The `long` type.
    Long,
}

impl PrimitiveType {
    fn descriptor_str(self) -> &'static str {
        match self {
            Self::Boolean => "Z",
            Self::Char => "C",
            Self::Float => "F",
            Self::Double => "D",
            Self::Byte => "B",
            Self::Short => "S",
            Self::Int => "I",
            Self::Long => "J",
        }
    }
}

impl Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean => write!(f, "boolean"),
            Self::Char => write!(f, "char"),
            Self::Float => write!(f, "float"),
            Self::Double => write!(f, "double"),
            Self::Byte => write!(f, "byte"),
            Self::Short => write!(f, "short"),
            Self::Int => write!(f, "int"),
            Self::Long => write!(f, "long"),
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
            unexpected => Err(InvalidDescriptor(unexpected.to_string())),
        }
    }
}

impl FromStr for PrimitiveType {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let mut chars = descriptor.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Self::try_from(c),
            _ => Err(InvalidDescriptor(descriptor.to_owned())),
        }
    }
}

/// A field type (non-generic) in Java.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum FieldType {
    /// A primitive type.
    Base(PrimitiveType),
    /// A reference type (except arrays).
    Object(ClassReference),
    /// An array type.
    Array(Box<FieldType>),
}

impl Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(it) => it.fmt(f),
            Self::Object(it) => it.fmt(f),
            Self::Array(it) => write!(f, "{it}[]"),
        }
    }
}

impl FromStr for FieldType {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let mut chars = descriptor.chars();
        match chars.next() {
            Some('[') => Self::from_str(chars.as_str())
                .map(|it| it.make_array_type())
                .map_err(|_| InvalidDescriptor(descriptor.to_owned())),
            Some('L') => {
                let type_name = chars.take_while_ref(|it| it != &';').collect::<String>();
                match (chars.next(), chars.next()) {
                    (Some(';'), None) => Ok(Self::Object(ClassReference::new(type_name))),
                    _ => Err(InvalidDescriptor(descriptor.to_owned())),
                }
            }
            Some(c) => match chars.next() {
                None => PrimitiveType::try_from(c).map(Self::Base),
                _ => Err(InvalidDescriptor(descriptor.to_owned())),
            },
            None => Err(InvalidDescriptor(descriptor.to_owned())),
        }
    }
}

impl FieldType {
    /// Creates an array type with the given type as its elements.
    #[must_use]
    pub fn make_array_type(&self) -> Self {
        Self::Array(Box::new(self.clone()))
    }

    pub(crate) fn descriptor_string(&self) -> String {
        match self {
            FieldType::Base(it) => it.descriptor_str().to_owned(),
            FieldType::Object(ClassReference { binary_name }) => {
                format!("L{binary_name};")
            }
            FieldType::Array(inner) => format!("[{}", inner.descriptor_string()),
        }
    }
}

/// A reference to a [`FieldType`].
#[derive(Debug, PartialEq, Clone)]
pub struct TypeReference(pub FieldType);

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::tests::{arb_class_name, arb_primitive_type_name};

    use super::*;

    #[test]
    fn primitive_type_descriptor_str() {
        assert_eq!(PrimitiveType::Boolean.descriptor_str(), "Z");
        assert_eq!(PrimitiveType::Char.descriptor_str(), "C");
        assert_eq!(PrimitiveType::Float.descriptor_str(), "F");
        assert_eq!(PrimitiveType::Double.descriptor_str(), "D");
        assert_eq!(PrimitiveType::Byte.descriptor_str(), "B");
        assert_eq!(PrimitiveType::Short.descriptor_str(), "S");
        assert_eq!(PrimitiveType::Int.descriptor_str(), "I");
        assert_eq!(PrimitiveType::Long.descriptor_str(), "J");
    }

    #[test]
    fn primitive_type_display() {
        assert_eq!(PrimitiveType::Boolean.to_string(), "boolean");
        assert_eq!(PrimitiveType::Char.to_string(), "char");
        assert_eq!(PrimitiveType::Float.to_string(), "float");
        assert_eq!(PrimitiveType::Double.to_string(), "double");
        assert_eq!(PrimitiveType::Byte.to_string(), "byte");
        assert_eq!(PrimitiveType::Short.to_string(), "short");
        assert_eq!(PrimitiveType::Int.to_string(), "int");
        assert_eq!(PrimitiveType::Long.to_string(), "long");
    }

    #[test]
    fn primitive_type_from_str() {
        assert_eq!(PrimitiveType::from_str("Z"), Ok(PrimitiveType::Boolean));
        assert_eq!(PrimitiveType::from_str("C"), Ok(PrimitiveType::Char));
        assert_eq!(PrimitiveType::from_str("F"), Ok(PrimitiveType::Float));
        assert_eq!(PrimitiveType::from_str("D"), Ok(PrimitiveType::Double));
        assert_eq!(PrimitiveType::from_str("B"), Ok(PrimitiveType::Byte));
        assert_eq!(PrimitiveType::from_str("S"), Ok(PrimitiveType::Short));
        assert_eq!(PrimitiveType::from_str("I"), Ok(PrimitiveType::Int));
        assert_eq!(PrimitiveType::from_str("J"), Ok(PrimitiveType::Long));
    }
    #[test]
    fn primitive_type_from_char() {
        assert_eq!(PrimitiveType::try_from('Z'), Ok(PrimitiveType::Boolean));
        assert_eq!(PrimitiveType::try_from('C'), Ok(PrimitiveType::Char));
        assert_eq!(PrimitiveType::try_from('F'), Ok(PrimitiveType::Float));
        assert_eq!(PrimitiveType::try_from('D'), Ok(PrimitiveType::Double));
        assert_eq!(PrimitiveType::try_from('B'), Ok(PrimitiveType::Byte));
        assert_eq!(PrimitiveType::try_from('S'), Ok(PrimitiveType::Short));
        assert_eq!(PrimitiveType::try_from('I'), Ok(PrimitiveType::Int));
        assert_eq!(PrimitiveType::try_from('J'), Ok(PrimitiveType::Long));
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
        assert_eq!(
            FieldType::Base(PrimitiveType::Boolean).to_string(),
            "boolean"
        );
        assert_eq!(FieldType::Base(PrimitiveType::Char).to_string(), "char");
        assert_eq!(FieldType::Base(PrimitiveType::Float).to_string(), "float");
        assert_eq!(FieldType::Base(PrimitiveType::Double).to_string(), "double");
        assert_eq!(FieldType::Base(PrimitiveType::Byte).to_string(), "byte");
        assert_eq!(FieldType::Base(PrimitiveType::Short).to_string(), "short");
        assert_eq!(FieldType::Base(PrimitiveType::Int).to_string(), "int");
        assert_eq!(FieldType::Base(PrimitiveType::Long).to_string(), "long");
        assert_eq!(
            FieldType::Object(ClassReference::new("java/lang/Object")).to_string(),
            "java/lang/Object"
        );
        assert_eq!(
            FieldType::Base(PrimitiveType::Int)
                .make_array_type()
                .to_string(),
            "int[]"
        );
        assert_eq!(
            FieldType::Object(ClassReference::new("java/lang/Object"))
                .make_array_type()
                .to_string(),
            "java/lang/Object[]"
        );
    }

    proptest! {
        #[test]
        fn field_type_from_str_class(class_name in arb_class_name()) {
            let s = format!("L{class_name};");
            let expected = FieldType::Object(ClassReference::new(class_name));
            assert_eq!(FieldType::from_str(&s), Ok(expected));
        }

        #[test]
        fn field_type_from_str_array(
            base_type in prop_oneof![
                arb_primitive_type_name(),
                arb_class_name().prop_map(|it| format!("L{it};"))
            ],
            dimention in 1..=u8::MAX
        ) {
            let s = format!("{}{}", "[".repeat(usize::from(dimention)), base_type);
            let base_type = FieldType::from_str(&base_type).expect("Failed to parse base type");
            let mut parsed = FieldType::from_str(&s).expect("Failed to parse field type");
            for _ in 0..dimention {
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
        assert_eq!(
            FieldType::from_str("Z"),
            Ok(FieldType::Base(PrimitiveType::Boolean))
        );
        assert_eq!(
            FieldType::from_str("C"),
            Ok(FieldType::Base(PrimitiveType::Char))
        );
        assert_eq!(
            FieldType::from_str("F"),
            Ok(FieldType::Base(PrimitiveType::Float))
        );
        assert_eq!(
            FieldType::from_str("D"),
            Ok(FieldType::Base(PrimitiveType::Double))
        );
        assert_eq!(
            FieldType::from_str("B"),
            Ok(FieldType::Base(PrimitiveType::Byte))
        );
        assert_eq!(
            FieldType::from_str("S"),
            Ok(FieldType::Base(PrimitiveType::Short))
        );
        assert_eq!(
            FieldType::from_str("I"),
            Ok(FieldType::Base(PrimitiveType::Int))
        );
        assert_eq!(
            FieldType::from_str("J"),
            Ok(FieldType::Base(PrimitiveType::Long))
        );
        assert_eq!(
            FieldType::from_str("Z"),
            Ok(FieldType::Base(PrimitiveType::Boolean))
        );
    }
}
