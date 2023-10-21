use crate::types::FieldType;

use super::{
    annotation::{Annotation, TypeAnnotation},
    class::Handle,
    method::MethodDescriptor,
    references::ClassReference,
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
    Handle(Handle),
    MethodType(MethodDescriptor),
    Dynamic(u16, String, FieldType),
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

    use std::str::FromStr;

    use crate::elements::references::ClassReference;
    use crate::types::PrimitiveType::*;
    use crate::types::{FieldType, PrimitiveType};

    #[test]
    fn parse_primitive_types() {
        let descs = vec!['Z', 'C', 'F', 'D', 'B', 'S', 'I', 'J'];
        let mut types = descs
            .into_iter()
            .map(|d| PrimitiveType::try_from(d))
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
        assert!(PrimitiveType::try_from('A').is_err())
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
            .map(|it| FieldType::from_str(it))
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to parse field types")
            .into_iter();

        let string_type = FieldType::Object(ClassReference::new("java/lang/String"));

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
        assert!(FieldType::from_str(descriptor).is_err())
    }

    #[test]
    fn tailing_chars() {
        let descriptor = "Ljava/lang/String;A";
        assert!(FieldType::from_str(descriptor).is_err())
    }
}
