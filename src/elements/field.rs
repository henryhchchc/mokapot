use super::{
    annotation::{Annotation, TypeAnnotation},
    class_parser::ClassFileParsingError,
    references::{ClassReference, MethodReference}, class::MethodHandle, method::MethodDescriptor,
};

#[derive(Debug)]
pub struct Field {
    pub access_flags: FieldAccessFlags,
    pub name: String,
    pub field_type: FieldType,
    pub constant_value: Option<ConstantValue>,
    pub is_synthetic: bool,
    pub is_deperecated: bool,
    pub signature: Option<String>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

#[derive(Debug, PartialEq)]
pub enum ConstantValue {
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    String(String),
    Class(ClassReference),
    MethodHandle(MethodHandle),
    MethodType(MethodDescriptor),
}

#[derive(Debug, PartialEq, Clone)]
pub enum PrimitiveType {
    Boolean,
    Char,
    Float,
    Double,
    Byte,
    Short,
    Int,
    Long,
}

impl PrimitiveType {
    pub fn from_descriptor(descriptor: &char) -> Result<Self, ClassFileParsingError> {
        match descriptor {
            'Z' => Ok(Self::Boolean),
            'C' => Ok(Self::Char),
            'F' => Ok(Self::Float),
            'D' => Ok(Self::Double),
            'B' => Ok(Self::Byte),
            'S' => Ok(Self::Short),
            'I' => Ok(Self::Int),
            'J' => Ok(Self::Long),
            _ => Err(ClassFileParsingError::InvalidDescriptor),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ArrayType {
    Primitive {
        primitive: PrimitiveType,
        dimensions: u8,
    },
    Reference {
        class: ClassReference,
        dimensions: u8,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    Base(PrimitiveType),
    Object(ClassReference),
    Array(ArrayType),
}

impl FieldType {
    pub fn make_array_type(&self) -> Self {
        use ArrayType::*;
        use FieldType::*;
        match self {
            Base(p) => Array(Primitive {
                primitive: p.clone(),
                dimensions: 1,
            }),
            Object(c) => Array(Reference {
                class: c.clone(),
                dimensions: 1,
            }),
            Array(t) => Array(match t {
                Primitive {
                    primitive: primitive_type,
                    dimensions,
                } => Primitive {
                    primitive: primitive_type.clone(),
                    dimensions: dimensions + 1,
                },
                Reference {
                    class: class_reference,
                    dimensions,
                } => Reference {
                    class: class_reference.clone(),
                    dimensions: dimensions + 1,
                },
            }),
        }
    }

    pub fn from_descriptor(descriptor: &str) -> Result<Self, ClassFileParsingError> {
        let mut chars = descriptor.chars();
        let result = match chars.next() {
            Some('L') => {
                let type_name = chars.take_while_ref(|it| *it != ';').collect::<String>();
                match chars.next() {
                    Some(';') => Ok(FieldType::Object(ClassReference { name: type_name })),
                    _ => Err(ClassFileParsingError::InvalidDescriptor),
                }
            }
            Some('[') => {
                // Skip trailing character checking via `return`
                return FieldType::from_descriptor(chars.as_str()).map(|it| it.make_array_type());
            }
            Some(ref c) => PrimitiveType::from_descriptor(c).map(|it| FieldType::Base(it)),
            None => Err(ClassFileParsingError::InvalidDescriptor),
        }?;
        // Check if there is any trailing character
        if chars.next().is_none() {
            Ok(result)
        } else {
            Err(ClassFileParsingError::UnexpectedData)
        }
    }
}

use bitflags::bitflags;
use itertools::Itertools;

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct FieldAccessFlags: u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC = 0x0001;
        /// Declared `private`; accessible only within the defining class and other classes belonging to the same nest.
        const PRIVATE = 0x0002;
        /// Declared `protected`; may be accessed within subclasses.
        const PROTECTED = 0x0004;
        /// Declared `static`.
        const STATIC = 0x0008;
        /// Declared `final`; never directly assigned to after object construction.
        const FINAL = 0x0010;
        /// Declared `volatile`; cannot be cached.
        const VOLATILE = 0x0040;
        /// Declared `transient`; not written or read by a persistent object manager.
        const TRANSIENT = 0x0080;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
        /// Declared as an element of an `enum` class.
        const ENUM = 0x4000;
    }
}

#[cfg(test)]
mod test {
    use crate::elements::{field::PrimitiveType, references::ClassReference};

    use super::{FieldType, PrimitiveType::*};

    #[test]
    fn parse_primitive_types() {
        let descs = vec!['Z', 'C', 'F', 'D', 'B', 'S', 'I', 'J'];
        let mut types = descs
            .into_iter()
            .map(|ref d| PrimitiveType::from_descriptor(d))
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to parse primitive types")
            .into_iter();
        assert_eq!(types.next(), Some(Boolean));
        assert_eq!(types.next(), Some(Char));
        assert_eq!(types.next(), Some(Float));
        assert_eq!(types.next(), Some(Double));
        assert_eq!(types.next(), Some(Byte));
        assert_eq!(types.next(), Some(Short));
        assert_eq!(types.next(), Some(Int));
        assert_eq!(types.next(), Some(Long));
    }

    #[test]
    fn parse_invalid_primitive_type() {
        assert!(PrimitiveType::from_descriptor(&'A').is_err())
    }

    #[test]
    fn prase_field_type() {
        let descriptors = vec![
            "Z",
            "C",
            "F",
            "D",
            "B",
            "S",
            "I",
            "J",
            "Ljava/lang/String;",
            "[I",
            "[[Ljava/lang/String;",
        ];
        let mut types = descriptors
            .into_iter()
            .map(|ref it| FieldType::from_descriptor(it))
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to parse field types")
            .into_iter();

        let string_type = FieldType::Object(ClassReference {
            name: "java/lang/String".to_string(),
        });

        assert_eq!(types.next(), Some(FieldType::Base(Boolean)));
        assert_eq!(types.next(), Some(FieldType::Base(Char)));
        assert_eq!(types.next(), Some(FieldType::Base(Float)));
        assert_eq!(types.next(), Some(FieldType::Base(Double)));
        assert_eq!(types.next(), Some(FieldType::Base(Byte)));
        assert_eq!(types.next(), Some(FieldType::Base(Short)));
        assert_eq!(types.next(), Some(FieldType::Base(Int)));
        assert_eq!(types.next(), Some(FieldType::Base(Long)));
        assert_eq!(types.next(), Some(string_type.clone()));
        assert_eq!(types.next(), Some(FieldType::Base(Int).make_array_type()));
        assert_eq!(
            types.next(),
            Some(string_type.make_array_type().make_array_type())
        );
    }

    #[test]
    fn missing_semicolon() {
        let descriptor = "Ljava/lang/String";
        assert!(FieldType::from_descriptor(descriptor).is_err())
    }

    #[test]
    fn tailing_chars() {
        let descriptor = "Ljava/lang/String;A";
        assert!(FieldType::from_descriptor(descriptor).is_err())
    }
}
