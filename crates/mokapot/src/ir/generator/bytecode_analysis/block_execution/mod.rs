//! Lifting and successor construction for one reachable structural block.

mod terminator;

use super::{
    analyzer::Analyzer,
    model::{AnalyzedBlock, AnalyzedEdge, AnalyzedSuccessors, Frame, Location},
};
use crate::{
    ir::generator::{
        bytecode_cfg::{self, BlockExit, JvmBlockId},
        error::Error,
    },
    ir::{TerminatorKind, control_flow::ControlTransfer},
};

impl Analyzer<'_, '_> {
    /// Executes one location, producing its analyzed block.
    ///
    /// The structural CFG fixes the successor target set independently of the
    /// input frame, allowing `Analyzer::run` to grow predecessor sets monotonically.
    pub(super) fn execute(
        &mut self,
        location: Location,
        input: Frame,
    ) -> Result<AnalyzedBlock, Error> {
        match location {
            Location::Bytecode(id) => self.execute_bytecode(id, input),
            Location::Handler(block) => Self::execute_handler(block, input),
            Location::Unwind => Ok(AnalyzedBlock {
                caught_exception: None,
                operations: Vec::new(),
                terminator: TerminatorKind::Unwind,
                terminator_source: None,
                successors: AnalyzedSuccessors::default(),
            }),
        }
    }

    fn execute_handler(block: JvmBlockId, input: Frame) -> Result<AnalyzedBlock, Error> {
        let caught = *input.handler_exception().map_err(Error::from)?;
        let target = Location::Bytecode(block);
        let mut successors = AnalyzedSuccessors::default();
        successors.push(
            AnalyzedEdge {
                target,
                transfer: ControlTransfer::Unconditional,
            },
            input,
        );
        Ok(AnalyzedBlock {
            caught_exception: Some(caught),
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            successors,
        })
    }

    fn execute_bytecode(&mut self, id: JvmBlockId, input: Frame) -> Result<AnalyzedBlock, Error> {
        let block = self.cfg.block(id);
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
            let operation = self
                .executor
                .lift_instruction(instruction, pc, &mut frame)
                .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation {
                operations.push((pc, operation));
            }
        }

        let pre_final_frame = frame.clone();
        if !has_explicit_terminator {
            let operation = self
                .executor
                .lift_instruction(final_instruction, final_pc, &mut frame)
                .map_err(|error| error.at_instruction(final_pc))?;
            if let Some(operation) = operation {
                operations.push((final_pc, operation));
            }
        }
        let (terminator, edges, terminator_operation) =
            Self::lower_terminator(block, final_instruction, &mut frame)
                .map_err(|error| error.at_instruction(final_pc))?;
        if let Some(operation) = terminator_operation {
            operations.push((final_pc, operation));
        }
        let mut successors = AnalyzedSuccessors::default();
        for edge in edges {
            successors.push(edge, frame.clone());
        }
        for exceptional_target in &block.exception_handlers {
            self.push_exception_successor(exceptional_target, &pre_final_frame, &mut successors)?;
        }
        Ok(AnalyzedBlock {
            caught_exception: None,
            operations,
            terminator,
            terminator_source: has_explicit_terminator.then_some(final_pc),
            successors,
        })
    }

    fn push_exception_successor(
        &mut self,
        exceptional_target: &bytecode_cfg::ExceptionalTarget,
        input_frame: &Frame,
        successors: &mut AnalyzedSuccessors,
    ) -> Result<(), Error> {
        match exceptional_target {
            bytecode_cfg::ExceptionalTarget::Handler { block, catch_type } => {
                let location = Location::Handler(*block);
                let caught = self.caught_exception(*block)?;
                let frame = input_frame.clone().exception_handler_frame(caught)?;
                successors.push(
                    AnalyzedEdge {
                        target: location,
                        transfer: ControlTransfer::Exception(catch_type.clone()),
                    },
                    frame,
                );
            }
            bytecode_cfg::ExceptionalTarget::Unwind => {
                successors.push(
                    AnalyzedEdge {
                        target: Location::Unwind,
                        transfer: ControlTransfer::Unwind,
                    },
                    input_frame.clone().into_unwind_frame(),
                );
            }
        }
        Ok(())
    }
}
