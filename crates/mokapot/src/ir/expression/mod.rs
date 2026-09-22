//! Module for the expressions in Moka IR.
use std::{collections::HashSet, fmt};

use itertools::Itertools;

use super::ValueId;
use crate::{
    jvm::{
        ConstantValue,
        references::{ClassRef, MethodRef},
    },
    types::method_descriptor::MethodDescriptor,
};

mod array;
mod conversion;
mod field;
mod literal;
mod lock;
mod math;
mod predicate;

pub use array::Operation as ArrayOperation;
pub use conversion::Operation as Conversion;
pub use field::Access as FieldAccess;
pub use literal::BooleanVariable;
pub use lock::Operation as LockOperation;
pub use math::{NaNTreatment, Operation as MathOperation};
pub use predicate::{PathValue, Predicate};

/// An expression over method-local SSA values.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From)]
pub enum Expression {
    /// A constant value.
    Const(ConstantValue),
    /// A function call.
    Call {
        /// The method being called.
        method: MethodRef,
        /// The receiver for an instance method.
        this: Option<ValueId>,
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
    New(ClassRef),
}

impl fmt::Display for Expression {
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
    use super::*;
    use crate::{
        ir::IdAllocator,
        jvm::references::FieldRef,
        types::{field_type::FieldType, reference_type::ReferenceType},
    };

    /// Every variant reports exactly its operands, including the payload shapes that differ in arity.
    #[test]
    fn uses_reports_exactly_the_operands_of_each_variant() {
        let mut ids = IdAllocator::<ValueId>::default();
        let (receiver, first, second) = (ids.new_id(), ids.new_id(), ids.new_id());
        let owner: ReferenceType = "java/lang/Object".parse().expect("a valid class name");
        let field_type: FieldType = "I".parse().expect("a valid field type");
        let descriptor: MethodDescriptor = "(II)I".parse().expect("a valid descriptor");
        let field = FieldRef {
            owner: owner.clone(),
            name: "value".to_owned(),
            field_type,
        };

        let str_type = "java/lang/String".parse().expect("a valid class name");
        let cases: [(Expression, HashSet<ValueId>); 12] = [
            (Expression::Const(ConstantValue::Integer(7)), HashSet::new()),
            (Expression::New(str_type), HashSet::new()),
            (
                Expression::Call {
                    method: MethodRef {
                        owner: owner.clone(),
                        name: "f".to_owned(),
                        descriptor: descriptor.clone(),
                    },
                    this: Some(receiver),
                    args: vec![first, second],
                },
                HashSet::from([receiver, first, second]),
            ),
            (
                Expression::Closure {
                    name: "lambda".to_owned(),
                    captures: vec![first, second],
                    bootstrap_method_index: 0,
                    closure_descriptor: descriptor,
                },
                HashSet::from([first, second]),
            ),
            (
                Expression::Math(MathOperation::Add(first, second)),
                HashSet::from([first, second]),
            ),
            (
                Expression::Math(MathOperation::Negate(first)),
                HashSet::from([first]),
            ),
            (
                Expression::Field(FieldAccess::ReadStatic {
                    field: field.clone(),
                }),
                HashSet::new(),
            ),
            (
                Expression::Field(FieldAccess::WriteInstance {
                    object_ref: receiver,
                    field,
                    value: first,
                }),
                HashSet::from([receiver, first]),
            ),
            (
                Expression::Array(ArrayOperation::Write {
                    array_ref: receiver,
                    index: first,
                    value: second,
                }),
                HashSet::from([receiver, first, second]),
            ),
            (
                Expression::Array(ArrayOperation::Length {
                    array_ref: receiver,
                }),
                HashSet::from([receiver]),
            ),
            (
                Expression::Conversion(Conversion::CheckCast(first, owner)),
                HashSet::from([first]),
            ),
            (
                Expression::Synchronization(LockOperation::Acquire(first)),
                HashSet::from([first]),
            ),
        ];

        for (expression, expected) in cases {
            assert_eq!(
                expression.uses(),
                expected,
                "{expression} reports the wrong operands"
            );
        }
    }
}
