//! Lifting and successor construction for one reachable structural block.

mod effects;

use std::collections::BTreeMap;

use super::{
    super::{Frame, lifting, values::ValueContext},
    frame_block::{FrameArm, FrameBlock, FrameTerminator},
};
use crate::{
    ir::{
        BlockId, BlockKind, Operation,
        control_flow::ControlTransfer,
        generator::{
            cfg::{ArmKey, BlockExit, Cfg, CfgNode, ExceptionArm, ExceptionTarget},
            error::Error,
        },
    },
    jvm::code::{Instruction, ProgramCounter},
};

/// Interprets structural blocks using the solver's shared value identity context.
pub(super) struct BlockInterpreter<'method, 'cfg> {
    cfg: &'cfg Cfg<'method>,
}

impl<'method, 'cfg> BlockInterpreter<'method, 'cfg> {
    pub(super) const fn new(cfg: &'cfg Cfg<'method>) -> Self {
        Self { cfg }
    }

    pub(super) fn block_pc(&self, block: BlockId) -> Option<ProgramCounter> {
        match self.cfg.block(block) {
            CfgNode::Code { start_pc, .. } => Some(*start_pc),
            CfgNode::LandingPad { .. } => None,
        }
    }

    pub(super) const fn entry_block(&self) -> BlockId {
        self.cfg.entry_block()
    }
}

/// The frame and operations reached at a block's final instruction.
struct ExitState<'instruction> {
    instruction: &'instruction Instruction,
    pc: ProgramCounter,
    /// The frame before the final instruction runs; handler frames derive from it.
    pre_final_frame: Frame,
    frame: Frame,
    operations: Vec<(ProgramCounter, Operation)>,
}

/// Builds one block arm carrying `frame`.
const fn block_arm(
    arm: ArmKey,
    target: BlockId,
    transfer: ControlTransfer,
    frame: Frame,
) -> FrameArm {
    FrameArm::Block {
        arm,
        target,
        transfer,
        frame,
    }
}

impl BlockInterpreter<'_, '_> {
    /// Interprets one block, producing its analyzed block.
    ///
    /// Its successor targets and arms were fixed before frame analysis.
    pub(super) fn interpret(
        &self,
        values: &mut ValueContext,
        block: BlockId,
        input: Frame,
    ) -> Result<FrameBlock, Error> {
        let node = self.cfg.block(block);
        match node {
            CfgNode::Code {
                start_pc,
                end_pc,
                exit,
            } => self.interpret_bytecode(values, *start_pc, *end_pc, exit.clone(), input),
            CfgNode::LandingPad { successor } => {
                let caught = *input.handler_exception().map_err(Error::from)?;
                let kind = BlockKind::LandingPad { exception: caught };
                Ok(Self::interpret_landing_pad(*successor, input, kind))
            }
        }
    }

    /// Interprets a landing pad, which unconditionally enters its handler block.
    const fn interpret_landing_pad(
        successor: BlockId,
        input: Frame,
        kind: BlockKind,
    ) -> FrameBlock {
        let target = block_arm(
            ArmKey::Unconditional,
            successor,
            ControlTransfer::Unconditional,
            input,
        );
        FrameBlock::new(kind, Vec::new(), FrameTerminator::Goto { target }, None)
    }

    fn interpret_bytecode(
        &self,
        values: &mut ValueContext,
        start_pc: ProgramCounter,
        end_pc: ProgramCounter,
        exit: BlockExit<BlockId>,
        input: Frame,
    ) -> Result<FrameBlock, Error> {
        let mut frame = input;
        let mut instructions = self.cfg.instructions_in(start_pc, end_pc);
        let (final_pc, instruction) = instructions
            .next_back()
            .expect("a structural block must contain its final instruction");
        debug_assert_eq!(final_pc, end_pc);
        let mut operations = Vec::new();
        for (pc, instruction) in instructions {
            let operation = lifting::lift_instruction(values, instruction, pc, &mut frame)
                .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation {
                operations.push((pc, operation));
            }
        }
        let mut exit_state = ExitState {
            instruction,
            pc: final_pc,
            pre_final_frame: frame.clone(),
            frame,
            operations,
        };
        let terminator_source = exit.forces_block_boundary().then_some(final_pc);
        let terminator = match exit {
            BlockExit::Continue {
                next,
                exception_arms,
            } => Self::interpret_continue(values, next, exception_arms, &mut exit_state)?,
            BlockExit::Goto { target } => FrameTerminator::Goto {
                target: block_arm(
                    ArmKey::Unconditional,
                    target,
                    ControlTransfer::Unconditional,
                    exit_state.frame.clone(),
                ),
            },
            BlockExit::Branch { taken, otherwise } => {
                interpret_branch(taken, otherwise, &mut exit_state)?
            }
            BlockExit::Switch { cases, default } => {
                interpret_switch(cases, default, &mut exit_state)?
            }
            BlockExit::Return { exception_arms } => {
                Self::interpret_return(values, exception_arms, &mut exit_state)?
            }
            BlockExit::Throw { exception_arms } => {
                Self::interpret_throw(values, exception_arms, &mut exit_state)?
            }
        };
        Ok(FrameBlock::new(
            BlockKind::Code,
            exit_state.operations,
            terminator,
            terminator_source,
        ))
    }

    /// Interprets a continuation, whose final operation is ordinary or fallible.
    fn interpret_continue(
        values: &mut ValueContext,
        next: BlockId,
        exception_arms: Vec<ExceptionArm<BlockId>>,
        exit: &mut ExitState<'_>,
    ) -> Result<FrameTerminator, Error> {
        let operation =
            lifting::lift_instruction(values, exit.instruction, exit.pc, &mut exit.frame)
                .map_err(|error| error.at_instruction(exit.pc))?;
        let normal = block_arm(
            ArmKey::Continue,
            next,
            ControlTransfer::Unconditional,
            exit.frame.clone(),
        );
        if exception_arms.is_empty() {
            if let Some(operation) = operation {
                exit.operations.push((exit.pc, operation));
            }
            return Ok(FrameTerminator::Goto { target: normal });
        }
        let operation = operation.expect("a fallible instruction must produce an operation");
        let exception_arms =
            Self::build_exception_arms(values, exception_arms, &exit.pre_final_frame)?;
        Ok(FrameTerminator::Try {
            operation,
            normal,
            exceptional: exception_arms,
        })
    }

    /// Interprets a return, which may fail while exiting.
    fn interpret_return(
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
        exit: &mut ExitState<'_>,
    ) -> Result<FrameTerminator, Error> {
        let value = effects::return_operand(exit.instruction, &mut exit.frame)
            .map_err(|error| error.at_instruction(exit.pc))?;
        if exception_arms.is_empty() {
            return Ok(FrameTerminator::Return { value });
        }
        let exceptional =
            Self::build_exception_arms(values, exception_arms, &exit.pre_final_frame)?;
        Ok(FrameTerminator::TryReturn { value, exceptional })
    }

    /// Interprets a throw, which delivers along its ordered exception arms.
    fn interpret_throw(
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
        exit: &mut ExitState<'_>,
    ) -> Result<FrameTerminator, Error> {
        let value = effects::throw_operand(exit.instruction, &mut exit.frame)
            .map_err(|error| error.at_instruction(exit.pc))?;
        let exceptional =
            Self::build_exception_arms(values, exception_arms, &exit.pre_final_frame)?;
        Ok(FrameTerminator::Throw { value, exceptional })
    }

    /// Builds dataflow arms from the ordered exception arms of an exit.
    fn build_exception_arms(
        values: &mut ValueContext,
        exception_arms: Vec<ExceptionArm<BlockId>>,
        input_frame: &Frame,
    ) -> Result<Vec<FrameArm>, Error> {
        exception_arms
            .into_iter()
            .enumerate()
            .map(|(index, exception_arm)| {
                let arm = ArmKey::Exception(index);
                match exception_arm.target {
                    ExceptionTarget::Handler(target) => {
                        let caught = values.caught_exception(target);
                        let frame = input_frame.clone().exception_handler_frame(caught)?;
                        let transfer = ControlTransfer::Exception(exception_arm.catch_type);
                        Ok(block_arm(arm, target, transfer, frame))
                    }
                    ExceptionTarget::Unwind => Ok(FrameArm::Unwind { arm }),
                }
            })
            .collect()
    }
}

/// Interprets a two-way branch, whose guard is read from the final instruction.
fn interpret_branch(
    taken: BlockId,
    otherwise: BlockId,
    exit: &mut ExitState<'_>,
) -> Result<FrameTerminator, Error> {
    let (taken_transfer, otherwise_transfer) =
        effects::branch_transfers(exit.instruction, &mut exit.frame)
            .map_err(|error| error.at_instruction(exit.pc))?;
    let taken = block_arm(ArmKey::Taken, taken, taken_transfer, exit.frame.clone());
    let otherwise = block_arm(
        ArmKey::Otherwise,
        otherwise,
        otherwise_transfer,
        exit.frame.clone(),
    );
    Ok(FrameTerminator::Branch { taken, otherwise })
}

/// Interprets a switch, whose selector is popped and matched by the final instruction.
fn interpret_switch(
    cases: BTreeMap<i32, BlockId>,
    default: BlockId,
    exit: &mut ExitState<'_>,
) -> Result<FrameTerminator, Error> {
    let selector =
        effects::switch_selector(&mut exit.frame).map_err(|error| error.at_instruction(exit.pc))?;
    if cases.is_empty() {
        let target = block_arm(
            ArmKey::Default,
            default,
            ControlTransfer::Unconditional,
            exit.frame.clone(),
        );
        return Ok(FrameTerminator::Goto { target });
    }
    let default_transfer = effects::default_guard(selector, &cases);
    let cases = cases
        .into_iter()
        .map(|(case, target)| {
            let transfer = effects::case_guard(selector, case);
            block_arm(ArmKey::Case(case), target, transfer, exit.frame.clone())
        })
        .collect();
    let default = block_arm(
        ArmKey::Default,
        default,
        default_transfer,
        exit.frame.clone(),
    );
    Ok(FrameTerminator::Switch { cases, default })
}
