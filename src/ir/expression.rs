use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::elements::{
    instruction::ProgramCounter,
    method::MethodDescriptor,
    references::{ClassReference, MethodReference},
    ConstantValue,
};

use super::{expressions::*, ValueRef};

/// Represents an expression in the Moka IR.
/// It may or may not generate a value.
#[derive(Debug)]
pub enum Expression {
    /// A constant value.
    Const(ConstantValue),
    /// A function call
    /// See the following documentation for more information:
    /// - [`invokestatic`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokestatic)
    /// - [`invokevirtual`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokevirtual)
    /// - [`invokespecial`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokespecial)
    /// - [`invokeinterface`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokeinterface)
    Call(MethodReference, Vec<ValueRef>),
    /// A call to a bootstrap method to create a closure.  
    /// See the following documentation for more information:
    /// - [`invokedynamic`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokedynamic)
    GetClosure(u16, String, Vec<ValueRef>, MethodDescriptor),
    /// A mathematical operation.
    Math(MathOperation),
    /// A field access.
    Field(FieldAccess),
    /// An array operation.
    Array(ArrayOperation),
    /// A type conversion.
    Conversion(ConversionOperation),
    /// Throws an exception.
    Throw(ValueRef),
    /// An operation on a monitor.
    Synchronization(LockOperation),
    /// Creates a new object.
    New(ClassReference),
    /// A return address.
    Subroutine {
        return_address: ProgramCounter,
        target: ProgramCounter,
    },
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Expression::*;
        match self {
            Const(c) => write!(f, "{:?}", c),
            Subroutine {
                target,
                return_address,
            } => write!(f, "subroutine to {}, return to {}", target, return_address),
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
                "get_closure#{}[{}]({}) // descriptor: {}",
                bootstrap_method_idx,
                name,
                args.iter().map(|it| it.to_string()).join(", "),
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
