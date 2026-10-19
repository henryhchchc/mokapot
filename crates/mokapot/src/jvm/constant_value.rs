//! JVM compile-time constant values.

use std::{cmp::Ordering, hash::Hash};

use derive_more::Display;

use super::{
    JavaStr, JavaString,
    class::{MethodHandle, MethodHandleView},
};
use crate::{
    intrinsics::see_jvm_spec,
    types::{
        field_type::FieldType, method_descriptor::MethodDescriptor, reference_type::ReferenceType,
    },
};

/// Denotes a compile-time constant value.
///
#[doc = see_jvm_spec!(4, 4)]
#[derive(Debug, Clone, Display)]
pub enum ConstantValue {
    /// The `null` value.
    #[display("null")]
    Null,
    /// A primitive integer value (i.e., `int`).
    #[display("int({_0})")]
    Integer(i32),
    /// A primitive floating point value (i.e., `float`).
    #[display("float({_0})")]
    Float(f32),
    /// A primitive long value (i.e., `long`).
    #[display("long({_0})")]
    Long(i64),
    /// A primitive double value (i.e., `double`).
    #[display("double({_0})")]
    Double(f64),
    /// A string literal.
    #[display("{_0}")]
    String(JavaString),
    /// A class literal.
    #[display("{_0}.class")]
    Class(ReferenceType),
    /// A method handle.
    #[display("{_0:?}")]
    Handle(MethodHandle),
    /// A method type.
    #[display("{_0:?}")]
    MethodType(MethodDescriptor),
    /// A dynamic constant.
    // TODO: Extract the BSM from constant pool
    #[display("Dynamic({_0}, {_1}, {_2})")]
    Dynamic(u16, JavaString, FieldType),
}

/// A borrowed compile-time constant value.
#[doc = see_jvm_spec!(4, 4)]
#[derive(Debug, Clone, Copy, Display)]
pub enum ConstantValueView<'pool> {
    /// The `null` value.
    #[display("null")]
    Null,
    /// A primitive integer value.
    #[display("int({_0})")]
    Integer(i32),
    /// A primitive floating point value.
    #[display("float({_0})")]
    Float(f32),
    /// A primitive long value.
    #[display("long({_0})")]
    Long(i64),
    /// A primitive double value.
    #[display("double({_0})")]
    Double(f64),
    /// A string literal.
    #[display("{_0}")]
    String(&'pool JavaStr),
    /// A class literal.
    #[display("{_0}.class")]
    Class(&'pool ReferenceType),
    /// A method handle.
    #[display("{_0:?}")]
    Handle(MethodHandleView<'pool>),
    /// A method type.
    #[display("{_0:?}")]
    MethodType(&'pool MethodDescriptor),
    /// A dynamic constant.
    #[display("Dynamic({_0}, {_1}, {_2})")]
    Dynamic(u16, &'pool JavaStr, &'pool FieldType),
}

impl From<ConstantValueView<'_>> for ConstantValue {
    fn from(value: ConstantValueView<'_>) -> Self {
        match value {
            ConstantValueView::Null => Self::Null,
            ConstantValueView::Integer(value) => Self::Integer(value),
            ConstantValueView::Float(value) => Self::Float(value),
            ConstantValueView::Long(value) => Self::Long(value),
            ConstantValueView::Double(value) => Self::Double(value),
            ConstantValueView::String(value) => Self::String(value.to_owned()),
            ConstantValueView::Class(value) => Self::Class(value.clone()),
            ConstantValueView::Handle(value) => Self::Handle(value.into()),
            ConstantValueView::MethodType(value) => Self::MethodType(value.clone()),
            ConstantValueView::Dynamic(bootstrap, name, ty) => {
                Self::Dynamic(bootstrap, name.to_owned(), ty.clone())
            }
        }
    }
}

macro_rules! impl_value_semantics {
    ($ty:ty) => {
        impl PartialEq<Self> for $ty {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    (Self::Null, Self::Null) => true,
                    (Self::Integer(lhs), Self::Integer(rhs)) => lhs == rhs,
                    (Self::Float(lhs), Self::Float(rhs)) if lhs.is_nan() && rhs.is_nan() => true,
                    (Self::Float(lhs), Self::Float(rhs)) => lhs.to_bits() == rhs.to_bits(),
                    (Self::Long(lhs), Self::Long(rhs)) => lhs == rhs,
                    (Self::Double(lhs), Self::Double(rhs)) if lhs.is_nan() && rhs.is_nan() => true,
                    (Self::Double(lhs), Self::Double(rhs)) => lhs.to_bits() == rhs.to_bits(),
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

        impl PartialOrd for $ty {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                match (self, other) {
                    (Self::Null, Self::Null) => Some(Ordering::Equal),
                    (Self::Integer(lhs), Self::Integer(rhs)) => lhs.partial_cmp(rhs),
                    (Self::Long(lhs), Self::Long(rhs)) => lhs.partial_cmp(rhs),
                    (Self::Float(lhs), Self::Float(rhs)) => match (lhs.is_nan(), rhs.is_nan()) {
                        (true, true) => Some(Ordering::Equal),
                        (false, false) => Some(lhs.total_cmp(rhs)),
                        _ => None,
                    },
                    (Self::Double(lhs), Self::Double(rhs)) => match (lhs.is_nan(), rhs.is_nan()) {
                        (true, true) => Some(Ordering::Equal),
                        (false, false) => Some(lhs.total_cmp(rhs)),
                        _ => None,
                    },
                    (Self::String(lhs), Self::String(rhs)) => lhs.partial_cmp(rhs),
                    (Self::Class(lhs), Self::Class(rhs)) if lhs == rhs => Some(Ordering::Equal),
                    (Self::Handle(lhs), Self::Handle(rhs)) if lhs == rhs => Some(Ordering::Equal),
                    (Self::MethodType(lhs), Self::MethodType(rhs)) if lhs == rhs => {
                        Some(Ordering::Equal)
                    }
                    (Self::Dynamic(lhs0, lhs1, lhs2), Self::Dynamic(rhs0, rhs1, rhs2))
                        if lhs0 == rhs0 && lhs1 == rhs1 && lhs2 == rhs2 =>
                    {
                        Some(Ordering::Equal)
                    }
                    _ => None,
                }
            }
        }

        impl Eq for $ty {}

        impl Hash for $ty {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                core::mem::discriminant(self).hash(state);
                match self {
                    Self::Integer(v) => v.hash(state),
                    Self::Long(v) => v.hash(state),
                    Self::Float(v) if !v.is_nan() => {
                        v.to_bits().hash(state);
                    }
                    Self::Double(v) if !v.is_nan() => {
                        v.to_bits().hash(state);
                    }
                    Self::Null | Self::Float(_) | Self::Double(_) => {}
                    Self::String(v) => v.hash(state),
                    Self::Class(v) => v.hash(state),
                    Self::Handle(v) => v.hash(state),
                    Self::MethodType(v) => v.hash(state),
                    Self::Dynamic(v0, v1, v2) => {
                        (v0, v1, v2).hash(state);
                    }
                }
            }
        }
    };
}

impl_value_semantics!(ConstantValue);
impl_value_semantics!(ConstantValueView<'_>);

#[cfg(test)]
mod tests {
    use std::{
        cmp::Ordering,
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use super::{ConstantValue, ConstantValueView, FieldType, JavaString};

    #[test]
    fn floating_point_identity() {
        let floats = [
            f32::NEG_INFINITY,
            -1.0,
            -0.0,
            0.0,
            1.0,
            f32::INFINITY,
            f32::NAN,
            f32::from_bits(0x7f80_0001),
            f32::from_bits(0xffc0_0042),
        ]
        .map(ConstantValueView::Float);
        let doubles = [
            f64::NEG_INFINITY,
            -1.0,
            -0.0,
            0.0,
            1.0,
            f64::INFINITY,
            f64::NAN,
            f64::from_bits(0x7ff0_0000_0000_0001),
            f64::from_bits(0xfff8_0000_0000_0042),
        ]
        .map(ConstantValueView::Double);

        check_identity(&floats);
        check_identity(&doubles);
        check_identity(&floats.map(ConstantValue::from));
        check_identity(&doubles.map(ConstantValue::from));
    }

    #[test]
    fn partial_order_agrees_with_equality() {
        let name = JavaString::from("value");
        let ty: FieldType = "I".parse().unwrap();
        let class = "Ljava/lang/Object;".parse().unwrap();
        let descriptor = "()V".parse().unwrap();
        let views = [
            ConstantValueView::Float(-0.0),
            ConstantValueView::Float(0.0),
            ConstantValueView::Float(f32::NAN),
            ConstantValueView::Double(-0.0),
            ConstantValueView::Double(0.0),
            ConstantValueView::Double(f64::NAN),
            ConstantValueView::Class(&class),
            ConstantValueView::MethodType(&descriptor),
            ConstantValueView::Dynamic(1, name.as_java_str(), &ty),
            ConstantValueView::Dynamic(2, name.as_java_str(), &ty),
        ];

        check_partial_order(&views);
        check_partial_order(&views.map(ConstantValue::from));
        assert_eq!(views[0].partial_cmp(&views[1]), Some(Ordering::Less));
        assert_eq!(views[3].partial_cmp(&views[4]), Some(Ordering::Less));
        assert_eq!(views[0].partial_cmp(&views[2]), None);
        assert_eq!(views[0].partial_cmp(&views[3]), None);
        assert_eq!(views[8].partial_cmp(&views[9]), None);
    }

    fn check_partial_order(values: &[impl PartialOrd]) {
        for lhs in values {
            for rhs in values {
                assert_eq!(lhs == rhs, lhs.partial_cmp(rhs) == Some(Ordering::Equal));
                assert_eq!(
                    lhs.partial_cmp(rhs),
                    rhs.partial_cmp(lhs).map(Ordering::reverse)
                );
            }
        }
    }

    fn check_identity(values: &[impl Eq + Hash]) {
        for (i, lhs) in values.iter().enumerate() {
            for (j, rhs) in values.iter().enumerate() {
                assert_eq!(lhs == rhs, i.min(6) == j.min(6));
                if lhs == rhs {
                    let mut lhs_hash = DefaultHasher::new();
                    let mut rhs_hash = DefaultHasher::new();
                    lhs.hash(&mut lhs_hash);
                    rhs.hash(&mut rhs_hash);
                    assert_eq!(lhs_hash.finish(), rhs_hash.finish());
                }
            }
        }
    }
}
