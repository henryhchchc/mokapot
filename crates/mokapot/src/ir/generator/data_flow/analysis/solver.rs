//! Worklist orchestration for reachable block analysis.

use std::collections::{HashMap, HashSet, VecDeque, hash_map::Entry};

use super::{
    DataflowParts, ValueContext, block_state::BlockState, execution::BlockInterpreter,
    frame_block::FrameSource,
};
use crate::ir::{
    BlockId,
    generator::{control_flow::Cfg, error::Error},
};

pub(crate) struct DataflowSolver<'method, 'cfg> {
    interpreter: BlockInterpreter<'method, 'cfg>,
    values: ValueContext,
    blocks: HashMap<BlockId, BlockState>,
}

impl<'method, 'cfg> DataflowSolver<'method, 'cfg> {
    pub(crate) fn new(cfg: &'cfg Cfg<'method>) -> Result<Self, Error> {
        let (values, initial_frame) = ValueContext::for_cfg(cfg)?;
        let interpreter = BlockInterpreter::new(cfg);
        let entry = interpreter.entry_block();
        let entry_state = BlockState::new(FrameSource::Entry, initial_frame);
        Ok(Self {
            interpreter,
            values,
            blocks: HashMap::from([(entry, entry_state)]),
        })
    }

    pub(crate) fn solve(mut self) -> Result<DataflowParts, Error> {
        let entry = self.interpreter.entry_block();
        let mut worklist = Worklist::default();
        worklist.schedule(entry);
        while let Some(block_id) = worklist.pop() {
            let state = self
                .blocks
                .get_mut(&block_id)
                .expect("every scheduled block has analysis state");

            let input = state.input();
            let block = self
                .interpreter
                .interpret(&mut self.values, block_id, input)?;
            let outputs = block
                .outgoing_frames(block_id)
                .map(|(source, target, frame)| (source, target, frame.clone()))
                .collect::<Vec<_>>();
            state.complete(block);

            for (source, target, frame) in outputs {
                let input_changed = match self.blocks.entry(target) {
                    Entry::Occupied(mut entry) => {
                        let block_pc = self.interpreter.block_pc(target);
                        entry
                            .get_mut()
                            .add_frame(source, frame, block_pc, &mut self.values)?
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(BlockState::new(source, frame));
                        true
                    }
                };
                if input_changed {
                    worklist.schedule(target);
                }
            }
        }

        let blocks = self
            .blocks
            .into_iter()
            .map(|(id, block)| (id, block.into_solution()))
            .collect();
        let (receiver_value, parameter_values) = self.values.into_method_values();
        Ok(DataflowParts {
            entry,
            blocks,
            this_value: receiver_value,
            parameter_values,
        })
    }
}

/// A FIFO worklist that schedules each block at most once while queued.
#[derive(Default)]
struct Worklist {
    pending: VecDeque<BlockId>,
    queued: HashSet<BlockId>,
}

impl Worklist {
    fn schedule(&mut self, block: BlockId) {
        if self.queued.insert(block) {
            self.pending.push_back(block);
        }
    }

    fn pop(&mut self) -> Option<BlockId> {
        let block = self.pending.pop_front()?;
        self.queued.remove(&block);
        Some(block)
    }
}
