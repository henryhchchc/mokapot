//! Worklist orchestration for reachable block analysis.

mod execution;
mod merge;
mod state;

pub(super) use state::{
    CompletedAnalysis, Contribution, LiftedArm, LiftedBlock, LiftedTerminator, ParameterDefinition,
    ParameterSite,
};

use std::collections::{BTreeMap, BTreeSet};

use super::{Frame, Position, ValueCategory, values::ValueContext};
use crate::{
    ir::{
        BasicBlock, BlockId, BlockParameter, SourceMap, Successor, SuccessorTarget, Terminator,
        ValueId,
        generator::{
            bytecode_cfg::{NormalizedBlockKind, NormalizedCfg},
            draft::DraftMethod,
            error::Error,
        },
    },
    jvm::code::ProgramCounter,
};
use state::{BlockExecution, BlockState, LiftedEdge};

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
    /// *same* value: distinct values would create a parameter at stack position 0
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
    /// start.
    pub(super) fn block_pc(&self, block: BlockId) -> ProgramCounter {
        match self.cfg.block(block).kind {
            NormalizedBlockKind::Bytecode(id) | NormalizedBlockKind::HandlerEntry(id) => {
                self.cfg.bytecode_block(id).start_pc
            }
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
                .terminator
                .arms()
                .filter_map(|(edge, frame)| match (edge.target, frame) {
                    (SuccessorTarget::Block(target), Some(frame)) => {
                        Some((edge.id, target, frame.clone()))
                    }
                    (SuccessorTarget::Unwind, None) => None,
                    _ => unreachable!("only block successors contribute frames"),
                })
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
            blocks: lifted_blocks,
            parameter_definitions,
            receiver_value: this_value,
            parameter_values,
        } = self;
        let mut source_map = SourceMap::default();
        let mut blocks = lifted_blocks
            .into_iter()
            .map(|(id, state)| {
                let lifted = state.execution.into_block().ok_or_else(|| {
                    Error::internal("a normalized reachable block was not executed")
                })?;
                Ok((id, materialize_block(id, lifted, &mut source_map)))
            })
            .collect::<Result<BTreeMap<_, _>, Error>>()?;

        for (site, definition) in &parameter_definitions {
            let Some(block) = blocks.get_mut(&site.block) else {
                continue;
            };
            block.parameters.push(BlockParameter {
                value: definition.result,
            });
        }

        let target_parameters = blocks
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
        let entry_arguments = target_parameters[&entry]
            .iter()
            .map(|parameter| {
                definitions_by_value
                    .get(parameter)
                    .and_then(|definition| definition.inputs.get(&Contribution::Entry))
                    .copied()
                    .ok_or_else(|| Error::internal("an entry parameter lacks an entry argument"))
            })
            .collect::<Result<_, _>>()?;
        for block in blocks.values_mut() {
            for edge in block.terminator.arms_mut() {
                edge.arguments = match edge.target {
                    SuccessorTarget::Block(target) => target_parameters[&target]
                        .iter()
                        .map(|parameter| {
                            definitions_by_value
                                .get(parameter)
                                .and_then(|def| def.inputs.get(&Contribution::Edge(edge.id)))
                                .copied()
                                .expect("a block parameter must have an edge argument")
                        })
                        .collect(),
                    SuccessorTarget::Unwind => Vec::new(),
                };
            }
        }

        Ok(DraftMethod {
            entry,
            entry_arguments,
            blocks,
            source_map,
            this_value,
            parameter_values,
        })
    }
}

impl BlockExecution {
    fn into_block(self) -> Option<LiftedBlock> {
        match self {
            Self::Complete { block, .. } => Some(*block),
            Self::Uninitialized | Self::Pending { .. } => None,
        }
    }
}

/// Materializes the final addressable draft structure and its detached provenance.
///
/// Later phases may rewrite values, but must not insert, remove, or reorder
/// operations or terminators because their source locations are fixed here.
fn materialize_block(
    id: BlockId,
    lifted: LiftedBlock,
    source_map: &mut SourceMap,
) -> BasicBlock<crate::ir::OperationKind> {
    if let Some(origin) = lifted.terminator_source {
        source_map.record_terminator(origin, id);
    }
    let operations = lifted
        .operations
        .into_iter()
        .enumerate()
        .map(|(index, (origin, kind))| {
            source_map.record_operation(origin, id, index);
            kind
        })
        .collect();
    BasicBlock {
        kind: lifted.kind,
        parameters: Vec::new(),
        operations,
        terminator: lifted.terminator.into(),
    }
}

impl From<LiftedEdge> for Successor {
    fn from(edge: LiftedEdge) -> Self {
        Self {
            id: edge.id,
            target: edge.target,
            arguments: Vec::new(),
            transfer: edge.transfer,
        }
    }
}

impl From<state::LiftedTerminator> for Terminator<Successor, crate::ir::OperationKind> {
    fn from(value: state::LiftedTerminator) -> Self {
        value.map_arms(|(edge, _)| edge.into())
    }
}
