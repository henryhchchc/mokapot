//! Lifting and successor construction for one reachable structural block.

mod terminator;

use std::collections::{BTreeMap, btree_map::Entry};

use super::{
    analyzer::Analyzer,
    model::{AnalyzedBlock, AnalyzedEdge, Frame, Location},
};
use crate::{
    ir::generator::{
        bytecode_cfg::{self, StructuralBlockId, StructuralTerminator},
        error::{Error, MalformedBytecode},
    },
    ir::{TerminatorKind, control_flow::ControlTransfer},
};

impl Analyzer<'_, '_> {
    /// Executes one location, producing its analyzed block.
    ///
    /// Precondition: a location's successor *target set* is a pure function of
    /// the location. The targets come only from a structural block's terminator
    /// and exception table, never from `input`: the frames differ between
    /// executions of one location, the targets never do. `Analyzer::run` relies
    /// on this to treat predecessor sets as monotonically growing.
    pub(super) fn execute(
        &mut self,
        location: Location,
        input: Frame,
    ) -> Result<AnalyzedBlock, Error> {
        match location {
            Location::Bytecode(id) => self.execute_bytecode(id, input),
            Location::Handler(id) => self.execute_handler(id, input),
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

    fn execute_handler(
        &mut self,
        id: bytecode_cfg::HandlerId,
        input: Frame,
    ) -> Result<AnalyzedBlock, Error> {
        let handler = self
            .cfg
            .handler(id)
            .ok_or_else(|| Error::internal("a handler location has no structural entry"))?;
        let caught = *input.handler_exception().map_err(Error::from)?;
        let target = Location::Bytecode(handler.target);
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
        let final_pc = *block
            .instruction_pcs
            .last()
            .ok_or_else(|| Error::internal("a structural bytecode block is empty"))?;
        let explicit_terminator =
            !matches!(block.terminator, StructuralTerminator::Fallthrough { .. });
        let mut frame = input;
        let mut operations = Vec::new();
        let mut exceptional_input = None;

        for &pc in &block.instruction_pcs {
            if pc == final_pc {
                exceptional_input = Some(frame.clone());
                if explicit_terminator {
                    continue;
                }
            }
            let instruction =
                self.executor.body.instruction_at(pc).ok_or_else(|| {
                    Error::malformed(Some(pc), MalformedBytecode::MissingInstruction)
                })?;
            let operation = self
                .executor
                .lift_instruction(instruction, pc, &mut frame)
                .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation {
                operations.push((pc, operation));
            }
        }

        let (terminator, mut edges, terminator_operation) =
            Self::lower_terminator(block, &mut frame)
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
