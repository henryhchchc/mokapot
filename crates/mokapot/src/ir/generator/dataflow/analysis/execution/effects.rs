//! Dataflow effects of a structural block's final instruction.

use std::collections::BTreeMap;

use super::super::super::{
    Frame,
    ValueCategory::{Category1, Category2},
};
use crate::{
    ir::{
        BlockId, ValueId,
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, PathValue},
        },
        expression::Predicate,
        generator::error::Error,
    },
    jvm::{ConstantValue, code::Instruction},
};

/// Pops the compared operands and returns the transfers selected when the branch
/// is taken and when it is not.
pub(super) fn branch_transfers(
    instruction: &Instruction,
    frame: &mut Frame,
) -> Result<(ControlTransfer, ControlTransfer), Error> {
    let condition: BooleanVariable<Predicate> = pop_condition(frame, instruction)?.into();
    Ok((
        ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
        ControlTransfer::Conditional(BranchGuard::of(!condition)),
    ))
}

/// Pops the switch selector.
///
/// The selector is popped even when `cases` is empty, so a caller executing an
/// empty switch to a goto still pops it.
pub(super) fn switch_selector(frame: &mut Frame) -> Result<ValueId, Error> {
    Ok(frame.stack.pop(Category1)?)
}

/// The transfer selected when `selector` matches `case`.
pub(super) fn case_guard(selector: ValueId, case: i32) -> ControlTransfer {
    ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(equal(
        selector, case,
    ))))
}

/// The transfer covering every value `selector` can hold when no case matches.
pub(super) fn default_guard(selector: ValueId, cases: &BTreeMap<i32, BlockId>) -> ControlTransfer {
    let branches = cases
        .keys()
        .map(|&case| BooleanVariable::Negative(equal(selector, case)))
        .collect();
    ControlTransfer::Conditional(branches)
}

/// Predicate matching `selector` against the constant `case`.
fn equal(selector: ValueId, case: i32) -> Predicate {
    Predicate::Equal(
        selector.into(),
        PathValue::Constant(ConstantValue::Integer(case)),
    )
}

/// Pops the operand returned by a return instruction, if any.
pub(super) fn return_operand(
    instruction: &Instruction,
    frame: &mut Frame,
) -> Result<Option<ValueId>, Error> {
    Ok(match instruction {
        Instruction::Return => None,
        Instruction::IReturn | Instruction::FReturn | Instruction::AReturn => {
            Some(frame.stack.pop(Category1)?)
        }
        Instruction::LReturn | Instruction::DReturn => Some(frame.stack.pop(Category2)?),
        _ => panic!("return block ends in a non-return instruction"),
    })
}

/// Pops the operand thrown by a throw instruction.
pub(super) fn throw_operand(
    instruction: &Instruction,
    frame: &mut Frame,
) -> Result<ValueId, Error> {
    match instruction {
        Instruction::AThrow => Ok(frame.stack.pop(Category1)?),
        _ => panic!("throw block ends in a non-throw instruction"),
    }
}

fn pop_condition(frame: &mut Frame, instruction: &Instruction) -> Result<Predicate, Error> {
    let unary = |frame: &mut Frame| frame.stack.pop(Category1).map(Into::into);
    let binary = |frame: &mut Frame| {
        let rhs = frame.stack.pop(Category1)?.into();
        let lhs = frame.stack.pop(Category1)?.into();
        Ok::<_, Error>((lhs, rhs))
    };
    Ok(match instruction {
        Instruction::IfEq(_) => Predicate::IsZero(unary(frame)?),
        Instruction::IfNe(_) => Predicate::IsNonZero(unary(frame)?),
        Instruction::IfLt(_) => Predicate::IsNegative(unary(frame)?),
        Instruction::IfGe(_) => Predicate::IsNonNegative(unary(frame)?),
        Instruction::IfGt(_) => Predicate::IsPositive(unary(frame)?),
        Instruction::IfLe(_) => Predicate::IsNonPositive(unary(frame)?),
        Instruction::IfNull(_) => Predicate::IsNull(unary(frame)?),
        Instruction::IfNonNull(_) => Predicate::IsNotNull(unary(frame)?),
        Instruction::IfICmpEq(_) | Instruction::IfACmpEq(_) => {
            let (lhs, rhs) = binary(frame)?;
            Predicate::Equal(lhs, rhs)
        }
        Instruction::IfICmpNe(_) | Instruction::IfACmpNe(_) => {
            let (lhs, rhs) = binary(frame)?;
            Predicate::NotEqual(lhs, rhs)
        }
        Instruction::IfICmpLt(_) => {
            let (lhs, rhs) = binary(frame)?;
            Predicate::LessThan(lhs, rhs)
        }
        Instruction::IfICmpGe(_) => {
            let (lhs, rhs) = binary(frame)?;
            Predicate::GreaterThanOrEqual(lhs, rhs)
        }
        Instruction::IfICmpGt(_) => {
            let (lhs, rhs) = binary(frame)?;
            Predicate::GreaterThan(lhs, rhs)
        }
        Instruction::IfICmpLe(_) => {
            let (lhs, rhs) = binary(frame)?;
            Predicate::LessThanOrEqual(lhs, rhs)
        }
        _ => panic!("branch block ends in a non-branch instruction"),
    })
}
