use itertools::Itertools;

use crate::{elements::references::ClassReference, errors::InvalidDescriptor};

/// A primitive type in Java.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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
    pub fn new(descriptor: &char) -> Result<Self, InvalidDescriptor> {
        match descriptor {
            'Z' => Ok(Self::Boolean),
            'C' => Ok(Self::Char),
            'F' => Ok(Self::Float),
            'D' => Ok(Self::Double),
            'B' => Ok(Self::Byte),
            'S' => Ok(Self::Short),
            'I' => Ok(Self::Int),
            'J' => Ok(Self::Long),
            _ => Err(InvalidDescriptor(descriptor.to_string())),
        }
    }

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

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum FieldType {
    Base(PrimitiveType),
    Object(ClassReference),
    Array(Box<FieldType>),
}

impl FieldType {
    pub fn make_array_type(&self) -> Self {
        Self::Array(Box::new(self.clone()))
    }

    pub fn new(descriptor: &str) -> Result<Self, InvalidDescriptor> {
        let mut chars = descriptor.chars();
        match chars.next() {
            Some('[') => Self::new(chars.as_str())
                .map(|it| it.make_array_type())
                .map_err(|_| InvalidDescriptor(descriptor.to_owned())),
            Some('L') => {
                let type_name = chars.take_while_ref(|it| it != &';').collect::<String>();
                match (chars.next(), chars.next()) {
                    (Some(';'), None) => Ok(Self::Object(ClassReference::new(type_name))),
                    _ => Err(InvalidDescriptor(descriptor.to_owned())),
                }
            }
            Some(ref c) => match chars.next() {
                None => PrimitiveType::new(c).map(|it| Self::Base(it)),
                _ => Err(InvalidDescriptor(descriptor.to_owned())),
            },
            None => Err(InvalidDescriptor(descriptor.to_owned())),
        }
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
