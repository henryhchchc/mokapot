use super::{
    annotation::{Annotation, TypeAnnotation},
    class_parser::ClassFileParsingError,
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
    pub fn from_descriptor(descriptor: &char) -> Result<PrimitiveType, ClassFileParsingError> {
        match descriptor {
            'Z' => Ok(PrimitiveType::Boolean),
            'C' => Ok(PrimitiveType::Char),
            'F' => Ok(PrimitiveType::Float),
            'D' => Ok(PrimitiveType::Double),
            'B' => Ok(PrimitiveType::Byte),
            'S' => Ok(PrimitiveType::Short),
            'I' => Ok(PrimitiveType::Int),
            'J' => Ok(PrimitiveType::Long),
            _ => Err(ClassFileParsingError::InvalidDescriptor),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    Base(PrimitiveType),
    Object(String),
    Array(Box<FieldType>),
}

impl FieldType {
    pub fn make_array_type(&self) -> Self {
        Self::Array(Box::new(self.clone()))
    }

    pub fn from_descriptor(descriptor: &str) -> Result<Self, ClassFileParsingError> {
        let mut chars = descriptor.chars();
        match chars.next() {
            Some('L') => {
                if descriptor.ends_with(';') {
                    let type_name = chars.take_while(|it| *it != ';').collect::<String>();
                    Ok(FieldType::Object(type_name))
                } else {
                    Err(ClassFileParsingError::InvalidDescriptor)
                }
            }
            Some('[') => {
                FieldType::from_descriptor(chars.as_str()).map(|ft| FieldType::Array(Box::new(ft)))
            }
            Some(ref c) => PrimitiveType::from_descriptor(c).map(|pt| FieldType::Base(pt)),
            None => Err(ClassFileParsingError::InvalidDescriptor),
        }
    }
}

use bitflags::bitflags;

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
    use super::{FieldType, PrimitiveType};

    #[test]
    fn parse_primitive_types() {
        let descs = vec!['Z', 'C', 'F', 'D', 'B', 'S', 'I', 'J'];
        let mut types = descs
            .into_iter()
            .map(|ref d| PrimitiveType::from_descriptor(d))
            .collect::<Result<Vec<PrimitiveType>, _>>()
            .expect("Failed to parse primitive types")
            .into_iter();
        assert_eq!(types.next(), Some(PrimitiveType::Boolean));
        assert_eq!(types.next(), Some(PrimitiveType::Char));
        assert_eq!(types.next(), Some(PrimitiveType::Float));
        assert_eq!(types.next(), Some(PrimitiveType::Double));
        assert_eq!(types.next(), Some(PrimitiveType::Byte));
        assert_eq!(types.next(), Some(PrimitiveType::Short));
        assert_eq!(types.next(), Some(PrimitiveType::Int));
        assert_eq!(types.next(), Some(PrimitiveType::Long));
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
            .collect::<Result<Vec<FieldType>, _>>()
            .expect("Failed to parse field types")
            .into_iter();

        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Boolean)));
        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Char)));
        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Float)));
        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Double)));
        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Byte)));
        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Short)));
        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Int)));
        assert_eq!(types.next(), Some(FieldType::Base(PrimitiveType::Long)));
        assert_eq!(
            types.next(),
            Some(FieldType::Object("java/lang/String".to_string()))
        );
        assert_eq!(
            types.next(),
            Some(FieldType::Base(PrimitiveType::Int).make_array_type())
        );
        assert_eq!(
            types.next(),
            Some(
                FieldType::Object("java/lang/String".to_string())
                    .make_array_type()
                    .make_array_type()
            )
        );
    }
}
