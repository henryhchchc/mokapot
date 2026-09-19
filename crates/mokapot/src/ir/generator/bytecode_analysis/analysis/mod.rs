//! Worklist orchestration for reachable block analysis.

mod execution;
mod merge;
mod state;

use state::{
    BlockExecution, BlockState, CompletedAnalysis, Contribution, LiftedArm, LiftedBlock,
    LiftedTerminator, ParameterSite,
};

use std::collections::{HashMap, HashSet, VecDeque};

use super::{Frame, Position, ValueCategory, values::ValueContext};
use crate::{
    ir::{
        BasicBlock, BlockId, BlockParameter, SourceMap, Successor, Terminator, ValueId,
        generator::{
            bytecode_cfg::{ArmId, Block, Cfg},
            draft::DraftMethod,
            error::Error,
        },
    },
    jvm::code::ProgramCounter,
};

pub(super) struct Analyzer<'method, 'cfg> {
    pub(super) cfg: &'cfg Cfg<'method>,
    pub(super) values: ValueContext,
    pub(super) initial_frame: Frame,
    blocks: HashMap<BlockId, BlockState>,
    parameter_definitions: HashMap<ParameterSite, ValueId>,
    pub(super) caught_exceptions: HashMap<BlockId, ValueId>,
}

impl<'method, 'cfg> Analyzer<'method, 'cfg> {
    /// Interns the identity of the value caught by handler-entry `block`.
    ///
    /// Every exceptional arm into one handler-entry location must carry the
    /// *same* value: distinct values would create a parameter at stack position 0
    /// in the handler's entry frame, which must hold one caught value.
    pub(super) fn caught_exception(&mut self, block: BlockId) -> ValueId {
        if let Some(&value) = self.caught_exceptions.get(&block) {
            return value;
        }
        let value = self.values.fresh();
        self.caught_exceptions.insert(block, value);
        value
    }

    /// The PC to attribute a diagnostic for `block` to, if it has one.
    ///
    /// A bytecode block points at its first instruction, while a synthetic
    /// landing pad has no location.
    pub(super) fn block_pc(&self, block: BlockId) -> Option<ProgramCounter> {
        match self.cfg.block(block) {
            Block::Bytecode { start_pc, .. } => Some(*start_pc),
            Block::HandlerEntry { .. } => None,
        }
    }

    pub(super) fn new(cfg: &'cfg Cfg<'method>) -> Result<Self, Error> {
        let (values, initial_frame) = ValueContext::for_cfg(cfg)?;
        let blocks = cfg
            .blocks
            .keys()
            .map(|id| (*id, BlockState::default()))
            .collect();
        Ok(Self {
            cfg,
            values,
            initial_frame,
            blocks,
            parameter_definitions: HashMap::new(),
            caught_exceptions: HashMap::new(),
        })
    }

    /// Analyzes every reachable block to a fixed point.
    ///
    /// A block's topology never changes; only frame contributions become
    /// available. A changed input frame moves a completed block back to pending.
    pub(super) fn run(mut self) -> Result<DraftMethod, Error> {
        let entry = self.cfg.entry_block();
        self.blocks
            .get_mut(&entry)
            .expect("the entry block must have analysis state")
            .contributions
            .insert(Contribution::Entry, self.initial_frame.clone());
        self.recompute_entry(entry)?;

        let mut pending = VecDeque::from([entry]);
        let mut queued = HashSet::from([entry]);
        while let Some(block_id) = pending.pop_front() {
            queued.remove(&block_id);
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
                .filter_map(|lifted| match lifted {
                    LiftedArm::Block {
                        arm, target, frame, ..
                    } => Some((*arm, *target, frame.clone())),
                    LiftedArm::Unwind { .. } => None,
                })
                .collect::<Vec<_>>();
            self.blocks
                .get_mut(&block_id)
                .expect("an executed block must have analysis state")
                .execution
                .complete(block);
            for (arm, target, frame) in outputs {
                self.blocks
                    .get_mut(&target)
                    .expect("a successor must have analysis state")
                    .contributions
                    .insert(
                        Contribution::Edge {
                            source: block_id,
                            arm,
                        },
                        frame,
                    );
                if self.recompute_entry(target)? && queued.insert(target) {
                    pending.push_back(target);
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
        // Keep this index local to materialization; contribution frames remain
        // the only source of incoming argument values during analysis.
        let mut parameters_by_block = parameter_definitions.iter().fold(
            HashMap::<BlockId, Vec<(ParameterSite, ValueId)>>::new(),
            |mut parameters, (&site, &value)| {
                parameters
                    .entry(site.block)
                    .or_default()
                    .push((site, value));
                parameters
            },
        );
        for parameters in parameters_by_block.values_mut() {
            parameters.sort_by_key(|(site, _)| site.position);
        }

        let entry_arguments = parameters_by_block
            .get(&entry)
            .into_iter()
            .flatten()
            .map(|(site, _)| contribution_value(&lifted_blocks, entry, Contribution::Entry, *site))
            .collect::<Result<Vec<_>, _>>()?;
        // Derive each arm's final arguments before consuming the analysis
        // states. This view is discarded after successor construction.
        let mut edge_arguments = lifted_blocks
            .iter()
            .flat_map(|(&source, state)| {
                let BlockExecution::Complete { block, .. } = &state.execution else {
                    unreachable!("a reachable block was not executed");
                };
                block
                    .terminator
                    .arms()
                    .filter_map(move |lifted| match lifted {
                        LiftedArm::Block { arm, target, .. } => Some((source, *arm, *target)),
                        LiftedArm::Unwind { .. } => None,
                    })
            })
            .map(|(source, arm, target)| {
                let arguments = parameters_by_block
                    .get(&target)
                    .into_iter()
                    .flatten()
                    .map(|(site, _)| {
                        contribution_value(
                            &lifted_blocks,
                            target,
                            Contribution::Edge { source, arm },
                            *site,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(((source, arm), arguments))
            })
            .collect::<Result<HashMap<_, _>, Error>>()?;
        let mut source_map = SourceMap::default();
        let mut blocks = lifted_blocks
            .into_iter()
            .map(|(id, state)| {
                let lifted = state
                    .execution
                    .into_block()
                    .ok_or_else(|| Error::internal("a reachable block was not executed"))?;
                Ok((
                    id,
                    materialize_block(id, lifted, &mut edge_arguments, &mut source_map)?,
                ))
            })
            .collect::<Result<HashMap<_, _>, Error>>()?;

        for (site, value) in &parameter_definitions {
            let Some(block) = blocks.get_mut(&site.block) else {
                continue;
            };
            block.parameters.push(BlockParameter { value: *value });
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

fn contribution_value(
    blocks: &HashMap<BlockId, BlockState>,
    block: BlockId,
    contribution: Contribution,
    site: ParameterSite,
) -> Result<ValueId, Error> {
    blocks
        .get(&block)
        .and_then(|state| state.contributions.get(&contribution))
        .and_then(|frame| frame.value_at(site.position))
        .copied()
        .ok_or_else(|| Error::internal("a block parameter lacks its contribution value"))
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
    edge_arguments: &mut HashMap<(BlockId, ArmId), Vec<ValueId>>,
    source_map: &mut SourceMap,
) -> Result<BasicBlock, Error> {
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
    Ok(BasicBlock {
        kind: lifted.kind,
        parameters: Vec::new(),
        operations,
        terminator: materialize_terminator(id, lifted.terminator, edge_arguments)?,
    })
}

/// Converts a lifted terminator into its completed form, taking each block arm's
/// arguments from the arm it was lifted from.
fn materialize_terminator(
    source: BlockId,
    terminator: LiftedTerminator,
    edge_arguments: &mut HashMap<(BlockId, ArmId), Vec<ValueId>>,
) -> Result<Terminator<Successor>, Error> {
    terminator.try_map_arms(|lifted| match lifted {
        LiftedArm::Block {
            arm,
            target,
            transfer,
            ..
        } => Ok(Successor::Block {
            target,
            arguments: edge_arguments
                .remove(&(source, arm))
                .ok_or_else(|| Error::internal("a block successor lacks its arguments"))?,
            transfer,
        }),
        LiftedArm::Unwind { .. } => Ok(Successor::Unwind),
    })
}
