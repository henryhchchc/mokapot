//! Worklist orchestration for reachable block analysis.

mod execution;
mod merge;
mod state;

pub(super) use state::{CompletedAnalysis, LiftedBlock, PhiDefinition, PhiSite, Predecessor};

use std::collections::{BTreeMap, BTreeSet};

use super::{Frame, Position, ScalarGraph, ValueCategory, output, values::ValueContext};
use crate::{
    ir::{
        BlockId, SourceMap, ValueId,
        generator::{
            bytecode_cfg::{NormalizedBlockKind, NormalizedCfg},
            error::Error,
        },
    },
    jvm::code::ProgramCounter,
};
use state::{BlockExecution, BlockState, LiftedEdge, LiftedSuccessors};

pub(super) struct Analyzer<'method, 'cfg> {
    pub(super) cfg: &'cfg NormalizedCfg<'method>,
    pub(super) values: ValueContext,
    pub(super) initial_frame: Frame,
    pub(super) blocks: BTreeMap<BlockId, BlockState>,
    pub(super) phi_definitions: BTreeMap<PhiSite, PhiDefinition>,
    pub(super) caught_exceptions: BTreeMap<BlockId, ValueId>,
}

impl Analyzer<'_, '_> {
    /// Interns the identity of the value caught by handler-entry `block`.
    ///
    /// Every exceptional arm into one handler-entry location must carry the
    /// *same* value: distinct values would merge into a phi at stack position 0
    /// in the handler's entry frame, which must hold one caught value.
    pub(super) fn caught_exception(&mut self, block: BlockId) -> Result<ValueId, Error> {
        if let Some(&value) = self.caught_exceptions.get(&block) {
            return Ok(value);
        }
        let value = self.values.fresh()?;
        self.caught_exceptions.insert(block, value);
        Ok(value)
    }

    /// The PC to attribute a diagnostic for `block` to, if it has one.
    ///
    /// A bytecode and its handler entry both point at the bytecode block's
    /// start; the synthetic entry and unwind blocks have no PC.
    pub(super) fn block_pc(&self, block: BlockId) -> Option<ProgramCounter> {
        match self.cfg.block(block).kind {
            NormalizedBlockKind::Bytecode(id) | NormalizedBlockKind::HandlerEntry(id) => {
                Some(self.cfg.bytecode_block(id).start_pc)
            }
            NormalizedBlockKind::EntryPreheader | NormalizedBlockKind::Unwind => None,
        }
    }
}

impl<'method, 'cfg> Analyzer<'method, 'cfg> {
    pub(super) fn new(cfg: &'cfg NormalizedCfg<'method>) -> Result<Self, Error> {
        let (values, initial_frame) = ValueContext::for_cfg(cfg)?;
        let blocks = cfg
            .blocks()
            .map(|(id, _)| (id, BlockState::default()))
            .collect();
        Ok(Self {
            cfg,
            values,
            initial_frame,
            blocks,
            phi_definitions: BTreeMap::new(),
            caught_exceptions: BTreeMap::new(),
        })
    }

    /// Analyzes every reachable normalized block to a fixed point.
    ///
    /// A block's topology never changes; only frame contributions become
    /// available. A changed input frame moves a completed block back to pending.
    pub(super) fn run(mut self) -> Result<(ScalarGraph, SourceMap), Error> {
        let entry = self.cfg.entry_block();
        self.blocks
            .get_mut(&entry)
            .expect("the normalized entry must have analysis state")
            .contributions
            .insert(Predecessor::Entry, self.initial_frame.clone());
        self.recompute_entry(entry)?;

        let mut pending = BTreeSet::from([entry]);
        while let Some(block_id) = pending.pop_first() {
            let state = self
                .blocks
                .get(&block_id)
                .expect("a worklist block must have analysis state");
            let BlockExecution::Pending { input } = &state.execution else {
                unreachable!("a worklist block must be pending");
            };
            let input = input.clone();
            let mut block = self.execute(block_id, input)?;
            let outputs = block.successors.take_output_frames();
            self.blocks
                .get_mut(&block_id)
                .expect("an executed block must have analysis state")
                .execution
                .complete(block);
            for (target, frame) in outputs {
                debug_assert!(
                    self.cfg.block(target).predecessors.contains(&block_id),
                    "frame propagation must follow normalized topology"
                );
                self.blocks
                    .get_mut(&target)
                    .expect("a normalized successor must have analysis state")
                    .contributions
                    .insert(Predecessor::Block(block_id), frame);
                if self.recompute_entry(target)? {
                    pending.insert(target);
                }
            }
        }

        let completed = CompletedAnalysis {
            blocks: self.blocks,
            phi_definitions: self.phi_definitions,
            receiver_value: self.values.receiver_value,
            parameter_values: self.values.parameter_values,
        };
        output::materialize(completed, entry)
    }
}
