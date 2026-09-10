#[allow(
    clippy::wildcard_imports,
    reason = "opcode-family lifters share the private lifting vocabulary"
)]
use super::*;

#[inline]
pub(super) fn conditional_jump<OP: Clone + std::fmt::Display>(
    frame: &mut JvmStackFrame<OP>,
    target: ProgramCounter,
    condition: impl FnOnce(OP) -> Condition<OP>,
) -> Result<IR<OP>, MokaIRBuildError> {
    let operand = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
pub(super) fn cmp_jump<OP: Clone + std::fmt::Display>(
    frame: &mut JvmStackFrame<OP>,
    target: ProgramCounter,
    condition: impl FnOnce(OP, OP) -> Condition<OP>,
) -> Result<IR<OP>, MokaIRBuildError> {
    let rhs = frame.pop_value::<SINGLE_SLOT>()?;
    let lhs = frame.pop_value::<SINGLE_SLOT>()?;
    Ok(IR::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}

pub(super) fn lift<OP: FrameOperand>(
    generator: &mut JvmFrameAnalysis<'_>,
    jvm_instruction: &Instruction,
    location: Location,
    pc: ProgramCounter,
    frame: &mut JvmStackFrame<OP>,
) -> Result<Option<IR<OP>>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use Instruction::*;

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
        Goto(target) | GotoW(target) => IR::Jump {
            condition: None,
            target: *target,
        },
        Jsr(target) | JsrW(target) => {
            let next_pc = generator.next_pc_of(pc)?;
            let (target, return_address) = generator.legacy.enter(location, *target, next_pc)?;
            frame.push_value::<SINGLE_SLOT>(return_address.into())?;
            IR::Subroutine { target }
        }
        Ret(idx) => {
            let idx = (*idx).into();
            let return_address = frame.get_local::<SINGLE_SLOT>(idx)?;
            IR::SubroutineReturn(return_address)
        }
        Wide(WideInstruction::Ret(idx)) => {
            let return_address = frame.get_local::<SINGLE_SLOT>(*idx)?;
            IR::SubroutineReturn(return_address)
        }
        TableSwitch {
            range,
            jump_targets,
            default,
        } => {
            let condition = frame.pop_value::<SINGLE_SLOT>()?;
            let branches = range.clone().zip(jump_targets.clone()).collect();
            IR::Switch {
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
            IR::Switch {
                match_value: condition,
                default: *default,
                branches: match_targets.clone(),
            }
        }
        IReturn | FReturn | AReturn => {
            let value = frame.pop_value::<SINGLE_SLOT>()?;
            IR::Return(Some(value))
        }
        LReturn | DReturn => {
            let value = frame.pop_value::<DUAL_SLOT>()?;
            IR::Return(Some(value))
        }
        Return => IR::Return(None),
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
