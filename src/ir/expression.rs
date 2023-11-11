use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::{
    elements::{
        instruction::ProgramCounter,
        method::MethodDescriptor,
        references::{ClassReference, FieldReference, MethodReference},
        ConstantValue,
    },
    types::FieldType,
};

use super::{expressions::*, ValueRef};

#[derive(Debug)]
pub enum Expression {
    Const(ConstantValue),
    Call(MethodReference, Vec<ValueRef>),
    GetClosure(u16, String, Vec<ValueRef>, MethodDescriptor),
    Math(MathOperation),
    Field(FieldAccess),
    Array(ArrayOperation),
    Conversion(ConversionOperation),
    Throw(ValueRef),
    Synchronization(LockOperation),
    New(ClassReference),
    ReturnAddress(ProgramCounter),
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Expression::*;
        match self {
            Const(c) => write!(f, "{:?}", c),
            ReturnAddress(pc) => write!(f, "{:?}", pc),
            Field(field_op) => field_op.fmt(f),
            Array(array_op) => array_op.fmt(f),
            Math(math_op) => math_op.fmt(f),
            Throw(value) => write!(f, "throw {}", value),
            Synchronization(monitor_op) => monitor_op.fmt(f),
            New(class) => write!(f, "new {}", class),
            Conversion(conv_op) => conv_op.fmt(f),
            Call(method, args) => write!(
                f,
                "call {}({}) // descriptor: {}",
                method,
                args.iter().map(|it| it.to_string()).join(", "),
                method.descriptor().to_string()
            ),
            GetClosure(bootstrap_method_idx, name, args, descriptor) => write!(
                f,
                "get_closure#{}({}) // {}{}",
                bootstrap_method_idx,
                args.iter().map(|it| it.to_string()).join(", "),
                name,
                descriptor.to_string()
            ),
        }
    }
}

#[derive(Debug)]
pub enum Condition {
    Equal(ValueRef, ValueRef),
    NotEqual(ValueRef, ValueRef),
    LessThan(ValueRef, ValueRef),
    LessThanOrEqual(ValueRef, ValueRef),
    GreaterThan(ValueRef, ValueRef),
    GreaterThanOrEqual(ValueRef, ValueRef),
    IsNull(ValueRef),
    IsNotNull(ValueRef),
    Zero(ValueRef),
    NonZero(ValueRef),
    Positive(ValueRef),
    Negative(ValueRef),
    NonNegative(ValueRef),
    NonPositive(ValueRef),
}

impl Display for Condition {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Condition::*;
        match self {
            Equal(a, b) => write!(f, "{} == {}", a, b),
            NotEqual(a, b) => write!(f, "{} != {}", a, b),
            LessThan(a, b) => write!(f, "{} < {}", a, b),
            LessThanOrEqual(a, b) => write!(f, "{} <= {}", a, b),
            GreaterThan(a, b) => write!(f, "{} > {}", a, b),
            GreaterThanOrEqual(a, b) => write!(f, "{} >= {}", a, b),
            IsNull(a) => write!(f, "{} == null", a),
            IsNotNull(a) => write!(f, "{} != null", a),
            Zero(a) => write!(f, "{} == 0", a),
            NonZero(a) => write!(f, "{} != 0", a),
            Positive(a) => write!(f, "{} > 0", a),
            Negative(a) => write!(f, "{} < 0", a),
            NonNegative(a) => write!(f, "{} >= 0", a),
            NonPositive(a) => write!(f, "{} <= 0", a),
        }
    }
}
