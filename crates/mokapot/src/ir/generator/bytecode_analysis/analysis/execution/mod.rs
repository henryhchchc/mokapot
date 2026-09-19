//! Lifting and successor construction for one reachable structural block.

mod dataflow;

use std::collections::BTreeMap;

use super::super::lifting;
use super::{Analyzer, Frame, LiftedArm, LiftedBlock, LiftedTerminator};
use crate::ir::{
    BlockId, BlockKind, Operation,
    control_flow::ControlTransfer,
    generator::{
        bytecode_cfg::{ArmId, Block, ControlFlow, Handler, Target},
        error::Error,
    },
};
use crate::jvm::code::{Instruction, ProgramCounter};

/// The frame and operations reached at a block's final instruction.
struct BlockTail<'instruction> {
    instruction: &'instruction Instruction,
    pc: ProgramCounter,
    /// The frame before the final instruction runs; handler frames derive from it.
    pre_final_frame: Frame,
    frame: Frame,
    operations: Vec<(ProgramCounter, Operation)>,
}

/// Builds one block arm carrying `frame`.
const fn block_arm(
    arm: ArmId,
    target: BlockId,
    transfer: ControlTransfer,
    frame: Frame,
) -> LiftedArm {
    LiftedArm::Block {
        arm,
        target,
        transfer,
        frame,
    }
}

impl Analyzer<'_, '_> {
    /// Executes one block, producing its analyzed block.
    ///
    /// Its successor targets and arms were fixed before frame analysis.
    pub(super) fn execute(&mut self, block: BlockId, input: Frame) -> Result<LiftedBlock, Error> {
        let node = self.cfg.block(block);
        match node {
            Block::Bytecode {
                start_pc, end_pc, ..
            } => {
                let control = node
                    .control()
                    .expect("a bytecode block has a control transfer")
                    .clone();
                self.execute_bytecode(*start_pc, *end_pc, control, input)
            }
            Block::HandlerEntry { successor } => {
                let caught = *input.handler_exception().map_err(Error::from)?;
                let kind = BlockKind::LandingPad { exception: caught };
                Ok(Self::execute_passthrough(*successor, input, kind))
            }
        }
    }

    /// Lowers a landing pad, which unconditionally enters its handler block.
    const fn execute_passthrough(successor: BlockId, input: Frame, kind: BlockKind) -> LiftedBlock {
        let target = block_arm(
            ArmId::Goto,
            successor,
            ControlTransfer::Unconditional,
            input,
        );
        LiftedBlock {
            kind,
            operations: Vec::new(),
            terminator: LiftedTerminator::Goto { target },
            terminator_source: None,
        }
    }

    fn execute_bytecode(
        &mut self,
        start_pc: ProgramCounter,
        end_pc: ProgramCounter,
        control: ControlFlow<BlockId>,
        input: Frame,
    ) -> Result<LiftedBlock, Error> {
        let mut frame = input;
        let mut instructions = self.cfg.instructions_in(start_pc, end_pc);
        let (final_pc, instruction) = instructions
            .next_back()
            .expect("a structural block must contain its final instruction");
        debug_assert_eq!(final_pc, end_pc);
        let mut operations = Vec::new();
        for (pc, instruction) in instructions {
            let operation =
                lifting::lift_instruction(&mut self.values, instruction, pc, &mut frame)
                    .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation {
                operations.push((pc, operation));
            }
        }
        let mut tail = BlockTail {
            instruction,
            pc: final_pc,
            pre_final_frame: frame.clone(),
            frame,
            operations,
        };
        let terminator_source = control.has_terminator_source().then_some(final_pc);
        let terminator = match control {
            ControlFlow::Fallthrough { next, handlers } => {
                self.lower_fallthrough(next, handlers, &mut tail)?
            }
            ControlFlow::Goto { target } => LiftedTerminator::Goto {
                target: block_arm(
                    ArmId::Goto,
                    target,
                    ControlTransfer::Unconditional,
                    tail.frame.clone(),
                ),
            },
            ControlFlow::Branch { taken, otherwise } => lower_branch(taken, otherwise, &mut tail)?,
            ControlFlow::Switch { cases, default } => lower_switch(cases, default, &mut tail)?,
            ControlFlow::Return { handlers } => self.lower_return(handlers, &mut tail)?,
            ControlFlow::Throw { handlers } => self.lower_throw(handlers, &mut tail)?,
        };
        Ok(LiftedBlock {
            kind: BlockKind::Code,
            operations: tail.operations,
            terminator,
            terminator_source,
        })
    }

    /// Lowers a fallthrough, whose final operation is ordinary or fallible.
    fn lower_fallthrough(
        &mut self,
        next: BlockId,
        handlers: Vec<Handler<BlockId>>,
        tail: &mut BlockTail<'_>,
    ) -> Result<LiftedTerminator, Error> {
        let operation =
            lifting::lift_instruction(&mut self.values, tail.instruction, tail.pc, &mut tail.frame)
                .map_err(|error| error.at_instruction(tail.pc))?;
        let normal = block_arm(
            ArmId::Fallthrough,
            next,
            ControlTransfer::Unconditional,
            tail.frame.clone(),
        );
        if handlers.is_empty() {
            if let Some(operation) = operation {
                tail.operations.push((tail.pc, operation));
            }
            return Ok(LiftedTerminator::Goto { target: normal });
        }
        let operation = operation
            .ok_or_else(|| Error::internal("a fallible block must end in an operation"))?;
        let exceptional = self.exceptional_arms(handlers, &tail.pre_final_frame)?;
        Ok(LiftedTerminator::Try {
            operation,
            normal,
            exceptional,
        })
    }

    /// Lowers a return, which may fail while exiting.
    fn lower_return(
        &mut self,
        handlers: Vec<Handler<BlockId>>,
        tail: &mut BlockTail<'_>,
    ) -> Result<LiftedTerminator, Error> {
        let value = dataflow::return_operand(tail.instruction, &mut tail.frame)
            .map_err(|error| error.at_instruction(tail.pc))?;
        if handlers.is_empty() {
            return Ok(LiftedTerminator::Return { value });
        }
        let exceptional = self.exceptional_arms(handlers, &tail.pre_final_frame)?;
        Ok(LiftedTerminator::TryReturn { value, exceptional })
    }

    /// Lowers a throw, which delivers to its handlers or unwinds.
    fn lower_throw(
        &mut self,
        handlers: Vec<Handler<BlockId>>,
        tail: &mut BlockTail<'_>,
    ) -> Result<LiftedTerminator, Error> {
        let value = dataflow::throw_operand(tail.instruction, &mut tail.frame)
            .map_err(|error| error.at_instruction(tail.pc))?;
        let exceptional = self.exceptional_arms(handlers, &tail.pre_final_frame)?;
        Ok(LiftedTerminator::Throw { value, exceptional })
    }

    /// Lowers the ordered exception handlers of a terminator into arms.
    fn exceptional_arms(
        &mut self,
        handlers: Vec<Handler<BlockId>>,
        input_frame: &Frame,
    ) -> Result<Vec<LiftedArm>, Error> {
        handlers
            .into_iter()
            .enumerate()
            .map(|(index, handler)| {
                let arm = ArmId::Handler(index);
                match handler.target {
                    Target::Block(target) => {
                        let caught = self.caught_exception(target);
                        let frame = input_frame.clone().exception_handler_frame(caught)?;
                        let transfer = ControlTransfer::Exception(handler.catch);
                        Ok(block_arm(arm, target, transfer, frame))
                    }
                    Target::Unwind => Ok(LiftedArm::Unwind { arm }),
                }
            })
            .collect()
    }
}

/// Lowers a two-way branch, whose guard is read from the final instruction.
fn lower_branch(
    taken: BlockId,
    otherwise: BlockId,
    tail: &mut BlockTail<'_>,
) -> Result<LiftedTerminator, Error> {
    let (taken_transfer, otherwise_transfer) =
        dataflow::branch_transfers(tail.instruction, &mut tail.frame)
            .map_err(|error| error.at_instruction(tail.pc))?;
    Ok(LiftedTerminator::Branch {
        taken: block_arm(ArmId::Taken, taken, taken_transfer, tail.frame.clone()),
        otherwise: block_arm(
            ArmId::Otherwise,
            otherwise,
            otherwise_transfer,
            tail.frame.clone(),
        ),
    })
}

/// Lowers a switch, whose selector is popped and matched by the final instruction.
fn lower_switch(
    cases: BTreeMap<i32, BlockId>,
    default: BlockId,
    tail: &mut BlockTail<'_>,
) -> Result<LiftedTerminator, Error> {
    let selector = dataflow::switch_selector(&mut tail.frame)
        .map_err(|error| error.at_instruction(tail.pc))?;
    if cases.is_empty() {
        return Ok(LiftedTerminator::Goto {
            target: block_arm(
                ArmId::Default,
                default,
                ControlTransfer::Unconditional,
                tail.frame.clone(),
            ),
        });
    }
    let default_transfer = dataflow::default_guard(selector, &cases);
    let cases = cases
        .into_iter()
        .map(|(case, target)| {
            let transfer = dataflow::case_guard(selector, case);
            block_arm(ArmId::Case(case), target, transfer, tail.frame.clone())
        })
        .collect();
    Ok(LiftedTerminator::Switch {
        cases,
        default: block_arm(
            ArmId::Default,
            default,
            default_transfer,
            tail.frame.clone(),
        ),
    })
}
