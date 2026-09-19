//! Lifting and successor construction for one reachable structural block.

mod terminator;

use super::super::lifting;
use super::{Analyzer, Frame, LiftedBlock, LiftedEdge, LiftedSuccessors};
use crate::ir::{
    BlockId, TerminatorKind,
    control_flow::ControlTransfer,
    generator::{
        bytecode_cfg::{BlockExit, EdgeKind, JvmBlockId, NormalizedBlockKind, NormalizedEdge},
        error::Error,
    },
};

impl Analyzer<'_, '_> {
    /// Executes one normalized block, producing its analyzed block.
    ///
    /// Its successor identities and targets were fixed before frame analysis.
    pub(super) fn execute(&mut self, block: BlockId, input: Frame) -> Result<LiftedBlock, Error> {
        match self.cfg.block(block).kind {
            NormalizedBlockKind::Bytecode(id) => self.execute_bytecode(block, id, input),
            NormalizedBlockKind::HandlerEntry(_) => {
                let caught = *input.handler_exception().map_err(Error::from)?;
                self.execute_passthrough(block, input, Some(caught))
            }
            NormalizedBlockKind::Unwind => Ok(LiftedBlock {
                caught_exception: None,
                operations: Vec::new(),
                terminator: TerminatorKind::Unwind,
                terminator_source: None,
                successors: LiftedSuccessors::default(),
            }),
        }
    }

    fn execute_passthrough(
        &self,
        block: BlockId,
        input: Frame,
        caught_exception: Option<crate::ir::ValueId>,
    ) -> Result<LiftedBlock, Error> {
        let [edge] = self.cfg.block(block).successors.as_slice() else {
            return Err(Error::internal(
                "a normalized passthrough block must have one successor",
            ));
        };
        let mut successors = LiftedSuccessors::default();
        successors.push(
            LiftedEdge {
                id: edge.id,
                target: edge.target,
                transfer: ControlTransfer::Unconditional,
            },
            input,
        );
        Ok(LiftedBlock {
            caught_exception,
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            successors,
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
        let (terminator, transfers, terminator_operation) =
            terminator::lower(block, final_instruction, &mut frame)
                .map_err(|error| error.at_instruction(final_pc))?;
        if let Some(operation) = terminator_operation {
            operations.push((final_pc, operation));
        }
        let mut successors = LiftedSuccessors::default();
        let topology = self.cfg.block(block_id).successors.clone();
        let (normal_edges, exceptional_edges): (Vec<_>, Vec<_>) = topology
            .into_iter()
            .partition(|edge| matches!(edge.kind, EdgeKind::Normal));
        if normal_edges.len() != transfers.len() {
            return Err(Error::internal(
                "lowered successors do not match normalized topology",
            ));
        }
        for (edge, transfer) in normal_edges.into_iter().zip(transfers) {
            successors.push(
                LiftedEdge {
                    id: edge.id,
                    target: edge.target,
                    transfer,
                },
                frame.clone(),
            );
        }
        for edge in exceptional_edges {
            self.push_exception_successor(&edge, &pre_final_frame, &mut successors)?;
        }
        Ok(LiftedBlock {
            caught_exception: None,
            operations,
            terminator,
            terminator_source: has_explicit_terminator.then_some(final_pc),
            successors,
        })
    }

    fn push_exception_successor(
        &mut self,
        edge: &NormalizedEdge,
        input_frame: &Frame,
        successors: &mut LiftedSuccessors,
    ) -> Result<(), Error> {
        match &edge.kind {
            EdgeKind::Exception(catch_type) => {
                let caught = self.caught_exception(edge.target)?;
                let frame = input_frame.clone().exception_handler_frame(caught)?;
                successors.push(
                    LiftedEdge {
                        id: edge.id,
                        target: edge.target,
                        transfer: ControlTransfer::Exception(catch_type.clone()),
                    },
                    frame,
                );
            }
            EdgeKind::Unwind => {
                successors.push(
                    LiftedEdge {
                        id: edge.id,
                        target: edge.target,
                        transfer: ControlTransfer::Unwind,
                    },
                    input_frame.clone().into_unwind_frame(),
                );
            }
            EdgeKind::Normal => {
                return Err(Error::internal(
                    "an ordinary edge was treated as an exceptional successor",
                ));
            }
        }
        Ok(())
    }
}
