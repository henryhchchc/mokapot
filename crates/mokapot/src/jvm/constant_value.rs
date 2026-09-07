//! JVM compile-time constant values.

use std::{cmp::Ordering, hash::Hash};

use crate::{
    intrinsics::see_jvm_spec,
    types::{
        field_type::FieldType, method_descriptor::MethodDescriptor, reference_type::ReferenceType,
    },
};

use super::{JavaString, class::MethodHandle};

/// Denotes a compile-time constant value.
///
/// Values sort by variant declaration order, then by payload. Within each floating-point
/// variant, all NaNs are equal and sort last, while negative zero precedes positive zero.
#[doc = see_jvm_spec!(4, 4)]
#[derive(Debug, Clone, derive_more::Display)]
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

impl ConstantValue {
    const fn rank(&self) -> u8 {
        match self {
            Self::Null => 0,
            Self::Integer(_) => 1,
            Self::Float(_) => 2,
            Self::Long(_) => 3,
            Self::Double(_) => 4,
            Self::String(_) => 5,
            Self::Class(_) => 6,
            Self::Handle(_) => 7,
            Self::MethodType(_) => 8,
            Self::Dynamic(..) => 9,
        }
    }
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

impl Eq for ConstantValue {}

impl PartialOrd for ConstantValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConstantValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Null, Self::Null) => Ordering::Equal,
            (Self::Integer(lhs), Self::Integer(rhs)) => lhs.cmp(rhs),
            (Self::Float(lhs), Self::Float(rhs)) => match (lhs.is_nan(), rhs.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => lhs.total_cmp(rhs),
            },
            (Self::Long(lhs), Self::Long(rhs)) => lhs.cmp(rhs),
            (Self::Double(lhs), Self::Double(rhs)) => match (lhs.is_nan(), rhs.is_nan()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => lhs.total_cmp(rhs),
            },
            (Self::String(lhs), Self::String(rhs)) => lhs.cmp(rhs),
            (Self::Class(lhs), Self::Class(rhs)) => lhs.cmp(rhs),
            (Self::Handle(lhs), Self::Handle(rhs)) => lhs.cmp(rhs),
            (Self::MethodType(lhs), Self::MethodType(rhs)) => lhs.cmp(rhs),
            (Self::Dynamic(lhs0, lhs1, lhs2), Self::Dynamic(rhs0, rhs1, rhs2)) => {
                (lhs0, lhs1, lhs2).cmp(&(rhs0, rhs1, rhs2))
            }
            _ => self.rank().cmp(&other.rank()),
        }
    }
}

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
                v0.hash(state);
                v1.hash(state);
                v2.hash(state);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeSet, HashSet, hash_map::DefaultHasher},
        hash::{Hash, Hasher},
    };

    use super::ConstantValue;

    #[test]
    fn floating_point_order_and_identity() {
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
                    let expected = i.min(6).cmp(&j.min(6));
                    assert_eq!(lhs.cmp(rhs), expected);
                    assert_eq!(lhs.partial_cmp(rhs), Some(expected));
                    assert_eq!(lhs == rhs, expected.is_eq());
                    if lhs == rhs {
                        let mut lhs_hash = DefaultHasher::new();
                        let mut rhs_hash = DefaultHasher::new();
                        lhs.hash(&mut lhs_hash);
                        rhs.hash(&mut rhs_hash);
                        assert_eq!(lhs_hash.finish(), rhs_hash.finish());
                    }
                }
            }
            assert_eq!(values.iter().collect::<BTreeSet<_>>().len(), 7);
            assert_eq!(values.iter().collect::<HashSet<_>>().len(), 7);
        }
    }

    #[test]
    fn primitive_variant_order() {
        let values = [
            ConstantValue::Null,
            ConstantValue::Integer(i32::MAX),
            ConstantValue::Float(f32::NAN),
            ConstantValue::Long(i64::MIN),
            ConstantValue::Double(f64::NEG_INFINITY),
        ];
        for (i, lhs) in values.iter().enumerate() {
            for (j, rhs) in values.iter().enumerate() {
                assert_eq!(lhs.cmp(rhs), i.cmp(&j));
            }
        }
    }
}
