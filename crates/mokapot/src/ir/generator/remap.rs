//! Rewrites the single IR value identity space during canonicalization.

use crate::ir::{
    Operation, Successor, Terminator, ValueId,
    control_flow::{
        ControlTransfer,
        path_condition::{BooleanVariable, BranchGuard, PathValue},
    },
    expression::{
        ArrayOperation, Conversion, Expression, FieldAccess, LockOperation, MathOperation,
        Predicate,
    },
};

pub(super) trait RemapValues {
    fn try_remap_values<E>(
        &mut self,
        remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
    ) -> Result<(), E>;
}

fn remap_value<E>(
    value: &mut ValueId,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    *value = remap(*value)?;
    Ok(())
}

impl RemapValues for Operation {
    fn try_remap_values<E>(
        &mut self,
        remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
    ) -> Result<(), E> {
        match self {
            Self::Definition { value, expr } => {
                remap_value(value, remap)?;
                remap_expression(expr, remap)
            }
            Self::Effect { expr } => remap_expression(expr, remap),
        }
    }
}

impl RemapValues for Terminator<Successor> {
    fn try_remap_values<E>(
        &mut self,
        remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
    ) -> Result<(), E> {
        match self {
            Self::Throw { value, .. }
            | Self::Return { value: Some(value) }
            | Self::TryReturn {
                value: Some(value), ..
            } => remap_value(value, remap),
            Self::Goto { .. }
            | Self::Branch { .. }
            | Self::Switch { .. }
            | Self::Return { value: None }
            | Self::TryReturn { value: None, .. } => Ok(()),
            Self::Try { operation, .. } => operation.try_remap_values(remap),
        }
    }
}

impl RemapValues for ControlTransfer {
    fn try_remap_values<E>(
        &mut self,
        remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
    ) -> Result<(), E> {
        let Self::Conditional(guard) = self else {
            return Ok(());
        };
        let old = std::mem::replace(guard, BranchGuard::one());
        *guard = old
            .into_iter()
            .map(|literal| match literal {
                BooleanVariable::Positive(mut predicate) => {
                    remap_predicate(&mut predicate, remap)?;
                    Ok(BooleanVariable::Positive(predicate))
                }
                BooleanVariable::Negative(mut predicate) => {
                    remap_predicate(&mut predicate, remap)?;
                    Ok(BooleanVariable::Negative(predicate))
                }
            })
            .collect::<Result<_, E>>()?;
        Ok(())
    }
}

fn remap_expression<E>(
    expression: &mut Expression,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    match expression {
        Expression::Const(_) | Expression::New(_) => Ok(()),
        Expression::Call { this, args, .. } => {
            if let Some(value) = this {
                remap_value(value, remap)?;
            }
            for value in args {
                remap_value(value, remap)?;
            }
            Ok(())
        }
        Expression::Closure { captures, .. } => {
            for value in captures {
                remap_value(value, remap)?;
            }
            Ok(())
        }
        Expression::Math(operation) => remap_math(operation, remap),
        Expression::Field(access) => remap_field(access, remap),
        Expression::Array(operation) => remap_array(operation, remap),
        Expression::Conversion(operation) => remap_conversion(operation, remap),
        Expression::Synchronization(operation) => match operation {
            LockOperation::Acquire(value) | LockOperation::Release(value) => {
                remap_value(value, remap)
            }
        },
    }
}

fn remap_math<E>(
    operation: &mut MathOperation,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    match operation {
        MathOperation::Add(lhs, rhs)
        | MathOperation::Subtract(lhs, rhs)
        | MathOperation::Multiply(lhs, rhs)
        | MathOperation::Divide(lhs, rhs)
        | MathOperation::Remainder(lhs, rhs)
        | MathOperation::ShiftLeft(lhs, rhs)
        | MathOperation::ShiftRight(lhs, rhs)
        | MathOperation::LogicalShiftRight(lhs, rhs)
        | MathOperation::BitwiseAnd(lhs, rhs)
        | MathOperation::BitwiseOr(lhs, rhs)
        | MathOperation::BitwiseXor(lhs, rhs)
        | MathOperation::LongComparison(lhs, rhs)
        | MathOperation::FloatingPointComparison(lhs, rhs, _) => {
            remap_value(lhs, remap)?;
            remap_value(rhs, remap)
        }
        MathOperation::Negate(value) | MathOperation::Increment(value, _) => {
            remap_value(value, remap)
        }
    }
}

fn remap_field<E>(
    access: &mut FieldAccess,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    match access {
        FieldAccess::ReadStatic { .. } => Ok(()),
        FieldAccess::WriteStatic { value, .. }
        | FieldAccess::ReadInstance {
            object_ref: value, ..
        } => remap_value(value, remap),
        FieldAccess::WriteInstance {
            object_ref, value, ..
        } => {
            remap_value(object_ref, remap)?;
            remap_value(value, remap)
        }
    }
}

fn remap_array<E>(
    operation: &mut ArrayOperation,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    match operation {
        ArrayOperation::New { length, .. } | ArrayOperation::Length { array_ref: length } => {
            remap_value(length, remap)
        }
        ArrayOperation::NewMultiDim { dimensions, .. } => {
            for value in dimensions {
                remap_value(value, remap)?;
            }
            Ok(())
        }
        ArrayOperation::Read { array_ref, index } => {
            remap_value(array_ref, remap)?;
            remap_value(index, remap)
        }
        ArrayOperation::Write {
            array_ref,
            index,
            value,
        } => {
            remap_value(array_ref, remap)?;
            remap_value(index, remap)?;
            remap_value(value, remap)
        }
    }
}

fn remap_conversion<E>(
    operation: &mut Conversion,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    let value = match operation {
        Conversion::Int2Long(value)
        | Conversion::Int2Float(value)
        | Conversion::Int2Double(value)
        | Conversion::Long2Int(value)
        | Conversion::Long2Float(value)
        | Conversion::Long2Double(value)
        | Conversion::Float2Int(value)
        | Conversion::Float2Long(value)
        | Conversion::Float2Double(value)
        | Conversion::Double2Int(value)
        | Conversion::Double2Long(value)
        | Conversion::Double2Float(value)
        | Conversion::Int2Byte(value)
        | Conversion::Int2Char(value)
        | Conversion::Int2Short(value)
        | Conversion::CheckCast(value, _)
        | Conversion::InstanceOf(value, _) => value,
    };
    remap_value(value, remap)
}

fn remap_predicate<E>(
    predicate: &mut Predicate,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    match predicate {
        Predicate::Equal(lhs, rhs)
        | Predicate::NotEqual(lhs, rhs)
        | Predicate::LessThan(lhs, rhs)
        | Predicate::LessThanOrEqual(lhs, rhs)
        | Predicate::GreaterThan(lhs, rhs)
        | Predicate::GreaterThanOrEqual(lhs, rhs) => {
            remap_path_value(lhs, remap)?;
            remap_path_value(rhs, remap)
        }
        Predicate::IsNull(value)
        | Predicate::IsNotNull(value)
        | Predicate::IsZero(value)
        | Predicate::IsNonZero(value)
        | Predicate::IsPositive(value)
        | Predicate::IsNegative(value)
        | Predicate::IsNonNegative(value)
        | Predicate::IsNonPositive(value) => remap_path_value(value, remap),
    }
}

fn remap_path_value<E>(
    value: &mut PathValue,
    remap: &mut impl FnMut(ValueId) -> Result<ValueId, E>,
) -> Result<(), E> {
    if let PathValue::Variable(value) = value {
        remap_value(value, remap)?;
    }
    Ok(())
}
