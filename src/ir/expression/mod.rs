//! Module for the expressions in Moka IR.
use std::fmt::{Display, Formatter};

use itertools::Itertools;

use super::Argument;

use crate::jvm::{
    class::ClassReference,
    code::ProgramCounter,
    field::ConstantValue,
    method::{MethodDescriptor, MethodReference},
};

mod array;
mod condition;
mod conversion;
mod field;
mod lock;
mod math;

pub use {array::*, condition::*, conversion::*, field::*, lock::*, math::*};

/// Represents an expression in the Moka IR.
/// It may or may not generate a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A constant value.
    Const(ConstantValue),
    /// A function call
    /// See the following documentation for more information:
    /// - [`invokestatic`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokestatic)
    /// - [`invokevirtual`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokevirtual)
    /// - [`invokespecial`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokespecial)
    /// - [`invokeinterface`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokeinterface)
    Call(MethodReference, Option<Argument>, Vec<Argument>),
    /// A call to a bootstrap method to create a closure.  
    /// See the following documentation for more information:
    /// - [`invokedynamic`](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-6.html#jvms-6.5.invokedynamic)
    GetClosure(u16, String, Vec<Argument>, MethodDescriptor),
    /// A mathematical operation.
    Math(MathOperation),
    /// A field access.
    Field(FieldAccess),
    /// An array operation.
    Array(ArrayOperation),
    /// A type conversion.
    Conversion(ConversionOperation),
    /// Throws an exception.
    Throw(Argument),
    /// An operation on a monitor.
    Synchronization(LockOperation),
    /// Creates a new object.
    New(ClassReference),
    /// A return address.
    Subroutine {
        /// The address to return to.
        return_address: ProgramCounter,
        /// The address where the subroutine starts.
        target: ProgramCounter,
    },
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Expression::*;
        match self {
            Const(c) => c.fmt(f),
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
            Call(method, None, args) => write!(
                f,
                "call {}({}) // desc: {}",
                method,
                args.iter().map(|it| it.to_string()).join(", "),
                method.descriptor.to_string()
            ),
            Call(method, Some(receiver), args) => write!(
                f,
                "call {}::{}({}) // owner: {}, desc: {}",
                receiver,
                method.name,
                args.iter().map(|it| it.to_string()).join(", "),
                method.owner.binary_name,
                method.descriptor.to_string()
            ),
            GetClosure(bootstrap_method_idx, name, args, descriptor) => write!(
                f,
                "get_closure#{}[{}]({}) // desc: {}",
                bootstrap_method_idx,
                name,
                args.iter().map(|it| it.to_string()).join(", "),
                descriptor.to_string()
            ),
        }
    }
}
