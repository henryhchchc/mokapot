use crate::{
    ir::{
        expression::Condition,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                subroutine_expansion::Location,
                symbolic_execution::{Executor, Value},
            },
        },
    },
    jvm::code::{Instruction as JVM, ProgramCounter, WideInstruction},
};

#[inline]
pub(super) fn unary_branch(
    frame: &mut Frame<Value>,
    target: ProgramCounter,
    condition: impl FnOnce(Value) -> Condition<Value>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let operand = frame.pop_value::<CATEGORY_1>()?;
    Ok(RegisterInstruction::Jump {
        condition: Some(condition(operand)),
        target,
    })
}

#[inline]
pub(super) fn comparison_branch(
    frame: &mut Frame<Value>,
    target: ProgramCounter,
    condition: impl FnOnce(Value, Value) -> Condition<Value>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let rhs = frame.pop_value::<CATEGORY_1>()?;
    let lhs = frame.pop_value::<CATEGORY_1>()?;
    Ok(RegisterInstruction::Jump {
        condition: Some(condition(lhs, rhs)),
        target,
    })
}

impl Executor<'_> {
    pub(super) fn try_lift_control_flow(
        &mut self,
        jvm_instruction: &JVM,
        location: Location,
        pc: ProgramCounter,
        frame: &mut Frame<Value>,
    ) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
        use JVM::{
            AReturn, AThrow, DReturn, FReturn, Goto, GotoW, IReturn, IfACmpEq, IfACmpNe, IfEq,
            IfGe, IfGt, IfICmpEq, IfICmpGe, IfICmpGt, IfICmpLe, IfICmpLt, IfICmpNe, IfLe, IfLt,
            IfNe, IfNonNull, IfNull, Jsr, JsrW, LReturn, LookupSwitch, Ret, Return, TableSwitch,
            Wide,
        };

        let instruction = match jvm_instruction {
            IfEq(target) => unary_branch(frame, *target, Condition::IsZero)?,
            IfNe(target) => unary_branch(frame, *target, Condition::IsNonZero)?,
            IfLt(target) => unary_branch(frame, *target, Condition::IsNegative)?,
            IfGe(target) => unary_branch(frame, *target, Condition::IsNonNegative)?,
            IfGt(target) => unary_branch(frame, *target, Condition::IsPositive)?,
            IfLe(target) => unary_branch(frame, *target, Condition::IsNonPositive)?,
            IfNull(target) => unary_branch(frame, *target, Condition::IsNull)?,
            IfNonNull(target) => unary_branch(frame, *target, Condition::IsNotNull)?,
            IfICmpEq(target) | IfACmpEq(target) => {
                comparison_branch(frame, *target, Condition::Equal)?
            }
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
                let next_pc = self.next_program_counter(pc)?;
                let (target, return_address) = self.enter_subroutine(location, *target, next_pc)?;
                frame.push_value::<CATEGORY_1>(return_address.into())?;
                RegisterInstruction::Subroutine { target }
            }
            Ret(idx) => {
                let local_index = (*idx).into();
                let return_address = frame.get_local::<CATEGORY_1>(local_index)?;
                RegisterInstruction::SubroutineReturn(return_address)
            }
            Wide(WideInstruction::Ret(idx)) => {
                let return_address = frame.get_local::<CATEGORY_1>(*idx)?;
                RegisterInstruction::SubroutineReturn(return_address)
            }
            TableSwitch {
                range,
                jump_targets,
                default,
            } => RegisterInstruction::Switch {
                match_value: frame.pop_value::<CATEGORY_1>()?,
                default: *default,
                branches: range.clone().zip(jump_targets.clone()).collect(),
            },
            LookupSwitch {
                default,
                match_targets,
            } => RegisterInstruction::Switch {
                match_value: frame.pop_value::<CATEGORY_1>()?,
                default: *default,
                branches: match_targets.clone(),
            },
            IReturn | FReturn | AReturn => {
                let value = frame.pop_value::<CATEGORY_1>()?;
                RegisterInstruction::Return(Some(value))
            }
            LReturn | DReturn => {
                let value = frame.pop_value::<CATEGORY_2>()?;
                RegisterInstruction::Return(Some(value))
            }
            Return => RegisterInstruction::Return(None),
            AThrow => {
                let exception_ref = frame.pop_value::<CATEGORY_1>()?;
                RegisterInstruction::Throw(exception_ref)
            }
            _ => return Ok(None),
        };
        Ok(Some(instruction))
    }
}
