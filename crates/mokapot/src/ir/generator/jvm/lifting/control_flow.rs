use crate::{
    ir::{
        expression::Condition,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                analysis::{JvmFrameAnalyzer, OperandState},
                frame::{DUAL_SLOT, JvmStackFrame, SINGLE_SLOT},
                instruction::Instruction,
                normalization::Location,
            },
        },
    },
    jvm::code::{Instruction as JVM, ProgramCounter, WideInstruction},
};

#[inline]
pub(super) fn conditional_jump(
    frame: &mut JvmStackFrame<OperandState>,
    target: ProgramCounter,
    condition: impl FnOnce(OperandState) -> Condition<OperandState>,
) -> Result<Instruction, MokaIRBuildError> {
    let operand = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(Instruction::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
pub(super) fn cmp_jump(
    frame: &mut JvmStackFrame<OperandState>,
    target: ProgramCounter,
    condition: impl FnOnce(OperandState, OperandState) -> Condition<OperandState>,
) -> Result<Instruction, MokaIRBuildError> {
    let rhs = frame.pop_value::<SINGLE_SLOT>()?;
    let lhs = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(Instruction::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}

pub(super) fn lift(
    semantics: &mut JvmFrameAnalyzer<'_>,
    jvm_instruction: &JVM,
    location: Location,
    pc: ProgramCounter,
    frame: &mut JvmStackFrame<OperandState>,
) -> Result<Option<Instruction>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use JVM::*;

    let instruction = match jvm_instruction {
        IfEq(target) => conditional_jump(frame, *target, Condition::IsZero)?,
        IfNe(target) => conditional_jump(frame, *target, Condition::IsNonZero)?,
        IfLt(target) => conditional_jump(frame, *target, Condition::IsNegative)?,
        IfGe(target) => conditional_jump(frame, *target, Condition::IsNonNegative)?,
        IfGt(target) => conditional_jump(frame, *target, Condition::IsPositive)?,
        IfLe(target) => conditional_jump(frame, *target, Condition::IsNonPositive)?,
        IfNull(target) => conditional_jump(frame, *target, Condition::IsNull)?,
        IfNonNull(target) => conditional_jump(frame, *target, Condition::IsNotNull)?,
        IfICmpEq(target) | IfACmpEq(target) => cmp_jump(frame, *target, Condition::Equal)?,
        IfICmpNe(target) | IfACmpNe(target) => cmp_jump(frame, *target, Condition::NotEqual)?,
        IfICmpGe(target) => cmp_jump(frame, *target, Condition::GreaterThanOrEqual)?,
        IfICmpLt(target) => cmp_jump(frame, *target, Condition::LessThan)?,
        IfICmpGt(target) => cmp_jump(frame, *target, Condition::GreaterThan)?,
        IfICmpLe(target) => cmp_jump(frame, *target, Condition::LessThanOrEqual)?,
        Goto(target) | GotoW(target) => Instruction::Jump {
            condition: None,
            target: *target,
        },
        Jsr(target) | JsrW(target) => {
            let next_pc = semantics.next_pc_of(pc)?;
            let (target, return_address) =
                semantics.enter_subroutine(location, *target, next_pc)?;
            frame.push_value::<SINGLE_SLOT>(return_address.into())?;
            Instruction::Subroutine { target }
        }
        Ret(idx) => {
            let idx = (*idx).into();
            let return_address = frame.get_local::<SINGLE_SLOT>(idx)?;
            Instruction::SubroutineReturn(return_address)
        }
        Wide(WideInstruction::Ret(idx)) => {
            let return_address = frame.get_local::<SINGLE_SLOT>(*idx)?;
            Instruction::SubroutineReturn(return_address)
        }
        TableSwitch {
            range,
            jump_targets,
            default,
        } => {
            let condition = frame.pop_value::<SINGLE_SLOT>()?;
            let branches = range.clone().zip(jump_targets.clone()).collect();
            Instruction::Switch {
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
            Instruction::Switch {
                match_value: condition,
                default: *default,
                branches: match_targets.clone(),
            }
        }
        IReturn | FReturn | AReturn => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            Instruction::Return(Some(value))
        }
        LReturn | DReturn => {
            let value = frame.pop_value::<DUAL_SLOT>()?;
            Instruction::Return(Some(value))
        }
        Return => Instruction::Return(None),
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
