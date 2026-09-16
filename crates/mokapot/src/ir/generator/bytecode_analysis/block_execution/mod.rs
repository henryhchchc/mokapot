//! Lifting and successor construction for one reachable structural block.

mod terminator;

use std::collections::BTreeMap;

use super::{
    FrameValue,
    analyzer::Analyzer,
    model::{AnalyzedBlock, AnalyzedSuccessor, Frame, Location},
};
use crate::{
    ir::generator::{
        bytecode_cfg::{self, StructuralBlockId, StructuralTerminator},
        error::{Error, MalformedBytecode},
    },
    ir::{TerminatorKind, control_flow::ControlTransfer},
};

impl Analyzer<'_, '_> {
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
                successors: Vec::new(),
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
        let caught = input
            .handler_exception()
            .map_err(Error::from)
            .and_then(|value| value.into_ssa_value_id())?;
        Ok(AnalyzedBlock {
            caught_exception: Some(caught),
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            successors: vec![AnalyzedSuccessor {
                target: Location::Bytecode(handler.target),
                transfer: ControlTransfer::Unconditional,
                frame: input,
            }],
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
            .ok_or_else(|| Error::internal("a bytecode location has no structural block"))?
            .clone();
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
            let instruction = self
                .executor
                .body
                .instruction_at(pc)
                .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingInstruction))?
                .clone();
            let operation = self
                .executor
                .lift_instruction(&instruction, pc, &mut frame)
                .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation {
                operations.push((pc, operation));
            }
        }

        let (terminator, mut successors, terminator_operation) = self
            .lower_terminator(&block, &mut frame, final_pc)
            .map_err(|error| error.at_instruction(final_pc))?;
        if let Some(operation) = terminator_operation {
            operations.push((final_pc, operation));
        }
        let exceptional_input = exceptional_input
            .ok_or_else(|| Error::internal("a structural bytecode block has no final input"))?;
        for target in &block.exceptional_successors {
            successors.push(self.exception_successor(*target, &exceptional_input)?);
        }
        Ok(AnalyzedBlock {
            caught_exception: None,
            operations,
            terminator,
            terminator_source: explicit_terminator.then_some(final_pc),
            successors,
        })
    }

    fn exception_successor(
        &mut self,
        target: bytecode_cfg::ExceptionalTarget,
        input: &Frame,
    ) -> Result<AnalyzedSuccessor, Error> {
        match target {
            bytecode_cfg::ExceptionalTarget::Handler(id) => {
                let handler = self
                    .cfg
                    .handler(id)
                    .ok_or_else(|| Error::internal("an exception edge has no handler entry"))?;
                let caught = if let Some(&value) = self.caught_exceptions.get(&id) {
                    value
                } else {
                    let value = self.executor.new_value_id()?;
                    self.caught_exceptions.insert(id, value);
                    value
                };
                Ok(AnalyzedSuccessor {
                    target: Location::Handler(id),
                    transfer: ControlTransfer::Exception(handler.catch_type.clone()),
                    frame: input
                        .clone()
                        .exception_handler_frame(FrameValue::Ordinary(caught))?,
                })
            }
            bytecode_cfg::ExceptionalTarget::Unwind => Ok(AnalyzedSuccessor {
                target: Location::Unwind,
                transfer: ControlTransfer::Unwind,
                frame: input.clone().into_unwind_frame(),
            }),
        }
    }

    pub(super) fn coalesce_output_frames(
        successors: &[AnalyzedSuccessor],
    ) -> Result<BTreeMap<Location, Frame>, Error> {
        let mut outputs = BTreeMap::new();
        for successor in successors {
            if let Some(existing) = outputs.insert(successor.target, successor.frame.clone())
                && existing != successor.frame
            {
                return Err(Error::internal(
                    "parallel edges from one block carry different JVM frames",
                ));
            }
        }
        Ok(outputs)
    }
}
