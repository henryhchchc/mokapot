//! Interpretation of a block's final instruction into its terminator.

use std::collections::BTreeMap;

use super::{Frame, FrameArm, FrameTerminator, ValueContext, effects, lifting};
use crate::{
    ir::{
        BlockId, ControlTransfer, Operation,
        generator::{
            control_flow::{ArmKey, BlockExit, ExceptionArm, ExceptionTarget},
            data_flow::FrameError,
        },
    },
    jvm::code::{Instruction, ProgramCounter},
};

/// The frame and operations reached at a block's final instruction.
pub(super) struct ExitState<'instruction> {
    instruction: &'instruction Instruction,
    pc: ProgramCounter,
    /// The frame before the final instruction runs; handler frames derive from it.
    pre_final_frame: Option<Frame>,
    frame: Frame,
}

impl<'instruction> ExitState<'instruction> {
    /// Snapshots the pre-final frame only when an exception arm enters a handler.
    pub(super) fn new(
        instruction: &'instruction Instruction,
        pc: ProgramCounter,
        frame: Frame,
        exit: &BlockExit<BlockId>,
    ) -> Self {
        let exception_arms = match exit {
            BlockExit::Continue { exception_arms, .. }
            | BlockExit::Return { exception_arms }
            | BlockExit::Throw { exception_arms } => exception_arms.as_slice(),
            BlockExit::Goto { .. } | BlockExit::Branch { .. } | BlockExit::Switch { .. } => &[],
        };
        let pre_final_frame = exception_arms
            .iter()
            .any(|arm| matches!(arm.target, ExceptionTarget::Handler(_)))
            .then(|| frame.clone());
        Self {
            instruction,
            pc,
            pre_final_frame,
            frame,
        }
    }

    /// Interprets the block exit, producing the block's terminator.
    pub(super) fn terminate(
        self,
        values: &mut ValueContext,
        exit: BlockExit<BlockId>,
        operations: &mut Vec<(ProgramCounter, Operation)>,
    ) -> Result<FrameTerminator, FrameError> {
        match exit {
            BlockExit::Continue {
                next,
                exception_arms,
            } => self.terminate_continue(values, next, exception_arms, operations),
            BlockExit::Goto { target } => Ok(self.terminate_goto(target)),
            BlockExit::Branch { taken, otherwise } => self.terminate_branch(taken, otherwise),
            BlockExit::Switch { cases, default } => self.terminate_switch(cases, default),
            BlockExit::Return { exception_arms } => self.terminate_return(values, exception_arms),
            BlockExit::Throw { exception_arms } => self.terminate_throw(values, exception_arms),
        }
    }

    /// Interprets a continuation, whose final operation is ordinary or fallible.
    fn terminate_continue(
        mut self,
        values: &mut ValueContext,
        next: BlockId,
        exception_arms: Vec<ExceptionArm<BlockId>>,
        operations: &mut Vec<(ProgramCounter, Operation)>,
    ) -> Result<FrameTerminator, FrameError> {
        let operation =
            lifting::lift_instruction(values, self.instruction, self.pc, &mut self.frame)?;
        let has_exceptions = !exception_arms.is_empty();
        let exceptional = self.exception_arms(values, exception_arms)?;
        let normal = FrameArm::block(
            ArmKey::Continue,
            next,
            ControlTransfer::Unconditional,
            self.frame,
        );
        if !has_exceptions {
            if let Some(operation) = operation {
                operations.push((self.pc, operation));
            }
            return Ok(FrameTerminator::Goto { target: normal });
        }
        let operation = operation.expect("a fallible instruction lifts an operation");
        Ok(FrameTerminator::Try {
            operation,
            normal,
            exceptional,
        })
    }

    /// Interprets a goto, which carries the frame unchanged.
    fn terminate_goto(self, target: BlockId) -> FrameTerminator {
        FrameTerminator::Goto {
            target: FrameArm::block(
                ArmKey::Unconditional,
                target,
                ControlTransfer::Unconditional,
                self.frame,
            ),
        }
    }

    /// Interprets a two-way branch, whose guard is read from the final instruction.
    fn terminate_branch(
        mut self,
        taken: BlockId,
        otherwise: BlockId,
    ) -> Result<FrameTerminator, FrameError> {
        let (taken_transfer, otherwise_transfer) =
            effects::branch_transfers(self.instruction, &mut self.frame)?;
        let taken = FrameArm::block(ArmKey::Taken, taken, taken_transfer, self.frame.clone());
        let otherwise =
            FrameArm::block(ArmKey::Otherwise, otherwise, otherwise_transfer, self.frame);
        Ok(FrameTerminator::Branch { taken, otherwise })
    }

    /// Interprets a switch, whose selector is popped and matched by the final instruction.
    fn terminate_switch(
        mut self,
        cases: BTreeMap<i32, BlockId>,
        default: BlockId,
    ) -> Result<FrameTerminator, FrameError> {
        let selector = effects::switch_selector(&mut self.frame)?;
        if cases.is_empty() {
            let target = FrameArm::block(
                ArmKey::Default,
                default,
                ControlTransfer::Unconditional,
                self.frame,
            );
            return Ok(FrameTerminator::Goto { target });
        }
        let default_transfer = effects::default_guard(selector, &cases);
        let cases = cases
            .into_iter()
            .map(|(case, target)| {
                let transfer = effects::case_guard(selector, case);
                FrameArm::block(ArmKey::Case(case), target, transfer, self.frame.clone())
            })
            .collect();
        let default = FrameArm::block(ArmKey::Default, default, default_transfer, self.frame);
        Ok(FrameTerminator::Switch { cases, default })
    }

    /// Interprets a return, which may fail while exiting.
    fn terminate_return(
        mut self,
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
    ) -> Result<FrameTerminator, FrameError> {
        let value = effects::return_operand(self.instruction, &mut self.frame)?;
        let exceptional = self.exception_arms(values, exception_arms)?;
        Ok(FrameTerminator::Return { value, exceptional })
    }

    /// Interprets a throw, which delivers along its ordered exception arms.
    fn terminate_throw(
        mut self,
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
    ) -> Result<FrameTerminator, FrameError> {
        let value = effects::throw_operand(self.instruction, &mut self.frame)?;
        let exceptional = self.exception_arms(values, exception_arms)?;
        Ok(FrameTerminator::Throw { value, exceptional })
    }

    /// Builds dataflow arms from the ordered exception arms of an exit.
    fn exception_arms(
        &self,
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
    ) -> Result<Vec<FrameArm>, FrameError> {
        exception_arms
            .into_iter()
            .enumerate()
            .map(|(index, exception_arm)| {
                let arm = ArmKey::Exception(index);
                match exception_arm.target {
                    ExceptionTarget::Handler(target) => {
                        let caught = values.caught_exception(target);
                        let frame = self
                            .pre_final_frame
                            .as_ref()
                            .expect("handler arms have a pre-final snapshot")
                            .exception_handler_frame(caught)?;
                        let transfer = ControlTransfer::Exception(exception_arm.catch_type);
                        Ok(FrameArm::block(arm, target, transfer, frame))
                    }
                    ExceptionTarget::Unwind => Ok(FrameArm::Unwind { arm }),
                }
            })
            .collect()
    }
}
