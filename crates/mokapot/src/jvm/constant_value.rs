//! JVM compile-time constant values.

use std::{cmp::Ordering, hash::Hash};

use derive_more::Display;

use super::{JavaString, class::MethodHandle};
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
    Dynamic(u16, String, FieldType),
}

impl PartialEq<Self> for ConstantValue {
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

impl PartialOrd for ConstantValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Null, Self::Null) => Some(Ordering::Equal),
            (Self::Integer(lhs), Self::Integer(rhs)) => lhs.partial_cmp(rhs),
            (Self::Long(lhs), Self::Long(rhs)) => lhs.partial_cmp(rhs),
            (Self::Float(lhs), Self::Float(rhs)) => match (lhs.is_nan(), rhs.is_nan()) {
                (true, true) => Some(Ordering::Equal),
                (false, false) => lhs.partial_cmp(rhs),
                _ => None,
            },
            (Self::Double(lhs), Self::Double(rhs)) => match (lhs.is_nan(), rhs.is_nan()) {
                (true, true) => Some(Ordering::Equal),
                (false, false) => lhs.partial_cmp(rhs),
                _ => None,
            },
            (Self::String(lhs), Self::String(rhs)) => lhs.partial_cmp(rhs),
            _ => None,
        }
    }
}

impl Eq for ConstantValue {}

impl Hash for ConstantValue {
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

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashSet, hash_map::DefaultHasher},
        hash::{Hash, Hasher},
    };

    use super::ConstantValue;

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
        .map(ConstantValue::Float);
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
        .map(ConstantValue::Double);

        for values in [floats, doubles] {
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
            assert_eq!(values.iter().collect::<HashSet<_>>().len(), 7);
        }
    }
}
