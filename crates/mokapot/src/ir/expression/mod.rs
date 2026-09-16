//! Module for the expressions in Moka IR.
use std::collections::HashSet;
use std::fmt;

use itertools::Itertools;

use super::{TryMapValues, ValueId};
use crate::{
    jvm::{
        ConstantValue,
        code::ProgramCounter,
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
/// A branch predicate over scalar SSA values, JVM constants, and legacy
/// return-address tokens.
pub type Predicate = Condition<crate::ir::control_flow::path_condition::PathValue>;
pub use conversion::Operation as Conversion;
pub use field::Access as FieldAccess;
pub use lock::Operation as LockOperation;
pub use math::NaNTreatment;
pub use math::Operation as MathOperation;

/// An expression parameterized by the lifting operand representation.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From)]
pub enum Expression<OP = ValueId> {
    /// A constant value.
    Const(ConstantValue),
    /// A legacy subroutine return-address token.
    ///
    /// The token names the instruction following the `jsr` or `jsr_w` that
    /// produced it. It is not an integer address and can only be consumed by a
    /// legacy subroutine return.
    ReturnAddress(ProgramCounter),
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
    Math(#[from] MathOperation<OP>),
    /// A field access.
    Field(#[from] FieldAccess<OP>),
    /// An array operation.
    Array(#[from] ArrayOperation<OP>),
    /// A type conversion.
    Conversion(#[from] Conversion<OP>),
    /// An operation on a monitor.
    Synchronization(#[from] LockOperation<OP>),
    /// Creates a new object.
    New(ClassRef),
}

impl<OP: fmt::Display> fmt::Display for Expression<OP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Const(value) => value.fmt(f),
            Self::ReturnAddress(continuation) => write!(f, "return_address {continuation}"),
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

impl<OP, OUT> TryMapValues<OUT> for Expression<OP> {
    type Value = OP;
    type Mapped = Expression<OUT>;

    fn try_map_values<E>(
        self,
        mut remap: impl FnMut(OP) -> Result<OUT, E>,
    ) -> Result<Expression<OUT>, E> {
        Ok(match self {
            Self::Const(value) => Expression::Const(value),
            Self::ReturnAddress(continuation) => Expression::ReturnAddress(continuation),
            Self::Call { method, this, args } => Expression::Call {
                method,
                this: this.map(&mut remap).transpose()?,
                args: args.into_iter().map(&mut remap).collect::<Result<_, _>>()?,
            },
            Self::Closure {
                name,
                captures,
                bootstrap_method_index,
                closure_descriptor,
            } => Expression::Closure {
                name,
                captures: captures
                    .into_iter()
                    .map(&mut remap)
                    .collect::<Result<_, _>>()?,
                bootstrap_method_index,
                closure_descriptor,
            },
            Self::Math(operation) => Expression::Math(operation.try_map_values(remap)?),
            Self::Field(access) => Expression::Field(access.try_map_values(remap)?),
            Self::Array(operation) => Expression::Array(operation.try_map_values(remap)?),
            Self::Conversion(operation) => Expression::Conversion(operation.try_map_values(remap)?),
            Self::Synchronization(operation) => {
                Expression::Synchronization(operation.try_map_values(remap)?)
            }
            Self::New(class) => Expression::New(class),
        })
    }
}

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

#[cfg(test)]
mod tests {
    use super::{Expression, MathOperation, TryMapValues};
    use crate::jvm::code::ProgramCounter;

    #[test]
    fn maps_nested_values_and_propagates_errors() {
        let expression = Expression::Math(MathOperation::Add(1_u8, 2));
        assert_eq!(
            expression.try_map_values(|value| Ok::<_, ()>(u16::from(value) + 10)),
            Ok(Expression::Math(MathOperation::Add(11_u16, 12)))
        );

        let expression = Expression::Math(MathOperation::Add(1_u8, 2));
        assert_eq!(
            expression.try_map_values(|value| {
                if value == 2 {
                    Err("unmapped")
                } else {
                    Ok(value)
                }
            }),
            Err("unmapped")
        );

        assert_eq!(
            Expression::<u8>::Const(crate::jvm::ConstantValue::Integer(3))
                .try_map_values(|_| Err::<u16, _>("unreachable")),
            Ok(Expression::Const(crate::jvm::ConstantValue::Integer(3)))
        );

        let continuation = ProgramCounter::from(0x12);
        assert_eq!(
            Expression::<u8>::ReturnAddress(continuation)
                .try_map_values(|_| Err::<u16, _>("unreachable")),
            Ok(Expression::ReturnAddress(continuation))
        );
        assert_eq!(
            Expression::<u8>::ReturnAddress(continuation).to_string(),
            "return_address #0012"
        );
    }
}
