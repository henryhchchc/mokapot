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
            Self::Array(it) => write!(f, "{}[]", it),
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

/// A reference to a [`FieldType`].
#[derive(Debug, PartialEq, Clone)]
pub struct TypeReference(pub FieldType);
