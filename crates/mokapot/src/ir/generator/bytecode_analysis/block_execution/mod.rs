//! Lifting and successor construction for one reachable structural block.

mod terminator;

use std::collections::{BTreeMap, btree_map::Entry};

use super::{
    analyzer::Analyzer,
    model::{AnalyzedBlock, AnalyzedEdge, Frame, Location},
};
use crate::{
    ir::generator::{
        bytecode_cfg::{self, BlockExit, StructuralBlockId},
        error::{Error, MalformedBytecode},
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
            Location::Handler(id) => Self::execute_handler(id, input),
            Location::Unwind => Ok(AnalyzedBlock {
                caught_exception: None,
                operations: Vec::new(),
                terminator: TerminatorKind::Unwind,
                terminator_source: None,
                edges: Vec::new(),
                output_frames: BTreeMap::default(),
            }),
        }
    }

    fn execute_handler(id: bytecode_cfg::HandlerId, input: Frame) -> Result<AnalyzedBlock, Error> {
        let caught = *input.handler_exception().map_err(Error::from)?;
        let target = Location::Bytecode(StructuralBlockId::from_pc(id.pc()));
        Ok(AnalyzedBlock {
            caught_exception: Some(caught),
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            edges: vec![AnalyzedEdge {
                target,
                transfer: ControlTransfer::Unconditional,
            }],
            output_frames: [(target, input)].into_iter().collect(),
        })
    }

    fn execute_bytecode(
        &mut self,
        id: StructuralBlockId,
        input: Frame,
    ) -> Result<AnalyzedBlock, Error> {
        let block = self
            .cfg
            .block(id)
            .ok_or_else(|| Error::internal("a bytecode location has no structural block"))?;
        let final_pc = block.end_pc;
        let explicit_terminator = !matches!(block.exit, BlockExit::Fallthrough { .. });
        let mut frame = input;
        let mut operations = Vec::new();
        let mut exceptional_input = None;

        let mut pc = id.pc();
        loop {
            let instruction =
                self.executor.body.instruction_at(pc).ok_or_else(|| {
                    Error::malformed(Some(pc), MalformedBytecode::MissingInstruction)
                })?;
            if pc == final_pc {
                exceptional_input = Some(frame.clone());
                if explicit_terminator {
                    break;
                }
            }
            let operation = self
                .executor
                .lift_instruction(instruction, pc, &mut frame)
                .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation {
                operations.push((pc, operation));
            }
            if pc == final_pc {
                break;
            }
            pc = self
                .executor
                .body
                .instructions
                .next_pc_of(&pc)
                .ok_or_else(|| Error::internal_at(pc, "a block ends after decoded bytecode"))?;
        }

        let final_instruction = self.executor.body.instruction_at(final_pc).ok_or_else(|| {
            Error::malformed(Some(final_pc), MalformedBytecode::MissingInstruction)
        })?;
        let (terminator, mut edges, terminator_operation) =
            Self::lower_terminator(block, final_instruction, &mut frame)
                .map_err(|error| error.at_instruction(final_pc))?;
        if let Some(operation) = terminator_operation {
            operations.push((final_pc, operation));
        }
        let exceptional_input = exceptional_input
            .ok_or_else(|| Error::internal("a structural bytecode block has no final input"))?;
        let mut output_frames = BTreeMap::new();
        for edge in &edges {
            output_frames
                .entry(edge.target)
                .or_insert_with(|| frame.clone());
        }
        for target in &block.exceptional_successors {
            edges.push(self.exception_edge(target, &exceptional_input, &mut output_frames)?);
        }
        Ok(AnalyzedBlock {
            caught_exception: None,
            operations,
            terminator,
            terminator_source: explicit_terminator.then_some(final_pc),
            edges,
            output_frames,
        })
    }

    fn exception_edge(
        &mut self,
        target: &bytecode_cfg::ExceptionalTarget,
        input: &Frame,
        output_frames: &mut BTreeMap<Location, Frame>,
    ) -> Result<AnalyzedEdge, Error> {
        match target {
            bytecode_cfg::ExceptionalTarget::Handler { id, catch_type } => {
                let location = Location::Handler(*id);
                let caught = self.caught_exception(*id)?;
                if let Entry::Vacant(output) = output_frames.entry(location) {
                    output.insert(input.clone().exception_handler_frame(caught)?);
                }
                Ok(AnalyzedEdge {
                    target: location,
                    transfer: ControlTransfer::Exception(catch_type.clone()),
                })
            }
            bytecode_cfg::ExceptionalTarget::Unwind => {
                output_frames
                    .entry(Location::Unwind)
                    .or_insert_with(|| input.clone().into_unwind_frame());
                Ok(AnalyzedEdge {
                    target: Location::Unwind,
                    transfer: ControlTransfer::Unwind,
                })
            }
        }
    }
}
