use super::MokaIRBuildError;
use crate::ir::{
    ValueId,
    control_flow::path_condition::{BooleanVariable, BranchGuard, LiftedValue, Value},
    control_flow::{ControlTransfer, LiftedControlTransfer},
    expression::{
        ArrayOperation, Conversion, Expression, FieldAccess, LiftedArrayOperation, LiftedCondition,
        LiftedConversion, LiftedExpression, LiftedFieldAccess, LiftedLockOperation,
        LiftedMathOperation, LockOperation, MathOperation, Predicate,
    },
};

pub(super) fn remap_expression<OP: std::fmt::Display>(
    expression: LiftedExpression<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<Expression, MokaIRBuildError> {
    Ok(match expression {
        LiftedExpression::Const(value) => Expression::Const(value),
        LiftedExpression::Call { method, this, args } => Expression::Call {
            method,
            this: this.map(remap).transpose()?,
            args: args.into_iter().map(remap).collect::<Result<_, _>>()?,
        },
        LiftedExpression::Closure {
            name,
            captures,
            bootstrap_method_index,
            closure_descriptor,
        } => Expression::Closure {
            name,
            captures: captures.into_iter().map(remap).collect::<Result<_, _>>()?,
            bootstrap_method_index,
            closure_descriptor,
        },
        LiftedExpression::Math(operation) => Expression::Math(remap_math(operation, remap)?),
        LiftedExpression::Field(access) => Expression::Field(remap_field(access, remap)?),
        LiftedExpression::Array(operation) => Expression::Array(remap_array(operation, remap)?),
        LiftedExpression::Conversion(operation) => {
            Expression::Conversion(remap_conversion(operation, remap)?)
        }
        LiftedExpression::Synchronization(operation) => {
            Expression::Synchronization(remap_lock(operation, remap)?)
        }
        LiftedExpression::New(class) => Expression::New(class),
    })
}

fn remap_math<OP: std::fmt::Display>(
    operation: LiftedMathOperation<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<MathOperation, MokaIRBuildError> {
    Ok(match operation {
        LiftedMathOperation::Add(a, b) => MathOperation::Add(remap(a)?, remap(b)?),
        LiftedMathOperation::Subtract(a, b) => MathOperation::Subtract(remap(a)?, remap(b)?),
        LiftedMathOperation::Multiply(a, b) => MathOperation::Multiply(remap(a)?, remap(b)?),
        LiftedMathOperation::Divide(a, b) => MathOperation::Divide(remap(a)?, remap(b)?),
        LiftedMathOperation::Remainder(a, b) => MathOperation::Remainder(remap(a)?, remap(b)?),
        LiftedMathOperation::Negate(value) => MathOperation::Negate(remap(value)?),
        LiftedMathOperation::Increment(value, amount) => {
            MathOperation::Increment(remap(value)?, amount)
        }
        LiftedMathOperation::ShiftLeft(a, b) => MathOperation::ShiftLeft(remap(a)?, remap(b)?),
        LiftedMathOperation::ShiftRight(a, b) => MathOperation::ShiftRight(remap(a)?, remap(b)?),
        LiftedMathOperation::LogicalShiftRight(a, b) => {
            MathOperation::LogicalShiftRight(remap(a)?, remap(b)?)
        }
        LiftedMathOperation::BitwiseAnd(a, b) => MathOperation::BitwiseAnd(remap(a)?, remap(b)?),
        LiftedMathOperation::BitwiseOr(a, b) => MathOperation::BitwiseOr(remap(a)?, remap(b)?),
        LiftedMathOperation::BitwiseXor(a, b) => MathOperation::BitwiseXor(remap(a)?, remap(b)?),
        LiftedMathOperation::LongComparison(a, b) => {
            MathOperation::LongComparison(remap(a)?, remap(b)?)
        }
        LiftedMathOperation::FloatingPointComparison(a, b, nan) => {
            MathOperation::FloatingPointComparison(remap(a)?, remap(b)?, nan)
        }
    })
}

fn remap_array<OP: std::fmt::Display>(
    operation: LiftedArrayOperation<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<ArrayOperation, MokaIRBuildError> {
    Ok(match operation {
        LiftedArrayOperation::New {
            element_type,
            length,
        } => ArrayOperation::New {
            element_type,
            length: remap(length)?,
        },
        LiftedArrayOperation::NewMultiDim {
            element_type,
            dimensions,
        } => ArrayOperation::NewMultiDim {
            element_type,
            dimensions: dimensions
                .into_iter()
                .map(remap)
                .collect::<Result<_, _>>()?,
        },
        LiftedArrayOperation::Read { array_ref, index } => ArrayOperation::Read {
            array_ref: remap(array_ref)?,
            index: remap(index)?,
        },
        LiftedArrayOperation::Write {
            array_ref,
            index,
            value,
        } => ArrayOperation::Write {
            array_ref: remap(array_ref)?,
            index: remap(index)?,
            value: remap(value)?,
        },
        LiftedArrayOperation::Length { array_ref } => ArrayOperation::Length {
            array_ref: remap(array_ref)?,
        },
    })
}

fn remap_field<OP: std::fmt::Display>(
    access: LiftedFieldAccess<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<FieldAccess, MokaIRBuildError> {
    Ok(match access {
        LiftedFieldAccess::ReadStatic { field } => FieldAccess::ReadStatic { field },
        LiftedFieldAccess::WriteStatic { field, value } => FieldAccess::WriteStatic {
            field,
            value: remap(value)?,
        },
        LiftedFieldAccess::ReadInstance { object_ref, field } => FieldAccess::ReadInstance {
            object_ref: remap(object_ref)?,
            field,
        },
        LiftedFieldAccess::WriteInstance {
            object_ref,
            field,
            value,
        } => FieldAccess::WriteInstance {
            object_ref: remap(object_ref)?,
            field,
            value: remap(value)?,
        },
    })
}

fn remap_conversion<OP: std::fmt::Display>(
    operation: LiftedConversion<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<Conversion, MokaIRBuildError> {
    Ok(match operation {
        LiftedConversion::Int2Long(value) => Conversion::Int2Long(remap(value)?),
        LiftedConversion::Int2Float(value) => Conversion::Int2Float(remap(value)?),
        LiftedConversion::Int2Double(value) => Conversion::Int2Double(remap(value)?),
        LiftedConversion::Long2Int(value) => Conversion::Long2Int(remap(value)?),
        LiftedConversion::Long2Float(value) => Conversion::Long2Float(remap(value)?),
        LiftedConversion::Long2Double(value) => Conversion::Long2Double(remap(value)?),
        LiftedConversion::Float2Int(value) => Conversion::Float2Int(remap(value)?),
        LiftedConversion::Float2Long(value) => Conversion::Float2Long(remap(value)?),
        LiftedConversion::Float2Double(value) => Conversion::Float2Double(remap(value)?),
        LiftedConversion::Double2Int(value) => Conversion::Double2Int(remap(value)?),
        LiftedConversion::Double2Long(value) => Conversion::Double2Long(remap(value)?),
        LiftedConversion::Double2Float(value) => Conversion::Double2Float(remap(value)?),
        LiftedConversion::Int2Byte(value) => Conversion::Int2Byte(remap(value)?),
        LiftedConversion::Int2Char(value) => Conversion::Int2Char(remap(value)?),
        LiftedConversion::Int2Short(value) => Conversion::Int2Short(remap(value)?),
        LiftedConversion::CheckCast(value, target) => Conversion::CheckCast(remap(value)?, target),
        LiftedConversion::InstanceOf(value, target) => {
            Conversion::InstanceOf(remap(value)?, target)
        }
    })
}

fn remap_lock<OP: std::fmt::Display>(
    operation: LiftedLockOperation<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<LockOperation, MokaIRBuildError> {
    Ok(match operation {
        LiftedLockOperation::Acquire(value) => LockOperation::Acquire(remap(value)?),
        LiftedLockOperation::Release(value) => LockOperation::Release(remap(value)?),
    })
}

pub(super) fn remap_transfer<OP: Eq + std::hash::Hash + std::fmt::Display>(
    transfer: LiftedControlTransfer<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<ControlTransfer, MokaIRBuildError> {
    Ok(match transfer {
        LiftedControlTransfer::Unconditional => ControlTransfer::Unconditional,
        LiftedControlTransfer::Normal => ControlTransfer::Normal,
        LiftedControlTransfer::Exception(types) => ControlTransfer::Exception(types),
        LiftedControlTransfer::Unwind => ControlTransfer::Unwind,
        LiftedControlTransfer::Conditional(guard) => ControlTransfer::Conditional(
            guard
                .into_iter()
                .map(|literal| remap_literal(literal, remap))
                .collect::<Result<BranchGuard<_>, _>>()?,
        ),
    })
}

fn remap_literal<OP: std::fmt::Display>(
    literal: BooleanVariable<LiftedCondition<LiftedValue<OP>>>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<BooleanVariable<Predicate>, MokaIRBuildError> {
    Ok(match literal {
        BooleanVariable::Positive(condition) => {
            BooleanVariable::Positive(remap_guard_condition(condition, remap)?)
        }
        BooleanVariable::Negative(condition) => {
            BooleanVariable::Negative(remap_guard_condition(condition, remap)?)
        }
    })
}

fn remap_guard_condition<OP: std::fmt::Display>(
    condition: LiftedCondition<LiftedValue<OP>>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<Predicate, MokaIRBuildError> {
    let value = |value| match value {
        LiftedValue::Variable(value) => remap(value).map(Value::Variable),
        LiftedValue::Constant(value) => Ok(Value::Constant(value)),
    };
    Ok(match condition {
        LiftedCondition::Equal(a, b) => Predicate::Equal(value(a)?, value(b)?),
        LiftedCondition::NotEqual(a, b) => Predicate::NotEqual(value(a)?, value(b)?),
        LiftedCondition::LessThan(a, b) => Predicate::LessThan(value(a)?, value(b)?),
        LiftedCondition::LessThanOrEqual(a, b) => Predicate::LessThanOrEqual(value(a)?, value(b)?),
        LiftedCondition::GreaterThan(a, b) => Predicate::GreaterThan(value(a)?, value(b)?),
        LiftedCondition::GreaterThanOrEqual(a, b) => {
            Predicate::GreaterThanOrEqual(value(a)?, value(b)?)
        }
        LiftedCondition::IsNull(a) => Predicate::IsNull(value(a)?),
        LiftedCondition::IsNotNull(a) => Predicate::IsNotNull(value(a)?),
        LiftedCondition::IsZero(a) => Predicate::IsZero(value(a)?),
        LiftedCondition::IsNonZero(a) => Predicate::IsNonZero(value(a)?),
        LiftedCondition::IsPositive(a) => Predicate::IsPositive(value(a)?),
        LiftedCondition::IsNegative(a) => Predicate::IsNegative(value(a)?),
        LiftedCondition::IsNonNegative(a) => Predicate::IsNonNegative(value(a)?),
        LiftedCondition::IsNonPositive(a) => Predicate::IsNonPositive(value(a)?),
    })
}
