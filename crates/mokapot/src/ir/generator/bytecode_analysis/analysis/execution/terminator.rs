//! Stack effects and analyzed successors derived from structural terminators.

use std::collections::BTreeMap;

use super::super::{
    Frame,
    ValueCategory::{Category1, Category2},
};
use crate::ir::{
    ValueId,
    control_flow::{
        ControlTransfer,
        path_condition::{BooleanVariable, BranchGuard, PathValue},
    },
    expression::Predicate,
    generator::{
        bytecode_cfg::{self, BlockExit},
        error::Error,
    },
};
use crate::jvm::code::Instruction;

pub(super) enum LoweredTerminator {
    Goto(ControlTransfer),
    Branch {
        taken: ControlTransfer,
        otherwise: ControlTransfer,
    },
    Switch {
        cases: Vec<ControlTransfer>,
        default: ControlTransfer,
    },
    Try(ControlTransfer),
    Return(Option<ValueId>),
    Throw(ValueId),
}

/// Lowers the structural terminator of `block`, appending its successors.
pub(super) fn lower(
    block: &bytecode_cfg::JvmBlock,
    instruction: &Instruction,
    frame: &mut Frame,
) -> Result<LoweredTerminator, Error> {
    let result = match &block.exit {
        BlockExit::Fallthrough { .. } => {
            if block.exception_handlers.is_empty() {
                LoweredTerminator::Goto(ControlTransfer::Unconditional)
            } else {
                LoweredTerminator::Try(ControlTransfer::Unconditional)
            }
        }
        BlockExit::Goto { .. } => LoweredTerminator::Goto(ControlTransfer::Unconditional),
        BlockExit::Branch { .. } => lower_branch(instruction, frame)?,
        BlockExit::Switch { cases, .. } => lower_switch(cases, frame)?,
        BlockExit::Terminal => lower_terminal(instruction, frame)?,
    };
    Ok(result)
}

fn lower_branch(instruction: &Instruction, frame: &mut Frame) -> Result<LoweredTerminator, Error> {
    let condition: BooleanVariable<_> = pop_condition(frame, instruction)?.into();
    Ok(LoweredTerminator::Branch {
        taken: ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
        otherwise: ControlTransfer::Conditional(BranchGuard::of(!condition)),
    })
}

fn lower_switch(
    cases: &BTreeMap<i32, bytecode_cfg::JvmBlockId>,
    frame: &mut Frame,
) -> Result<LoweredTerminator, Error> {
    let match_value = frame.stack.pop(Category1)?;
    if cases.is_empty() {
        return Ok(LoweredTerminator::Goto(ControlTransfer::Unconditional));
    }
    let successors = cases
        .keys()
        .map(|&case| {
            ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
                Predicate::Equal(
                    match_value.into(),
                    PathValue::Constant(crate::jvm::ConstantValue::Integer(case)),
                ),
            )))
        })
        .collect::<Vec<_>>();
    let default_guard = cases
        .keys()
        .map(|case| {
            BooleanVariable::Negative(Predicate::Equal(
                match_value.into(),
                PathValue::Constant(crate::jvm::ConstantValue::Integer(*case)),
            ))
        })
        .collect();
    Ok(LoweredTerminator::Switch {
        cases: successors,
        default: ControlTransfer::Conditional(default_guard),
    })
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
        _ => {
            return Err(Error::internal(
                "a branch block ends in a non-branch instruction",
            ));
        }
    })
}

fn lower_terminal(
    instruction: &Instruction,
    frame: &mut Frame,
) -> Result<LoweredTerminator, Error> {
    let kind = match instruction {
        Instruction::Return => LoweredTerminator::Return(None),
        Instruction::IReturn | Instruction::FReturn | Instruction::AReturn => {
            LoweredTerminator::Return(Some(frame.stack.pop(Category1)?))
        }
        Instruction::LReturn | Instruction::DReturn => {
            LoweredTerminator::Return(Some(frame.stack.pop(Category2)?))
        }
        Instruction::AThrow => LoweredTerminator::Throw(frame.stack.pop(Category1)?),
        _ => {
            return Err(Error::internal(
                "a terminal block ends in a non-terminal instruction",
            ));
        }
    };
    Ok(kind)
}
