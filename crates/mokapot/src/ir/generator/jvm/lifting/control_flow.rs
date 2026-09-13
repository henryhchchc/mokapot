use crate::{
    ir::{
        expression::Condition,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT},
                instruction::RegisterInstruction,
                normalization::Location,
                symbolic_execution::{JvmSymbolicExecutor, SymbolicValue},
            },
        },
    },
    jvm::code::{Instruction as JVM, ProgramCounter, WideInstruction},
};

#[inline]
pub(super) fn unary_branch(
    frame: &mut JvmStackFrame<SymbolicValue>,
    target: ProgramCounter,
    condition: impl FnOnce(SymbolicValue) -> Condition<SymbolicValue>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let operand = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(RegisterInstruction::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
pub(super) fn comparison_branch(
    frame: &mut JvmStackFrame<SymbolicValue>,
    target: ProgramCounter,
    condition: impl FnOnce(SymbolicValue, SymbolicValue) -> Condition<SymbolicValue>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let rhs = frame.pop_value::<SINGLE_SLOT>()?;
    let lhs = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(RegisterInstruction::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}

pub(super) fn try_lift(
    executor: &mut JvmSymbolicExecutor<'_>,
    jvm_instruction: &JVM,
    location: Location,
    pc: ProgramCounter,
    frame: &mut JvmStackFrame<SymbolicValue>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use JVM::*;

    let instruction = match jvm_instruction {
        IfEq(target) => unary_branch(frame, *target, Condition::IsZero)?,
        IfNe(target) => unary_branch(frame, *target, Condition::IsNonZero)?,
        IfLt(target) => unary_branch(frame, *target, Condition::IsNegative)?,
        IfGe(target) => unary_branch(frame, *target, Condition::IsNonNegative)?,
        IfGt(target) => unary_branch(frame, *target, Condition::IsPositive)?,
        IfLe(target) => unary_branch(frame, *target, Condition::IsNonPositive)?,
        IfNull(target) => unary_branch(frame, *target, Condition::IsNull)?,
        IfNonNull(target) => unary_branch(frame, *target, Condition::IsNotNull)?,
        IfICmpEq(target) | IfACmpEq(target) => comparison_branch(frame, *target, Condition::Equal)?,
        IfICmpNe(target) | IfACmpNe(target) => {
            comparison_branch(frame, *target, Condition::NotEqual)?
        }
        IfICmpGe(target) => comparison_branch(frame, *target, Condition::GreaterThanOrEqual)?,
        IfICmpLt(target) => comparison_branch(frame, *target, Condition::LessThan)?,
        IfICmpGt(target) => comparison_branch(frame, *target, Condition::GreaterThan)?,
        IfICmpLe(target) => comparison_branch(frame, *target, Condition::LessThanOrEqual)?,
        Goto(target) | GotoW(target) => RegisterInstruction::Jump {
            condition: None,
            target: *target,
        },
        Jsr(target) | JsrW(target) => {
            let next_pc = executor.next_pc_of(pc)?;
            let (target, return_address) = executor.enter_subroutine(location, *target, next_pc)?;
            frame.push_value::<SINGLE_SLOT>(return_address.into())?;
            RegisterInstruction::Subroutine { target }
        }
        Ret(idx) => {
            let idx = (*idx).into();
            let return_address = frame.get_local::<SINGLE_SLOT>(idx)?;
            RegisterInstruction::SubroutineReturn(return_address)
        }
        Wide(WideInstruction::Ret(idx)) => {
            let return_address = frame.get_local::<SINGLE_SLOT>(*idx)?;
            RegisterInstruction::SubroutineReturn(return_address)
        }
        TableSwitch {
            range,
            jump_targets,
            default,
        } => {
            let condition = frame.pop_value::<SINGLE_SLOT>()?;
            let branches = range.clone().zip(jump_targets.clone()).collect();
            RegisterInstruction::Switch {
                match_value: condition,
                default: *default,
                branches,
            }
        }
        LookupSwitch {
            default,
            match_targets,
        } => {
            let condition = frame.pop_value::<SINGLE_SLOT>()?;
            RegisterInstruction::Switch {
                match_value: condition,
                default: *default,
                branches: match_targets.clone(),
            }
        }
        IReturn | FReturn | AReturn => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            RegisterInstruction::Return(Some(value))
        }
        LReturn | DReturn => {
            let value = frame.pop_value::<DUAL_SLOT>()?;
            RegisterInstruction::Return(Some(value))
        }
        Return => RegisterInstruction::Return(None),
        AThrow => {
            let exception_ref = frame.pop_value::<SINGLE_SLOT>()?;
            RegisterInstruction::Throw(exception_ref)
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
