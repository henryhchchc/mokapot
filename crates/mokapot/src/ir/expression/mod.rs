//! Module for the expressions in Moka IR.
use std::collections::HashSet;

use itertools::Itertools;

use super::{Identifier, Operand};
use crate::{
    jvm::{
        ConstantValue,
        references::{ClassRef, MethodRef},
    },
    types::method_descriptor::MethodDescriptor,
};

mod array;
mod condition;
mod conversion;
mod field;
mod lock;
mod math;

pub use array::Operation as ArrayOperation;
pub use condition::Condition;
pub use conversion::Operation as Conversion;
pub use field::Access as FieldAccess;
pub use lock::Operation as LockOperation;
pub use math::{NaNTreatment, Operation as MathOperation};

/// Represents an expression in the Moka IR.
/// It may or may not generate a value.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Expression {
    /// A constant value.
    Const(ConstantValue),
    /// A function call
    /// Corresponds to the following JVM instructions:
    /// - `invokestatic`
    /// - `invokevirtual`
    /// - `invokespecial`
    /// - `invokeinterface`
    #[display(
        "call {} {}{}::{}({})",
        method.descriptor.return_type,
        this.as_ref().map(|it| format!("{it}@")).unwrap_or_default(),
        method.owner,
        method.name,
        args.iter().map(std::string::ToString::to_string).join(", "),
    )]
    Call {
        /// The method being called.
        method: MethodRef,
        /// [`Some`] argument for the `this` object if the method is an instance method.
        /// [`None`] if the method is `static` or `native`.
        this: Option<Operand>,
        /// A list of arguments.
        args: Vec<Operand>,
    },
    /// A call to a bootstrap method to create a closure.
    /// Corresponds to the following JVM instructions:
    /// - `invokedynamic`
    #[display(
        "closure {} {}#{}({})",
        closure_descriptor.return_type,
        name,
        bootstrap_method_index,
        captures.iter().map(ToString::to_string).join(", "),
    )]
    Closure {
        /// The name of the closure.
        name: String,
        /// The arguments captured by the closure.
        captures: Vec<Operand>,
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
    /// An operation on a monitor.
    Synchronization(LockOperation),
    /// Creates a new object.
    #[display("new {_0}")]
    New(ClassRef),
    /// A legacy subroutine return address.
    #[display("subroutine_return_address")]
    SubroutineReturnAddress,
}

impl Expression {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<Identifier> {
        match self {
            Self::Call { this, args, .. } => this.iter().chain(args).flatten().copied().collect(),
            Self::Closure { captures, .. } => captures.iter().flatten().copied().collect(),
            Self::Math(math_op) => math_op.uses(),
            Self::Field(field_op) => field_op.uses(),
            Self::Array(array_op) => array_op.uses(),
            Self::Conversion(conv_op) => conv_op.uses(),
            Self::Synchronization(monitor_op) => monitor_op.uses(),
            _ => HashSet::default(),
        }
    }
}
