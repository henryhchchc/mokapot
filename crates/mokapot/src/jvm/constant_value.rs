//! JVM compile-time constant values.

use std::hash::Hash;

use crate::{
    intrinsics::see_jvm_spec,
    types::{field_type::FieldType, method_descriptor::MethodDescriptor},
};

use super::{JavaString, class::MethodHandle, references::ClassRef};

/// Denotes a compile-time constant value.
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
    Class(ClassRef),
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

impl PartialOrd for ConstantValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConstantValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        #[allow(clippy::enum_glob_use)]
        use ConstantValue::*;
        match (self, other) {
            (Null, Null) => std::cmp::Ordering::Equal,
            (Null, _) => std::cmp::Ordering::Less,
            (_, Null) => std::cmp::Ordering::Greater,
            (Integer(lhs), Integer(rhs)) => lhs.cmp(rhs),
            (Integer(_), _) => std::cmp::Ordering::Less,
            (_, Integer(_)) => std::cmp::Ordering::Greater,
            (Float(lhs), Float(rhs)) => lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal),
            (Float(_), _) => std::cmp::Ordering::Less,
            (_, Float(_)) => std::cmp::Ordering::Greater,
            (Long(lhs), Long(rhs)) => lhs.cmp(rhs),
            (Long(_), _) => std::cmp::Ordering::Less,
            (_, Long(_)) => std::cmp::Ordering::Greater,
            (Double(lhs), Double(rhs)) => lhs.partial_cmp(rhs).unwrap_or(std::cmp::Ordering::Equal),
            (Double(_), _) => std::cmp::Ordering::Less,
            (_, Double(_)) => std::cmp::Ordering::Greater,
            (String(lhs), String(rhs)) => lhs.cmp(rhs),
            (String(_), _) => std::cmp::Ordering::Less,
            (_, String(_)) => std::cmp::Ordering::Greater,
            (Class(lhs), Class(rhs)) => lhs.cmp(rhs),
            (Class(_), _) => std::cmp::Ordering::Less,
            (_, Class(_)) => std::cmp::Ordering::Greater,
            (Handle(lhs), Handle(rhs)) => lhs.cmp(rhs),
            (Handle(_), _) => std::cmp::Ordering::Less,
            (_, Handle(_)) => std::cmp::Ordering::Greater,
            (MethodType(lhs), MethodType(rhs)) => lhs.cmp(rhs),
            (MethodType(_), _) => std::cmp::Ordering::Less,
            (_, MethodType(_)) => std::cmp::Ordering::Greater,
            (Dynamic(lhs0, lhs1, lhs2), Dynamic(rhs0, rhs1, rhs2)) => lhs0
                .cmp(rhs0)
                .then_with(|| lhs1.cmp(rhs1))
                .then_with(|| lhs2.cmp(rhs2)),
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
