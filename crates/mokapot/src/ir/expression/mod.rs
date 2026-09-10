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

/// An array operation over scalar SSA values.
pub type ArrayOperation = array::Operation<ValueId>;
/// A condition over scalar SSA values.
pub type Condition = condition::Condition<ValueId>;
/// A branch predicate over scalar SSA values and JVM constants.
pub type Predicate = condition::Condition<crate::ir::control_flow::path_condition::Value>;
/// A scalar value conversion.
pub type Conversion = conversion::Operation<ValueId>;
/// A field access over scalar SSA values.
pub type FieldAccess = field::Access<ValueId>;
/// A monitor operation over a scalar SSA value.
pub type LockOperation = lock::Operation<ValueId>;
/// A mathematical operation over scalar SSA values.
pub type MathOperation = math::Operation<ValueId>;
pub use math::NaNTreatment;

pub(crate) use array::Operation as LiftedArrayOperation;
pub(crate) use condition::Condition as LiftedCondition;
pub(crate) use conversion::Operation as LiftedConversion;
pub(crate) use field::Access as LiftedFieldAccess;
pub(crate) use lock::Operation as LiftedLockOperation;
pub(crate) use math::Operation as LiftedMathOperation;

mod model {
    use std::fmt;

    use itertools::Itertools;

    use super::{
        ClassRef, ConstantValue, LiftedArrayOperation, LiftedConversion, LiftedFieldAccess,
        LiftedLockOperation, LiftedMathOperation, MethodDescriptor, MethodRef,
    };

    /// An expression parameterized by the lifting operand representation.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Expression<OP> {
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
        Math(LiftedMathOperation<OP>),
        /// A field access.
        Field(LiftedFieldAccess<OP>),
        /// An array operation.
        Array(LiftedArrayOperation<OP>),
        /// A type conversion.
        Conversion(LiftedConversion<OP>),
        /// An operation on a monitor.
        Synchronization(LiftedLockOperation<OP>),
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

/// An expression in Moka IR over scalar SSA values.
pub type Expression = model::Expression<ValueId>;
pub(crate) use model::Expression as LiftedExpression;

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
