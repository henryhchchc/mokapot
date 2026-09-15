use crate::ir::{
    expression::Condition,
    generator::{
        error::MokaIRBuildError,
        jvm::{
            frame::{ValueCategory, ValueCategory::Category1},
            instruction::RegisterInstruction,
            lifting::LiftContext,
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
        let operand = self.frame.stack.pop(Category1)?;
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
        let rhs = self.frame.stack.pop(Category1)?;
        let lhs = self.frame.stack.pop(Category1)?;
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
            match_value: self.frame.stack.pop(Category1)?,
            branches,
            default,
        })
    }

    pub(super) fn return_value(
        &mut self,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        Ok(RegisterInstruction::Return(Some(
            self.frame.stack.pop(category)?,
        )))
    }

    pub(super) fn throw(&mut self) -> Result<RegisterInstruction, MokaIRBuildError> {
        Ok(RegisterInstruction::Throw(self.frame.stack.pop(Category1)?))
    }

    pub(super) fn subroutine_return(
        &self,
        idx: u16,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        Ok(RegisterInstruction::SubroutineReturn(
            *self.frame.locals.get(idx, Category1)?,
        ))
    }

    pub(super) fn subroutine_call(
        &mut self,
        target: ProgramCounter,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let (target, return_address) = self.executor.enter_subroutine(self.addr, target)?;
        self.frame.stack.push(return_address.into(), Category1)?;
        Ok(RegisterInstruction::Subroutine { target })
    }
}
use std::collections::BTreeMap;
