//! Module for the expressions in Moka IR.
use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
};

use itertools::Itertools;

use super::{Argument, Identifier};

use crate::{
    jvm::{
        code::ProgramCounter,
        references::{ClassRef, MethodRef},
        ConstantValue,
    },
    types::method_descriptor::MethodDescriptor,
};

mod array;
mod condition;
mod conversion;
mod field;
mod lock;
mod math;

pub use {
    array::Operation as ArrayOperation,
    condition::Condition,
    conversion::Operaion as Conversion,
    field::Access as FieldAccess,
    lock::Operation as LockOperation,
    math::{NaNTreatment, Operation as MathOperation},
};

/// Represents an expression in the Moka IR.
/// It may or may not generate a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    /// A constant value.
    Const(ConstantValue),
    /// A function call
    /// Corresponds to the following JVM instructions:
    /// - `invokestatic`
    /// - `invokevirtual`
    /// - `invokespecial`
    /// - `invokeinterface`
    Call {
        /// The method being called.
        method: MethodRef,
        /// [`Some`] argument for the `this` object if the method is an instance method.
        /// [`None`] if the method is `static` or `native`.
        this: Option<Argument>,
        /// A list of arguments.
        args: Vec<Argument>,
    },
    /// A call to a bootstrap method to create a closure.  
    /// Corresponds to the following JVM instructions:
    /// - `invokedynamic`
    Closure {
        /// The name of the closure.
        name: String,
        /// The arguments captured by the closure.
        captures: Vec<Argument>,
        /// The index of the bootstrap method.
        bootstrap_method_index: u16,
        /// The descriptor of the closure generation.
        closure_descriptor: MethodDescriptor,
    },
    /// A mathematical operation.
    Math(MathOperation),
    /// A field access.
    Field(FieldAccess),
    /// An array operation.
    Array(ArrayOperation),
    /// A type conversion.
    Conversion(Conversion),
    /// Throws an exception.
    Throw(Argument),
    /// An operation on a monitor.
    Synchronization(LockOperation),
    /// Creates a new object.
    New(ClassRef),
    /// A return address.
    Subroutine {
        /// The address to return to.
        return_address: ProgramCounter,
        /// The address where the subroutine starts.
        target: ProgramCounter,
    },
}

impl Expression {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Call { this, args, .. } => this.iter().chain(args).flatten().copied().collect(),
            Self::Closure { captures, .. } => captures.iter().flatten().copied().collect(),
            Self::Math(math_op) => math_op.uses(),
            Self::Field(field_op) => field_op.uses(),
            Self::Array(array_op) => array_op.uses(),
            Self::Conversion(conv_op) => conv_op.uses(),
            Self::Throw(arg) => arg.iter().copied().collect(),
            Self::Synchronization(monitor_op) => monitor_op.uses(),
            _ => BTreeSet::default(),
        }
    }
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Const(c) => c.fmt(f),
            Self::Subroutine {
                target,
                return_address,
            } => write!(f, "subroutine {target}, return to {return_address}"),
            Self::Field(field_op) => field_op.fmt(f),
            Self::Array(array_op) => array_op.fmt(f),
            Self::Math(math_op) => math_op.fmt(f),
            Self::Throw(value) => write!(f, "throw {value}"),
            Self::Synchronization(monitor_op) => monitor_op.fmt(f),
            Self::New(class) => write!(f, "new {class}"),
            Self::Conversion(conv_op) => conv_op.fmt(f),
            Self::Call {
                method,
                this: None,
                args,
            } => write!(
                f,
                "call {} {}({})",
                method.descriptor.return_type,
                method,
                args.iter().map(std::string::ToString::to_string).join(", "),
            ),
            Self::Call {
                method,
                this: Some(receiver),
                args,
            } => write!(
                f,
                "call {} {}@{}::{}({})",
                method.descriptor.return_type,
                receiver,
                method.owner,
                method.name,
                args.iter().map(std::string::ToString::to_string).join(", "),
            ),
            Self::Closure {
                bootstrap_method_index,
                name,
                captures,
                closure_descriptor,
            } => write!(
                f,
                "closure {} {}#{}({})",
                closure_descriptor.return_type,
                name,
                bootstrap_method_index,
                captures
                    .iter()
                    .map(std::string::ToString::to_string)
                    .join(", "),
            ),
        }
    }
}
