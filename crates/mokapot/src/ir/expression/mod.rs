//! Module for the expressions in Moka IR.
use std::collections::HashSet;

use super::ValueId;
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
/// A branch predicate over scalar SSA values and JVM constants.
pub type Predicate = Condition<crate::ir::control_flow::path_condition::Value>;
pub use conversion::Operation as Conversion;
pub use field::Access as FieldAccess;
pub use lock::Operation as LockOperation;
pub use math::NaNTreatment;
pub use math::Operation as MathOperation;

mod model {
    use std::fmt;

    use itertools::Itertools;

    use super::{
        ArrayOperation, ClassRef, ConstantValue, Conversion, FieldAccess, LockOperation,
        MathOperation, MethodDescriptor, MethodRef, ValueId,
    };

    /// An expression parameterized by the lifting operand representation.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Expression<OP = ValueId> {
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
            this: Option<OP>,
            /// A list of arguments.
            args: Vec<OP>,
        },
        /// A call to a bootstrap method to create a closure.
        /// Corresponds to the following JVM instructions:
        /// - `invokedynamic`
        Closure {
            /// The name of the closure.
            name: String,
            /// The arguments captured by the closure.
            captures: Vec<OP>,
            /// The index of the bootstrap method.
            bootstrap_method_index: u16,
            /// The descriptor of the closure generation.
            closure_descriptor: MethodDescriptor,
        },
        /// A mathematical operation.
        Math(MathOperation<OP>),
        /// A field access.
        Field(FieldAccess<OP>),
        /// An array operation.
        Array(ArrayOperation<OP>),
        /// A type conversion.
        Conversion(Conversion<OP>),
        /// An operation on a monitor.
        Synchronization(LockOperation<OP>),
        /// Creates a new object.
        New(ClassRef),
    }

    impl<OP: fmt::Display> fmt::Display for Expression<OP> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Const(value) => value.fmt(f),
                Self::Call { method, this, args } => write!(
                    f,
                    "call {} {}{}::{}({})",
                    method.descriptor.return_type,
                    this.as_ref()
                        .map(|value| format!("{value}@"))
                        .unwrap_or_default(),
                    method.owner,
                    method.name,
                    args.iter().format(", "),
                ),
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
}

pub use model::Expression;

impl Expression {
    /// Returns the values used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::Call { this, args, .. } => this.iter().chain(args).copied().collect(),
            Self::Closure { captures, .. } => captures.iter().copied().collect(),
            Self::Math(math_op) => math_op.uses(),
            Self::Field(field_op) => field_op.uses(),
            Self::Array(array_op) => array_op.uses(),
            Self::Conversion(conv_op) => conv_op.uses(),
            Self::Synchronization(monitor_op) => monitor_op.uses(),
            _ => HashSet::default(),
        }
    }
}
