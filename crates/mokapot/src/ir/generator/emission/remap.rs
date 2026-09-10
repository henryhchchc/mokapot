use super::super::MokaIRBuildError;
use crate::ir::{
    ValueId,
    control_flow::ControlTransfer,
    control_flow::path_condition::{BooleanVariable, BranchGuard, Value},
    expression::{
        ArrayOperation, Condition, Conversion, Expression, FieldAccess, LockOperation,
        MathOperation, Predicate,
    },
};

pub(super) fn remap_expression<OP>(
    expression: Expression<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<Expression, MokaIRBuildError> {
    Ok(match expression {
        Expression::Const(value) => Expression::Const(value),
        Expression::Call { method, this, args } => Expression::Call {
            method,
            this: this.map(remap).transpose()?,
            args: args.into_iter().map(remap).collect::<Result<_, _>>()?,
        },
        Expression::Closure {
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
        Expression::Math(operation) => Expression::Math(remap_math(operation, remap)?),
        Expression::Field(access) => Expression::Field(remap_field(access, remap)?),
        Expression::Array(operation) => Expression::Array(remap_array(operation, remap)?),
        Expression::Conversion(operation) => {
            Expression::Conversion(remap_conversion(operation, remap)?)
        }
        Expression::Synchronization(operation) => {
            Expression::Synchronization(remap_lock(operation, remap)?)
        }
        Expression::New(class) => Expression::New(class),
    })
}

fn remap_math<OP>(
    operation: MathOperation<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<MathOperation, MokaIRBuildError> {
    Ok(match operation {
        MathOperation::Add(a, b) => MathOperation::Add(remap(a)?, remap(b)?),
        MathOperation::Subtract(a, b) => MathOperation::Subtract(remap(a)?, remap(b)?),
        MathOperation::Multiply(a, b) => MathOperation::Multiply(remap(a)?, remap(b)?),
        MathOperation::Divide(a, b) => MathOperation::Divide(remap(a)?, remap(b)?),
        MathOperation::Remainder(a, b) => MathOperation::Remainder(remap(a)?, remap(b)?),
        MathOperation::Negate(value) => MathOperation::Negate(remap(value)?),
        MathOperation::Increment(value, amount) => MathOperation::Increment(remap(value)?, amount),
        MathOperation::ShiftLeft(a, b) => MathOperation::ShiftLeft(remap(a)?, remap(b)?),
        MathOperation::ShiftRight(a, b) => MathOperation::ShiftRight(remap(a)?, remap(b)?),
        MathOperation::LogicalShiftRight(a, b) => {
            MathOperation::LogicalShiftRight(remap(a)?, remap(b)?)
        }
        MathOperation::BitwiseAnd(a, b) => MathOperation::BitwiseAnd(remap(a)?, remap(b)?),
        MathOperation::BitwiseOr(a, b) => MathOperation::BitwiseOr(remap(a)?, remap(b)?),
        MathOperation::BitwiseXor(a, b) => MathOperation::BitwiseXor(remap(a)?, remap(b)?),
        MathOperation::LongComparison(a, b) => MathOperation::LongComparison(remap(a)?, remap(b)?),
        MathOperation::FloatingPointComparison(a, b, nan) => {
            MathOperation::FloatingPointComparison(remap(a)?, remap(b)?, nan)
        }
    })
}

fn remap_array<OP>(
    operation: ArrayOperation<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<ArrayOperation, MokaIRBuildError> {
    Ok(match operation {
        ArrayOperation::New {
            element_type,
            length,
        } => ArrayOperation::New {
            element_type,
            length: remap(length)?,
        },
        ArrayOperation::NewMultiDim {
            element_type,
            dimensions,
        } => ArrayOperation::NewMultiDim {
            element_type,
            dimensions: dimensions
                .into_iter()
                .map(remap)
                .collect::<Result<_, _>>()?,
        },
        ArrayOperation::Read { array_ref, index } => ArrayOperation::Read {
            array_ref: remap(array_ref)?,
            index: remap(index)?,
        },
        ArrayOperation::Write {
            array_ref,
            index,
            value,
        } => ArrayOperation::Write {
            array_ref: remap(array_ref)?,
            index: remap(index)?,
            value: remap(value)?,
        },
        ArrayOperation::Length { array_ref } => ArrayOperation::Length {
            array_ref: remap(array_ref)?,
        },
    })
}

fn remap_field<OP>(
    access: FieldAccess<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<FieldAccess, MokaIRBuildError> {
    Ok(match access {
        FieldAccess::ReadStatic { field } => FieldAccess::ReadStatic { field },
        FieldAccess::WriteStatic { field, value } => FieldAccess::WriteStatic {
            field,
            value: remap(value)?,
        },
        FieldAccess::ReadInstance { object_ref, field } => FieldAccess::ReadInstance {
            object_ref: remap(object_ref)?,
            field,
        },
        FieldAccess::WriteInstance {
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

fn remap_conversion<OP>(
    operation: Conversion<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<Conversion, MokaIRBuildError> {
    Ok(match operation {
        Conversion::Int2Long(value) => Conversion::Int2Long(remap(value)?),
        Conversion::Int2Float(value) => Conversion::Int2Float(remap(value)?),
        Conversion::Int2Double(value) => Conversion::Int2Double(remap(value)?),
        Conversion::Long2Int(value) => Conversion::Long2Int(remap(value)?),
        Conversion::Long2Float(value) => Conversion::Long2Float(remap(value)?),
        Conversion::Long2Double(value) => Conversion::Long2Double(remap(value)?),
        Conversion::Float2Int(value) => Conversion::Float2Int(remap(value)?),
        Conversion::Float2Long(value) => Conversion::Float2Long(remap(value)?),
        Conversion::Float2Double(value) => Conversion::Float2Double(remap(value)?),
        Conversion::Double2Int(value) => Conversion::Double2Int(remap(value)?),
        Conversion::Double2Long(value) => Conversion::Double2Long(remap(value)?),
        Conversion::Double2Float(value) => Conversion::Double2Float(remap(value)?),
        Conversion::Int2Byte(value) => Conversion::Int2Byte(remap(value)?),
        Conversion::Int2Char(value) => Conversion::Int2Char(remap(value)?),
        Conversion::Int2Short(value) => Conversion::Int2Short(remap(value)?),
        Conversion::CheckCast(value, target) => Conversion::CheckCast(remap(value)?, target),
        Conversion::InstanceOf(value, target) => Conversion::InstanceOf(remap(value)?, target),
    })
}

fn remap_lock<OP>(
    operation: LockOperation<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<LockOperation, MokaIRBuildError> {
    Ok(match operation {
        LockOperation::Acquire(value) => LockOperation::Acquire(remap(value)?),
        LockOperation::Release(value) => LockOperation::Release(remap(value)?),
    })
}

pub(super) fn remap_transfer<OP: Eq + std::hash::Hash>(
    transfer: ControlTransfer<OP>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<ControlTransfer, MokaIRBuildError> {
    Ok(match transfer {
        ControlTransfer::Unconditional => ControlTransfer::Unconditional,
        ControlTransfer::Normal => ControlTransfer::Normal,
        ControlTransfer::Exception(types) => ControlTransfer::Exception(types),
        ControlTransfer::Unwind => ControlTransfer::Unwind,
        ControlTransfer::Conditional(guard) => ControlTransfer::Conditional(
            guard
                .into_iter()
                .map(|literal| remap_literal(literal, remap))
                .collect::<Result<BranchGuard<_>, _>>()?,
        ),
    })
}

fn remap_literal<OP>(
    literal: BooleanVariable<Condition<Value<OP>>>,
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

fn remap_guard_condition<OP>(
    condition: Condition<Value<OP>>,
    remap: &impl Fn(OP) -> Result<ValueId, MokaIRBuildError>,
) -> Result<Predicate, MokaIRBuildError> {
    let value = |value| match value {
        Value::Variable(value) => remap(value).map(Value::Variable),
        Value::Constant(value) => Ok(Value::Constant(value)),
    };
    Ok(match condition {
        Condition::Equal(a, b) => Predicate::Equal(value(a)?, value(b)?),
        Condition::NotEqual(a, b) => Predicate::NotEqual(value(a)?, value(b)?),
        Condition::LessThan(a, b) => Predicate::LessThan(value(a)?, value(b)?),
        Condition::LessThanOrEqual(a, b) => Predicate::LessThanOrEqual(value(a)?, value(b)?),
        Condition::GreaterThan(a, b) => Predicate::GreaterThan(value(a)?, value(b)?),
        Condition::GreaterThanOrEqual(a, b) => Predicate::GreaterThanOrEqual(value(a)?, value(b)?),
        Condition::IsNull(a) => Predicate::IsNull(value(a)?),
        Condition::IsNotNull(a) => Predicate::IsNotNull(value(a)?),
        Condition::IsZero(a) => Predicate::IsZero(value(a)?),
        Condition::IsNonZero(a) => Predicate::IsNonZero(value(a)?),
        Condition::IsPositive(a) => Predicate::IsPositive(value(a)?),
        Condition::IsNegative(a) => Predicate::IsNegative(value(a)?),
        Condition::IsNonNegative(a) => Predicate::IsNonNegative(value(a)?),
        Condition::IsNonPositive(a) => Predicate::IsNonPositive(value(a)?),
    })
}
