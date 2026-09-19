//! Lifting and successor construction for one reachable structural block.

mod terminator;

use super::super::lifting;
use super::{Analyzer, Frame, LiftedArm, LiftedBlock, LiftedEdge, LiftedTerminator};
use crate::ir::{
    BlockId, BlockKind, SuccessorTarget,
    control_flow::ControlTransfer,
    generator::{
        bytecode_cfg::{BlockExit, EdgeKind, JvmBlockId, NormalizedBlockKind, NormalizedEdge},
        error::Error,
    },
};
use terminator::LoweredTerminator;

impl Analyzer<'_, '_> {
    /// Executes one normalized block, producing its analyzed block.
    ///
    /// Its successor identities and targets were fixed before frame analysis.
    pub(super) fn execute(&mut self, block: BlockId, input: Frame) -> Result<LiftedBlock, Error> {
        match self.cfg.block(block).kind {
            NormalizedBlockKind::Bytecode(id) => self.execute_bytecode(block, id, input),
            NormalizedBlockKind::HandlerEntry(_) => {
                let caught = *input.handler_exception().map_err(Error::from)?;
                self.execute_passthrough(block, input, BlockKind::LandingPad { exception: caught })
            }
        }
    }

    fn execute_passthrough(
        &self,
        block: BlockId,
        input: Frame,
        kind: BlockKind,
    ) -> Result<LiftedBlock, Error> {
        let [edge] = self.cfg.block(block).successors.as_slice() else {
            return Err(Error::internal(
                "a normalized passthrough block must have one successor",
            ));
        };
        let target = (
            LiftedEdge {
                id: edge.id,
                target: edge.target,
                transfer: ControlTransfer::Unconditional,
            },
            Some(input),
        );
        Ok(LiftedBlock {
            kind,
            operations: Vec::new(),
            terminator: LiftedTerminator::Goto { target },
            terminator_source: None,
        })
    }

    fn execute_bytecode(
        &mut self,
        block_id: BlockId,
        id: JvmBlockId,
        input: Frame,
    ) -> Result<LiftedBlock, Error> {
        let block = self.cfg.bytecode_block(id);
        let final_pc = block.end_pc;
        let has_explicit_terminator = !matches!(block.exit, BlockExit::Fallthrough { .. });
        let mut frame = input;
        let mut operations = Vec::new();
        let mut instructions = self.cfg.block_instructions(id);
        let (actual_final_pc, final_instruction) = instructions
            .next_back()
            .expect("a structural block must contain its final instruction");
        debug_assert_eq!(actual_final_pc, final_pc);

        for (pc, instruction) in instructions {
            let operation =
                lifting::lift_instruction(&mut self.values, instruction, pc, &mut frame)
                    .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation {
                operations.push((pc, operation));
            }
        }

        let pre_final_frame = frame.clone();
        if !has_explicit_terminator {
            let operation = lifting::lift_instruction(
                &mut self.values,
                final_instruction,
                final_pc,
                &mut frame,
            )
            .map_err(|error| error.at_instruction(final_pc))?;
            if let Some(operation) = operation {
                operations.push((final_pc, operation));
            }
        }
        let (lowered, terminator_operation) =
            terminator::lower(block, final_instruction, &mut frame)
                .map_err(|error| error.at_instruction(final_pc))?;
        if let Some(operation) = terminator_operation {
            operations.push((final_pc, operation));
        }
        let topology = self.cfg.block(block_id).successors.clone();
        let (normal_edges, exceptional_edges): (Vec<_>, Vec<_>) = topology
            .into_iter()
            .partition(|edge| matches!(edge.kind, EdgeKind::Normal));
        let mut normal_edges = normal_edges.into_iter();
        let mut normal = |transfer| {
            let edge = normal_edges.next().ok_or_else(|| {
                Error::internal("lowered successors do not match normalized topology")
            })?;
            Ok::<LiftedArm, Error>((
                LiftedEdge {
                    id: edge.id,
                    target: edge.target,
                    transfer,
                },
                Some(frame.clone()),
            ))
        };
        let exceptional = exceptional_edges
            .iter()
            .map(|edge| self.exception_successor(edge, &pre_final_frame))
            .collect::<Result<Vec<_>, _>>()?;
        let terminator = match lowered {
            LoweredTerminator::Goto(transfer) => LiftedTerminator::Goto {
                target: normal(transfer)?,
            },
            LoweredTerminator::Branch { taken, otherwise } => LiftedTerminator::Branch {
                taken: normal(taken)?,
                otherwise: normal(otherwise)?,
            },
            LoweredTerminator::Switch { cases, default } => LiftedTerminator::Switch {
                cases: cases
                    .into_iter()
                    .map(&mut normal)
                    .collect::<Result<_, _>>()?,
                default: normal(default)?,
            },
            LoweredTerminator::Fallible(transfer) => LiftedTerminator::Fallible {
                normal: Some(normal(transfer)?),
                exceptional,
            },
            LoweredTerminator::Return(value) => LiftedTerminator::Return { value, exceptional },
            LoweredTerminator::Throw(value) => LiftedTerminator::Throw { value, exceptional },
        };
        if normal_edges.next().is_some() {
            return Err(Error::internal(
                "lowered successors do not match normalized topology",
            ));
        }
        Ok(LiftedBlock {
            kind: BlockKind::Code,
            operations,
            terminator,
            terminator_source: has_explicit_terminator.then_some(final_pc),
        })
    }

    fn exception_successor(
        &mut self,
        edge: &NormalizedEdge,
        input_frame: &Frame,
    ) -> Result<LiftedArm, Error> {
        match &edge.kind {
            EdgeKind::Exception(catch_type) => {
                let SuccessorTarget::Block(target) = edge.target else {
                    return Err(Error::internal("an exception handler must target a block"));
                };
                let caught = self.caught_exception(target)?;
                let frame = input_frame.clone().exception_handler_frame(caught)?;
                let lifted = LiftedEdge {
                    id: edge.id,
                    target: edge.target,
                    transfer: ControlTransfer::Exception(catch_type.clone()),
                };
                Ok((lifted, Some(frame)))
            }
            EdgeKind::Unwind => {
                debug_assert_eq!(edge.target, SuccessorTarget::Unwind);
                let lifted = LiftedEdge {
                    id: edge.id,
                    target: edge.target,
                    transfer: ControlTransfer::Unwind,
                };
                Ok((lifted, None))
            }
            EdgeKind::Normal => Err(Error::internal(
                "an ordinary edge was treated as an exceptional successor",
            )),
        }
    }
}
