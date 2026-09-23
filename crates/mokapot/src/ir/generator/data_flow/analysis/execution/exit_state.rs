//! Interpretation of a block's final instruction into its terminator.

use std::collections::BTreeMap;

use super::{
    super::{
        super::{Frame, lifting, values::ValueContext},
        frame_block::{FrameArm, FrameTerminator},
    },
    effects,
};
use crate::{
    ir::{
        BlockId, ControlTransfer, Operation,
        generator::{
            control_flow::{ArmKey, BlockExit, ExceptionArm, ExceptionTarget},
            error::Error,
        },
    },
    jvm::code::{Instruction, ProgramCounter},
};

/// The frame and operations reached at a block's final instruction.
pub(super) struct ExitState<'instruction> {
    instruction: &'instruction Instruction,
    pc: ProgramCounter,
    /// The frame before the final instruction runs; handler frames derive from it.
    pre_final_frame: Frame,
    frame: Frame,
    operations: Vec<(ProgramCounter, Operation)>,
}

impl<'instruction> ExitState<'instruction> {
    /// Snapshots `frame` into the pre-final frame before the exit runs.
    pub(super) fn new(
        instruction: &'instruction Instruction,
        pc: ProgramCounter,
        frame: Frame,
        operations: Vec<(ProgramCounter, Operation)>,
    ) -> Self {
        Self {
            instruction,
            pc,
            pre_final_frame: frame.clone(),
            frame,
            operations,
        }
    }

    /// Interprets the block exit, producing the block's terminator.
    pub(super) fn terminate(
        &mut self,
        values: &mut ValueContext,
        exit: BlockExit<BlockId>,
    ) -> Result<FrameTerminator, Error> {
        match exit {
            BlockExit::Continue {
                next,
                exception_arms,
            } => self.terminate_continue(values, next, exception_arms),
            BlockExit::Goto { target } => Ok(self.terminate_goto(target)),
            BlockExit::Branch { taken, otherwise } => self.terminate_branch(taken, otherwise),
            BlockExit::Switch { cases, default } => self.terminate_switch(cases, default),
            BlockExit::Return { exception_arms } => self.terminate_return(values, exception_arms),
            BlockExit::Throw { exception_arms } => self.terminate_throw(values, exception_arms),
        }
    }

    /// Consumes the state, returning the operations lifted up to the exit.
    pub(super) fn into_operations(self) -> Vec<(ProgramCounter, Operation)> {
        self.operations
    }

    /// Interprets a continuation, whose final operation is ordinary or fallible.
    fn terminate_continue(
        &mut self,
        values: &mut ValueContext,
        next: BlockId,
        exception_arms: Vec<ExceptionArm<BlockId>>,
    ) -> Result<FrameTerminator, Error> {
        let operation =
            lifting::lift_instruction(values, self.instruction, self.pc, &mut self.frame)
                .map_err(|error| error.at_pc(self.pc))?;
        let normal = FrameArm::block(
            ArmKey::Continue,
            next,
            ControlTransfer::Unconditional,
            self.frame.clone(),
        );
        if exception_arms.is_empty() {
            if let Some(operation) = operation {
                self.operations.push((self.pc, operation));
            }
            return Ok(FrameTerminator::Goto { target: normal });
        }
        let operation = operation.expect("a fallible instruction lifts an operation");
        let exceptional = self.exception_arms(values, exception_arms)?;
        Ok(FrameTerminator::Try {
            operation,
            normal,
            exceptional,
        })
    }

    /// Interprets a goto, which carries the frame unchanged.
    fn terminate_goto(&self, target: BlockId) -> FrameTerminator {
        FrameTerminator::Goto {
            target: FrameArm::block(
                ArmKey::Unconditional,
                target,
                ControlTransfer::Unconditional,
                self.frame.clone(),
            ),
        }
    }

    /// Interprets a two-way branch, whose guard is read from the final instruction.
    fn terminate_branch(
        &mut self,
        taken: BlockId,
        otherwise: BlockId,
    ) -> Result<FrameTerminator, Error> {
        let (taken_transfer, otherwise_transfer) =
            effects::branch_transfers(self.instruction, &mut self.frame)
                .map_err(|e| e.at_pc(self.pc))?;
        let taken = FrameArm::block(ArmKey::Taken, taken, taken_transfer, self.frame.clone());
        let otherwise = FrameArm::block(
            ArmKey::Otherwise,
            otherwise,
            otherwise_transfer,
            self.frame.clone(),
        );
        Ok(FrameTerminator::Branch { taken, otherwise })
    }

    /// Interprets a switch, whose selector is popped and matched by the final instruction.
    fn terminate_switch(
        &mut self,
        cases: BTreeMap<i32, BlockId>,
        default: BlockId,
    ) -> Result<FrameTerminator, Error> {
        let selector = effects::switch_selector(&mut self.frame).map_err(|e| e.at_pc(self.pc))?;
        if cases.is_empty() {
            let target = FrameArm::block(
                ArmKey::Default,
                default,
                ControlTransfer::Unconditional,
                self.frame.clone(),
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
        let default = FrameArm::block(
            ArmKey::Default,
            default,
            default_transfer,
            self.frame.clone(),
        );
        Ok(FrameTerminator::Switch { cases, default })
    }

    /// Interprets a return, which may fail while exiting.
    fn terminate_return(
        &mut self,
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
    ) -> Result<FrameTerminator, Error> {
        let value = effects::return_operand(self.instruction, &mut self.frame)
            .map_err(|e| e.at_pc(self.pc))?;
        let exceptional = self.exception_arms(values, exception_arms)?;
        Ok(FrameTerminator::Return { value, exceptional })
    }

    /// Interprets a throw, which delivers along its ordered exception arms.
    fn terminate_throw(
        &mut self,
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
    ) -> Result<FrameTerminator, Error> {
        let value = effects::throw_operand(self.instruction, &mut self.frame)
            .map_err(|e| e.at_pc(self.pc))?;
        let exceptional = self.exception_arms(values, exception_arms)?;
        Ok(FrameTerminator::Throw { value, exceptional })
    }

    /// Builds dataflow arms from the ordered exception arms of an exit.
    fn exception_arms(
        &self,
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
    ) -> Result<Vec<FrameArm>, Error> {
        exception_arms
            .into_iter()
            .enumerate()
            .map(|(index, exception_arm)| {
                let arm = ArmKey::Exception(index);
                match exception_arm.target {
                    ExceptionTarget::Handler(target) => {
                        let caught = values.caught_exception(target);
                        let frame = self.pre_final_frame.exception_handler_frame(caught)?;
                        let transfer = ControlTransfer::Exception(exception_arm.catch_type);
                        Ok(FrameArm::block(arm, target, transfer, frame))
                    }
                    ExceptionTarget::Unwind => Ok(FrameArm::Unwind { arm }),
                }
            })
            .collect()
    }
}
