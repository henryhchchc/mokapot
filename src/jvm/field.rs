//! JVM fields and constant values.
use core::f32;
use std::fmt::Display;

use crate::{
    macros::see_jvm_spec,
    types::{
        field_type::FieldType, method_descriptor::MethodDescriptor, signitures::FieldSignature,
    },
};

use super::{
    annotation::{Annotation, TypeAnnotation},
    class::MethodHandle,
    references::{ClassRef, FieldRef},
};

/// A JVM field.
#[doc = see_jvm_spec!(4, 5)]
#[derive(Debug, Clone)]
pub struct Field {
    /// The access modifiers of the field.
    pub access_flags: AccessFlags,
    /// The name of the field.
    pub name: String,
    /// The class containing the field.
    pub owner: ClassRef,
    /// The type of the field.
    pub field_type: FieldType,
    /// The constant value of the field, if any.
    pub constant_value: Option<ConstantValue>,
    /// Indicates if the field is synthesized by the compiler.
    pub is_synthetic: bool,
    /// Indicates if the field is deprecated.
    pub is_deperecated: bool,
    /// The generic signature.
    pub signature: Option<FieldSignature>,
    /// The runtime visible annotations.
    pub runtime_visible_annotations: Vec<Annotation>,
    /// The runtime invisible annotations.
    pub runtime_invisible_annotations: Vec<Annotation>,
    /// The runtime visible type annotations.
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    /// The runtime invisible type annotations.
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    /// Unrecognized JVM attributes.
    pub free_attributes: Vec<(String, Vec<u8>)>,
}

impl Field {
    /// Creates a [`FieldRef`] referring to the field.
    #[must_use]
    pub fn as_ref(&self) -> FieldRef {
        FieldRef {
            owner: self.owner.clone(),
            name: self.name.clone(),
            field_type: self.field_type.clone(),
        }
    }
}

/// A string in the JVM bytecode.
#[derive(PartialEq, Eq, Debug, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum JavaString {
    /// A valid UTF-8 string.
    Utf8(String),
    /// An string that is not valid UTF-8.
    InvalidUtf8(Vec<u8>),
}

impl Display for JavaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JavaString::Utf8(value) => write!(f, "String(\"{value}\")"),
            JavaString::InvalidUtf8(value) => write!(
                f,
                "String({}) // Invalid UTF-8",
                value.iter().map(|it| format!("0x{it:02X}")).join(" ")
            ),
        }
    }
}

/// Denotes a compile-time constant value.
#[doc = see_jvm_spec!(4, 4)]
#[derive(Debug, Clone)]
pub enum ConstantValue {
    /// The `null` value.
    Null,
    /// A primitive integer value (i.e., `int`).
    Integer(i32),
    /// A primitive floating point value (i.e., `float`).
    Float(f32),
    /// A primitive long value (i.e., `long`).
    Long(i64),
    /// A primitive double value (i.e., `double`).
    Double(f64),
    /// A string literal.
    String(JavaString),
    /// A class literal.
    Class(ClassRef),
    /// A method handle.
    Handle(MethodHandle),
    /// A method type.
    MethodType(MethodDescriptor),
    /// A dynamic constant.
    Dynamic(u16, String, FieldType),
}

impl PartialEq<Self> for ConstantValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Integer(lhs), Self::Integer(rhs)) => lhs == rhs,
            (Self::Float(lhs), Self::Float(rhs)) if lhs.is_nan() && rhs.is_nan() => true,
            (Self::Float(lhs), Self::Float(rhs)) => lhs == rhs,
            (Self::Long(lhs), Self::Long(rhs)) => lhs == rhs,
            (Self::Double(lhs), Self::Double(rhs)) if lhs.is_nan() && rhs.is_nan() => true,
            (Self::Double(lhs), Self::Double(rhs)) => lhs == rhs,
            (Self::String(lhs), Self::String(rhs)) => lhs == rhs,
            (Self::Class(lhs), Self::Class(rhs)) => lhs == rhs,
            (Self::Handle(lhs), Self::Handle(rhs)) => lhs == rhs,
            (Self::MethodType(lhs), Self::MethodType(rhs)) => lhs == rhs,
            (Self::Dynamic(lhs0, lhs1, lhs2), Self::Dynamic(rhs0, rhs1, rhs2)) => {
                lhs0 == rhs0 && lhs1 == rhs1 && lhs2 == rhs2
            }
            _ => false,
        }
    }
}

impl Eq for ConstantValue {}

impl Display for ConstantValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantValue::Null => write!(f, "null"),
            ConstantValue::Integer(value) => write!(f, "int({value})"),
            ConstantValue::Float(value) => write!(f, "float({value})"),
            ConstantValue::Long(value) => write!(f, "long({value})"),
            ConstantValue::Double(value) => write!(f, "double({value})"),
            ConstantValue::String(value) => value.fmt(f),
            ConstantValue::Class(value) => write!(f, "{value}.class"),
            ConstantValue::Handle(value) => write!(f, "{value:?}"),
            ConstantValue::MethodType(value) => write!(f, "{value:?}"),
            ConstantValue::Dynamic(bootstrap_method_attr_index, name, field_type) => {
                write!(
                    f,
                    "Dynamic({}, {}, {})",
                    bootstrap_method_attr_index,
                    name,
                    field_type.descriptor()
                )
            }
        }
    }
}

use bitflags::bitflags;
use itertools::Itertools;

bitflags! {
    /// The access flags of a field.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u16 {
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

    use super::*;
    use crate::types::field_type::PrimitiveType;
    use PrimitiveType::{Boolean, Byte, Char, Double, Float, Int, Long, Short};

    use proptest::prelude::*;

    use super::AccessFlags;

    #[test]
    fn parse_primitive() {
        assert_eq!(PrimitiveType::try_from('Z'), Ok(Boolean));
        assert_eq!(PrimitiveType::try_from('C'), Ok(Char));
        assert_eq!(PrimitiveType::try_from('F'), Ok(Float));
        assert_eq!(PrimitiveType::try_from('D'), Ok(Double));
        assert_eq!(PrimitiveType::try_from('B'), Ok(Byte));
        assert_eq!(PrimitiveType::try_from('S'), Ok(Short));
        assert_eq!(PrimitiveType::try_from('I'), Ok(Int));
        assert_eq!(PrimitiveType::try_from('J'), Ok(Long));
    }

    #[test]
    fn prase_field_type() {
        assert_eq!("Z".parse(), Ok(FieldType::Base(Boolean)));
        assert_eq!("C".parse(), Ok(FieldType::Base(Char)));
        assert_eq!("F".parse(), Ok(FieldType::Base(Float)));
        assert_eq!("D".parse(), Ok(FieldType::Base(Double)));
        assert_eq!("B".parse(), Ok(FieldType::Base(Byte)));
        assert_eq!("S".parse(), Ok(FieldType::Base(Short)));
        assert_eq!("I".parse(), Ok(FieldType::Base(Int)));
        assert_eq!("J".parse(), Ok(FieldType::Base(Long)));
        assert_eq!(
            "Ljava/lang/String;".parse(),
            Ok(FieldType::Object(ClassRef::new("java/lang/String")))
        );
        assert_eq!("[I".parse(), Ok(FieldType::Base(Int).into_array_type()));
        assert_eq!(
            "[[Ljava/lang/String;".parse(),
            Ok(FieldType::Object(ClassRef::new("java/lang/String"))
                .into_array_type()
                .into_array_type())
        );
    }

    #[test]
    fn missing_semicolon() {
        assert!(FieldType::from_str("Ljava/lang/String").is_err());
    }

    #[test]
    fn tailing_chars() {
        assert!(FieldType::from_str("Ljava/lang/String;A").is_err());
    }

    #[test]
    fn misisng_array_element() {
        assert!(FieldType::from_str("[").is_err());
    }

    #[test]
    fn invalid_array_element() {
        assert!(FieldType::from_str("[A").is_err());
    }

    fn arb_access_flag() -> impl Strategy<Value = AccessFlags> {
        prop_oneof![
            Just(AccessFlags::PUBLIC),
            Just(AccessFlags::PRIVATE),
            Just(AccessFlags::PROTECTED),
            Just(AccessFlags::STATIC),
            Just(AccessFlags::FINAL),
            Just(AccessFlags::VOLATILE),
            Just(AccessFlags::TRANSIENT),
            Just(AccessFlags::SYNTHETIC),
            Just(AccessFlags::ENUM),
        ]
    }

    proptest! {

        #[test]
        fn access_flags_bit_no_overlap(
            lhs in arb_access_flag(),
            rhs in arb_access_flag()
        ){
            prop_assume!(lhs != rhs);
            assert_eq!(lhs.bits() & rhs.bits(), 0);
        }
    }
}
