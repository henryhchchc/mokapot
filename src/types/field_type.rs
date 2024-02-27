//! Non-generic JVM type system
use std::{fmt::Display, str::FromStr};

use itertools::Itertools;

use super::method_descriptor::InvalidDescriptor;
use crate::jvm::references::ClassRef;

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
    Object(ClassRef),
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
                .map(FieldType::into_array_type)
                .map_err(|_| InvalidDescriptor(descriptor.to_owned())),
            Some('L') => {
                let type_name = chars.take_while_ref(|&it| it != ';').collect::<String>();
                match (chars.next(), chars.next()) {
                    (Some(';'), None) => Ok(Self::Object(ClassRef::new(type_name))),
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
    pub fn into_array_type(self) -> Self {
        Self::Array(Box::new(self))
    }

    /// Creates an array type with the given type as its elements.
    #[must_use]
    pub fn array_of(inner: Self, dim: u8) -> Self {
        (0..dim).fold(inner, |acc, _| acc.into_array_type())
    }

    /// Returns the JVM descriptor for this type.
    #[must_use]
    pub fn descriptor(&self) -> String {
        match self {
            FieldType::Base(it) => it.descriptor().to_string(),
            FieldType::Object(ClassRef { binary_name }) => {
                format!("L{binary_name};")
            }
            FieldType::Array(inner) => format!("[{}", inner.descriptor()),
        }
    }
}

/// A reference to a [`FieldType`].
#[derive(Debug, PartialEq, Clone)]
pub struct TypeReference(pub FieldType);

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::tests::{arb_class_name, arb_non_array_field_type};

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
        fn field_type_from_str_class(class_name in arb_class_name()) {
            let s = format!("L{class_name};");
            let expected = FieldType::Object(ClassRef::new(class_name));
            assert_eq!(s.parse(), Ok(expected));
        }

        #[test]
        fn field_type_from_str_array(
            base_type in arb_non_array_field_type(),
            dimention in 1..=u8::MAX
        ) {
            let s = format!("{}{}", "[".repeat(usize::from(dimention)), base_type.descriptor());
            let mut parsed = s.parse().expect("Failed to parse field type");
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
}
