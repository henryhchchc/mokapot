//! Module for the expressions in Moka IR.
use crate::types::class_name::ClassName;

use std::fmt;

use derive_more::From;
use itertools::Itertools;

use super::ValueId;
use crate::{
    jvm::{ConstantValue, references::MethodRef},
    types::method_descriptor::MethodDescriptor,
};

mod array;
mod call;
mod conversion;
mod field;
mod literal;
mod lock;
mod math;
mod predicate;

pub use array::Operation as ArrayOperation;
pub use call::InvocationKind;
pub use conversion::Operation as Conversion;
pub use field::Access as FieldAccess;
pub use literal::BooleanVariable;
pub use lock::Operation as LockOperation;
pub use math::{NaNTreatment, Operation as MathOperation};
pub use predicate::{PathValue, Predicate};

/// An expression over method-local SSA values.
#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum Expression {
    /// A constant value.
    Const(ConstantValue),
    /// A function call.
    Call {
        /// The invocation opcode's dispatch semantics and receiver.
        kind: InvocationKind,
        /// The unresolved symbolic method reference.
        method: MethodRef,
        /// The arguments.
        args: Vec<ValueId>,
    },
    /// A call to a bootstrap method to create a closure.
    Closure {
        /// The name of the closure.
        name: String,
        /// The values captured by the closure.
        captures: Vec<ValueId>,
        /// The index of the bootstrap method.
        bootstrap_method_index: u16,
        /// The descriptor of the closure generation.
        closure_descriptor: MethodDescriptor,
    },
    /// A mathematical operation.
    Math(#[from] MathOperation),
    /// A field access.
    Field(#[from] FieldAccess),
    /// An array operation.
    Array(#[from] ArrayOperation),
    /// A type conversion.
    Conversion(#[from] Conversion),
    /// An operation on a monitor.
    Synchronization(#[from] LockOperation),
    /// Creates a new object.
    New(ClassName),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(value) => value.fmt(f),
            Self::Call { kind, method, args } => {
                write!(f, "call {kind} {} ", method.descriptor.return_type)?;
                match kind {
                    InvocationKind::Static => {}
                    InvocationKind::Virtual { this }
                    | InvocationKind::Interface { this }
                    | InvocationKind::Special { this } => write!(f, "{this}@")?,
                }
                write!(
                    f,
                    "{}::{}({})",
                    method.owner,
                    method.name,
                    args.iter().format(", ")
                )
            }
            Self::Closure {
                name,
                captures,
                bootstrap_method_index,
                closure_descriptor,
            } => write!(
                f,
                "closure {} {}#{}({})",
                closure_descriptor.return_type,
                name,
                bootstrap_method_index,
                captures.iter().format(", "),
            ),
            Self::Math(operation) => operation.fmt(f),
            Self::Field(access) => access.fmt(f),
            Self::Array(operation) => operation.fmt(f),
            Self::Conversion(operation) => operation.fmt(f),
            Self::Synchronization(operation) => operation.fmt(f),
            Self::New(class) => write!(f, "new {class}"),
        }
    }
}
