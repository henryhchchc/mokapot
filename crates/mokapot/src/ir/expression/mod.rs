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
        ir::test::prelude::{cls_r, field_ref, ids, ref_t},
        jvm::ConstantValue,
    };

    #[test]
    fn uses_reports_call_and_capture_operands() {
        let [receiver, first, second] = ids(0);
        let call = Expression::Call {
            method: MethodRef {
                owner: ref_t("java/lang/Object"),
                name: "f".to_owned(),
                descriptor: "(II)I".parse().unwrap(),
            },
            this: Some(receiver),
            args: vec![first, second],
        };
        let closure = Expression::Closure {
            name: "lambda".to_owned(),
            captures: vec![first, second],
            bootstrap_method_index: 0,
            closure_descriptor: "(II)I".parse().unwrap(),
        };
        assert_eq!(call.uses(), HashSet::from([receiver, first, second]));
        assert_eq!(closure.uses(), HashSet::from([first, second]));
    }

    #[test]
    fn constants_and_allocations_have_no_operands() {
        let expression = Expression::Const(ConstantValue::Integer(7));
        assert!(expression.uses().is_empty());
        assert!(Expression::New(cls_r("java/lang/Object")).uses().is_empty());
    }

    #[test]
    fn uses_forwards_to_each_nested_operation() {
        let [receiver, first, second] = ids(0);
        let math = MathOperation::Add(first, second);
        let access = FieldAccess::WriteInstance {
            object_ref: receiver,
            field: field_ref(),
            value: first,
        };
        let array = ArrayOperation::Write {
            array_ref: receiver,
            index: first,
            value: second,
        };
        let conversion = Conversion::CheckCast(first, ref_t("java/lang/Object"));
        let lock = LockOperation::Acquire(first);

        assert_eq!(Expression::Math(math.clone()).uses(), math.uses());
        assert_eq!(Expression::Field(access.clone()).uses(), access.uses());
        assert_eq!(Expression::Array(array.clone()).uses(), array.uses());
        let converted = Expression::Conversion(conversion.clone());
        assert_eq!(converted.uses(), conversion.uses());
        let synchronization = Expression::Synchronization(lock.clone());
        assert_eq!(synchronization.uses(), lock.uses());
    }
}
