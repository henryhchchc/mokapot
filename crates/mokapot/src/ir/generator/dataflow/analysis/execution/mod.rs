//! Lifting and successor construction for one reachable structural block.

mod effects;
mod exit_state;

use exit_state::ExitState;

use super::{
    super::{Frame, lifting, values::ValueContext},
    frame_block::{FrameArm, FrameBlock, FrameTerminator},
};
use crate::{
    ir::{
        BlockId, BlockKind,
        control_flow::ControlTransfer,
        generator::{
            cfg::{ArmKey, BlockExit, Cfg, CfgNode},
            error::Error,
        },
    },
    jvm::code::ProgramCounter,
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
        let target = FrameArm::block(
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
            .expect("a structural block contains its final instruction");
        debug_assert_eq!(
            final_pc, end_pc,
            "the block's last instruction is not at its end pc"
        );
        let mut operations = Vec::new();
        for (pc, instruction) in instructions {
            if let Some(operation) = lifting::lift_instruction(values, instruction, pc, &mut frame)
                .map_err(|e| e.at_instruction(pc))?
            {
                operations.push((pc, operation));
            }
        }
        let terminator_source = exit.forces_block_boundary().then_some(final_pc);
        let mut exit_state = ExitState::new(instruction, final_pc, frame, operations);
        let terminator = exit_state.terminate(values, exit)?;
        Ok(FrameBlock::new(
            BlockKind::Code,
            exit_state.into_operations(),
            terminator,
            terminator_source,
        ))
    }
}
