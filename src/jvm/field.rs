use std::fmt::Display;

use crate::types::FieldType;

use super::{
    annotation::{Annotation, TypeAnnotation},
    class::{ClassReference, Handle},
    method::MethodDescriptor,
};

#[derive(Debug)]
pub struct Field {
    pub access_flags: FieldAccessFlags,
    pub name: String,
    pub owner: ClassReference,
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

impl Field {
    pub fn make_reference(&self) -> FieldReference {
        FieldReference {
            class: self.owner.clone(),
            name: self.name.clone(),
            field_type: self.field_type.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum JavaString {
    ValidUtf8(String),
    InvalidUtf8(Vec<u8>),
}

impl Display for JavaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaString::ValidUtf8(value) => write!(f, "String(\"{}\")", value),
            JavaString::InvalidUtf8(value) => write!(
                f,
                "String({}) // Invalid UTF-8",
                value.iter().map(|it| format!("0x{:02X}", it)).join(" ")
            ),
        }
    }
}

/// Denotes a compile-time constant value.
#[derive(Debug, PartialEq, Clone)]
pub enum ConstantValue {
    Null,
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    String(JavaString),
    Class(ClassReference),
    Handle(Handle),
    MethodType(MethodDescriptor),
    Dynamic(u16, String, FieldType),
}

impl Display for ConstantValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantValue::Null => write!(f, "null"),
            ConstantValue::Integer(value) => write!(f, "int({})", value),
            ConstantValue::Float(value) => write!(f, "float({})", value),
            ConstantValue::Long(value) => write!(f, "long({})", value),
            ConstantValue::Double(value) => write!(f, "double({})", value),
            ConstantValue::String(value) => value.fmt(f),
            ConstantValue::Class(value) => write!(f, "{}.class", value),
            ConstantValue::Handle(value) => write!(f, "{:?}", value),
            ConstantValue::MethodType(value) => write!(f, "{:?}", value),
            ConstantValue::Dynamic(bootstrap_method_attr_index, name, field_type) => {
                write!(
                    f,
                    "Dynamic({}, {}, {})",
                    bootstrap_method_attr_index,
                    name,
                    field_type.descriptor_string()
                )
            }
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

/// A reference to a field.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct FieldReference {
    /// A reference to the class that contains the field.
    pub class: ClassReference,
    /// The name of the field.
    pub name: String,

    /// The type of the field.
    pub field_type: FieldType,
}

impl Display for FieldReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.class, self.name)
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use crate::jvm::class::ClassReference;
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
