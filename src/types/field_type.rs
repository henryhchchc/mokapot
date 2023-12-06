//! JVM non-generic type system
use std::{fmt::Display, str::FromStr};

use itertools::Itertools;

use crate::jvm::{class::ClassReference, method::InvalidDescriptor};

/// A primitive type in Java.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
    fn descriptor_str(&self) -> &'static str {
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

#[test]
fn primitive_type_from_str() {
    assert_eq!(
        PrimitiveType::from_str("Z").unwrap(),
        PrimitiveType::Boolean
    );
    assert_eq!(PrimitiveType::from_str("C").unwrap(), PrimitiveType::Char);
    assert_eq!(PrimitiveType::from_str("F").unwrap(), PrimitiveType::Float);
    assert_eq!(PrimitiveType::from_str("D").unwrap(), PrimitiveType::Double);
    assert_eq!(PrimitiveType::from_str("B").unwrap(), PrimitiveType::Byte);
    assert_eq!(PrimitiveType::from_str("S").unwrap(), PrimitiveType::Short);
    assert_eq!(PrimitiveType::from_str("I").unwrap(), PrimitiveType::Int);
    assert_eq!(PrimitiveType::from_str("J").unwrap(), PrimitiveType::Long);
    assert_eq!(
        PrimitiveType::from_str("Z").unwrap(),
        PrimitiveType::Boolean
    );
    assert!(PrimitiveType::from_str("X").is_err());
    assert!(PrimitiveType::from_str("FF").is_err());
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
            Self::Array(it) => write!(f, "{}[]", it),
        }
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
    pub fn make_array_type(&self) -> Self {
        Self::Array(Box::new(self.clone()))
    }

    pub(crate) fn descriptor_string(&self) -> String {
        match self {
            FieldType::Base(it) => it.descriptor_str().to_owned(),
            FieldType::Object(ClassReference { binary_name }) => {
                format!("L{};", binary_name)
            }
            FieldType::Array(inner) => format!("[{}", inner.descriptor_string()),
        }
    }
}

#[test]
fn field_type_from_str() {
    assert_eq!(
        FieldType::from_str("Z").unwrap(),
        FieldType::Base(PrimitiveType::Boolean)
    );
    assert_eq!(
        FieldType::from_str("C").unwrap(),
        FieldType::Base(PrimitiveType::Char)
    );
    assert_eq!(
        FieldType::from_str("F").unwrap(),
        FieldType::Base(PrimitiveType::Float)
    );
    assert_eq!(
        FieldType::from_str("D").unwrap(),
        FieldType::Base(PrimitiveType::Double)
    );
    assert_eq!(
        FieldType::from_str("B").unwrap(),
        FieldType::Base(PrimitiveType::Byte)
    );
    assert_eq!(
        FieldType::from_str("S").unwrap(),
        FieldType::Base(PrimitiveType::Short)
    );
    assert_eq!(
        FieldType::from_str("I").unwrap(),
        FieldType::Base(PrimitiveType::Int)
    );
    assert_eq!(
        FieldType::from_str("J").unwrap(),
        FieldType::Base(PrimitiveType::Long)
    );
    assert_eq!(
        FieldType::from_str("Z").unwrap(),
        FieldType::Base(PrimitiveType::Boolean)
    );
    assert_eq!(
        FieldType::from_str("Ljava/lang/Object;").unwrap(),
        FieldType::Object(ClassReference::new("java/lang/Object"))
    );
    assert_eq!(
        FieldType::from_str("[I").unwrap(),
        FieldType::Base(PrimitiveType::Int).make_array_type()
    );
    assert_eq!(
        FieldType::from_str("[Ljava/lang/Object;").unwrap(),
        FieldType::Object(ClassReference::new("java/lang/Object")).make_array_type()
    );
    assert!(FieldType::from_str("X").is_err());
    assert!(FieldType::from_str("FF").is_err());
}

/// A reference to a [`FieldType`].
#[derive(Debug, PartialEq, Clone)]
pub struct TypeReference(pub FieldType);
