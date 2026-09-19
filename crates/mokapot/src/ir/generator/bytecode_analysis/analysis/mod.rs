//! Worklist orchestration for reachable block analysis.

mod execution;
mod merge;
mod state;

pub(super) use state::{
    CompletedAnalysis, Contribution, LiftedBlock, ParameterDefinition, ParameterSite,
};

use std::collections::{BTreeMap, BTreeSet};

use super::{Frame, Position, ValueCategory, values::ValueContext};
use crate::{
    ir::{
        BlockId, ValueId,
        generator::{
            bytecode_cfg::{NormalizedBlockKind, NormalizedCfg},
            draft::{
                DraftBlock, DraftEdge, DraftMethod, DraftOperation, DraftParameter, DraftTerminator,
            },
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
    pub(super) parameter_definitions: BTreeMap<ParameterSite, ParameterDefinition>,
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
            parameter_definitions: BTreeMap::new(),
            caught_exceptions: BTreeMap::new(),
        })
    }

    /// Analyzes every reachable normalized block to a fixed point.
    ///
    /// A block's topology never changes; only frame contributions become
    /// available. A changed input frame moves a completed block back to pending.
    pub(super) fn run(mut self) -> Result<DraftMethod, Error> {
        let entry = self.cfg.entry_block();
        self.blocks
            .get_mut(&entry)
            .expect("the normalized entry must have analysis state")
            .contributions
            .insert(Contribution::Entry, self.initial_frame.clone());
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
            let block = self.execute(block_id, input)?;
            let outputs = block
                .successors
                .edges
                .iter()
                .map(|(edge, frame)| (edge.id, edge.target, frame.clone()))
                .collect::<Vec<_>>();
            self.blocks
                .get_mut(&block_id)
                .expect("an executed block must have analysis state")
                .execution
                .complete(block);
            for (edge, target, frame) in outputs {
                debug_assert!(
                    self.cfg.block(target).predecessors.contains(&block_id),
                    "frame propagation must follow normalized topology"
                );
                self.blocks
                    .get_mut(&target)
                    .expect("a normalized successor must have analysis state")
                    .contributions
                    .insert(Contribution::Edge(edge), frame);
                if self.recompute_entry(target)? {
                    pending.insert(target);
                }
            }
        }

        CompletedAnalysis {
            blocks: self.blocks,
            parameter_definitions: self.parameter_definitions,
            receiver_value: self.values.receiver_value,
            parameter_values: self.values.parameter_values,
        }
        .into_draft(entry)
    }
}

impl CompletedAnalysis {
    fn into_draft(self, entry: BlockId) -> Result<DraftMethod, Error> {
        let Self {
            blocks,
            parameter_definitions,
            receiver_value,
            parameter_values,
        } = self;
        let mut draft_blocks = blocks
            .into_iter()
            .map(|(id, state)| {
                let lifted = state.execution.into_block().ok_or_else(|| {
                    Error::internal("a normalized reachable block was not executed")
                })?;
                Ok((id, DraftBlock::from(lifted)))
            })
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        for (site, definition) in &parameter_definitions {
            let Some(block) = draft_blocks.get_mut(&site.block) else {
                continue;
            };
            block.parameters.push(DraftParameter {
                value: definition.result,
            });
        }

        let target_parameters = draft_blocks
            .iter()
            .map(|(&id, block)| {
                (
                    id,
                    block
                        .parameters
                        .iter()
                        .map(|parameter| parameter.value)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let definitions_by_value = parameter_definitions
            .values()
            .map(|definition| (definition.result, definition))
            .collect::<BTreeMap<_, _>>();
        for block in draft_blocks.values_mut() {
            for edge in &mut block.terminator.successors {
                let parameters = target_parameters
                    .get(&edge.target)
                    .expect("a draft edge target must belong to the method");
                edge.arguments = parameters
                    .iter()
                    .map(|parameter| {
                        definitions_by_value
                            .get(parameter)
                            .and_then(|definition| definition.inputs.get(&edge.id))
                            .copied()
                            .ok_or_else(|| {
                                Error::internal("a block parameter lacks an edge argument")
                            })
                    })
                    .collect::<Result<_, _>>()?;
            }
        }

        Ok(DraftMethod {
            entry,
            blocks: draft_blocks,
            this_value: receiver_value,
            parameter_values,
        })
    }
}

impl BlockExecution {
    fn into_block(self) -> Option<LiftedBlock> {
        match self {
            Self::Complete { block, .. } => Some(block),
            Self::Uninitialized | Self::Pending { .. } => None,
        }
    }
}

impl From<LiftedBlock> for DraftBlock {
    fn from(lifted: LiftedBlock) -> Self {
        Self {
            caught_exception: lifted.caught_exception,
            parameters: Vec::new(),
            operations: lifted
                .operations
                .into_iter()
                .map(|(origin, kind)| DraftOperation {
                    kind,
                    origin: Some(origin),
                })
                .collect(),
            terminator: DraftTerminator {
                kind: lifted.terminator,
                successors: lifted
                    .successors
                    .edges
                    .into_iter()
                    .map(|(successor, _)| DraftEdge {
                        id: successor.id,
                        target: successor.target,
                        arguments: Vec::new(),
                        transfer: successor.transfer,
                    })
                    .collect(),
                origin: lifted.terminator_source,
            },
        }
    }
}
