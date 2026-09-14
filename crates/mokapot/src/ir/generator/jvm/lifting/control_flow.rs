use crate::ir::{
    expression::Condition,
    generator::{
        error::MokaIRBuildError,
        jvm::{
            frame::CATEGORY_1, instruction::RegisterInstruction, lifting::LiftContext,
            symbolic_execution::Value,
        },
    },
};
use crate::jvm::code::ProgramCounter;

impl LiftContext<'_, '_, '_> {
    pub(super) fn unary_branch(
        &mut self,
        target: ProgramCounter,
        condition: impl FnOnce(Value) -> Condition<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let operand = self.frame.pop_value::<CATEGORY_1>()?;
        Ok(RegisterInstruction::Jump {
            condition: Some(condition(operand)),
            target,
        })
    }

    pub(super) fn comparison_branch(
        &mut self,
        target: ProgramCounter,
        condition: impl FnOnce(Value, Value) -> Condition<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let rhs = self.frame.pop_value::<CATEGORY_1>()?;
        let lhs = self.frame.pop_value::<CATEGORY_1>()?;
        Ok(RegisterInstruction::Jump {
            condition: Some(condition(lhs, rhs)),
            target,
        })
    }

    pub(super) fn switch(
        &mut self,
        default: ProgramCounter,
        branches: BTreeMap<i32, ProgramCounter>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        Ok(RegisterInstruction::Switch {
            match_value: self.frame.pop_value::<CATEGORY_1>()?,
            branches,
            default,
        })
    }

    pub(super) fn return_value<const SLOT: bool>(
        &mut self,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        Ok(RegisterInstruction::Return(Some(
            self.frame.pop_value::<SLOT>()?,
        )))
    }

    pub(super) fn throw(&mut self) -> Result<RegisterInstruction, MokaIRBuildError> {
        Ok(RegisterInstruction::Throw(
            self.frame.pop_value::<CATEGORY_1>()?,
        ))
    }

    pub(super) fn subroutine_return(
        &self,
        idx: u16,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        Ok(RegisterInstruction::SubroutineReturn(
            self.frame.get_local::<CATEGORY_1>(idx)?,
        ))
    }

    pub(super) fn subroutine_call(
        &mut self,
        target: ProgramCounter,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let next_pc = self.executor.next_program_counter(self.pc)?;
        let (target, return_address) =
            self.executor
                .enter_subroutine(self.location, target, next_pc)?;
        self.frame.push_value::<CATEGORY_1>(return_address.into())?;
        Ok(RegisterInstruction::Subroutine { target })
    }
}
use std::collections::BTreeMap;
