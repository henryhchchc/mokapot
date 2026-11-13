//! The static partition of a decoded body into leader-delimited blocks.

use std::{collections::BTreeMap, ops::Range, sync::Arc};

use super::{BlockExit, ExceptionArm, ExceptionTarget};
use crate::{
    ir::generator::error::{Error, ErrorKind, ResultExt},
    jvm::code::{ExceptionTableEntry, MethodBody, ProgramCounter as PC},
};

pub(super) struct BlockLayout<'class> {
    entry: PC,
    blocks: BTreeMap<PC, BlockShape<'class>>,
}

pub(super) struct BlockShape<'class> {
    pub instruction_range: Range<usize>,
    pub exit: BlockExit<'class, PC>,
}

impl<'class> BlockLayout<'class> {
    /// Validates every instruction before forming leader-delimited spans.
    pub(super) fn of(body: MethodBody<'class>) -> Result<Self, Error> {
        let instructions = body.instructions();
        let mut iter = instructions.iter().peekable();
        let entry = iter
            .peek()
            .map(|(pc, _)| *pc)
            .ok_or(ErrorKind::MissingOrEmptyBody)?;
        let mut leaders = vec![entry];
        leaders.extend(body.exception_table().map(ExceptionTableEntry::handler_pc));
        let no_exceptions = Arc::from([]);
        let unwind = Arc::from([ExceptionArm {
            target: ExceptionTarget::Unwind,
            catch_type: None,
        }]);
        let mut pcs = Vec::with_capacity(instructions.len());
        while let Some((pc, instruction)) = iter.next() {
            let next = iter.peek().map(|(pc, _)| *pc);
            let exit = BlockExit::of(&instruction, next, &no_exceptions, &unwind).at_pc(pc)?;
            pcs.push(pc);
            exit.collect_targets(&mut leaders);
            if exit.forces_block_boundary() {
                leaders.extend(next);
            }
        }
        leaders.sort_unstable();
        leaders.dedup();
        let mut positions = pcs.iter().enumerate().peekable();
        let mut starts = Vec::with_capacity(leaders.len());
        for pc in leaders {
            while positions
                .peek()
                .is_some_and(|(_, instruction_pc)| **instruction_pc < pc)
            {
                positions.next();
            }
            match positions.peek() {
                Some((index, instruction_pc)) if **instruction_pc == pc => {
                    starts.push((pc, *index));
                }
                _ => return Err(ErrorKind::MissingInstruction).at_pc(pc),
            }
        }
        let blocks = starts
            .iter()
            .enumerate()
            .map(|(index, &(start_pc, start))| {
                let end = starts.get(index + 1).map_or(pcs.len(), |(_, index)| *index);
                let instruction_range = start..end;
                let (end_pc, instruction) = instructions
                    .index_range(instruction_range.clone())
                    .next_back()
                    .expect("a leader span is nonempty");
                let mut exit =
                    BlockExit::of(&instruction, pcs.get(end).copied(), &no_exceptions, &unwind)
                        .expect("every instruction was validated before partitioning");
                match &mut exit {
                    BlockExit::Continue { exception_arms, .. } if exception_arms.is_empty() => {}
                    BlockExit::Continue { exception_arms, .. }
                    | BlockExit::Return { exception_arms }
                    | BlockExit::Throw { exception_arms } => {
                        *exception_arms = handlers_at(body, end_pc);
                    }
                    _ => {}
                }
                (
                    start_pc,
                    BlockShape {
                        instruction_range,
                        exit,
                    },
                )
            })
            .collect();
        Ok(Self { entry, blocks })
    }

    pub(super) const fn entry(&self) -> PC {
        self.entry
    }

    pub(super) fn block(&self, start_pc: PC) -> &BlockShape<'class> {
        self.blocks
            .get(&start_pc)
            .expect("a discovered block starts at a leader")
    }
}

/// Selects handlers in exception-table order, stopping at the first catch-all.
fn handlers_at(body: MethodBody<'_>, pc: PC) -> Arc<[ExceptionArm<'_, PC>]> {
    let mut arms = Vec::new();
    for entry in body.exception_table().filter(|entry| entry.covers(pc)) {
        arms.push(ExceptionArm {
            target: ExceptionTarget::Handler(entry.handler_pc()),
            catch_type: entry.catch_type(),
        });
        if entry.catches_all() {
            return arms.into();
        }
    }
    arms.push(ExceptionArm {
        target: ExceptionTarget::Unwind,
        catch_type: None,
    });
    arms.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test::prelude::*;

    #[test]
    fn rejects_empty_bodies() {
        let method = method(std::iter::empty::<(u16, InstructionData)>(), "()V", vec![]);
        let body = method.as_method().body().unwrap();
        let error = BlockLayout::of(body).err().unwrap();
        assert_eq!(error.kind, ErrorKind::MissingOrEmptyBody);
    }

    #[test]
    fn rejects_missing_jump_targets_in_reachable_and_unreachable_code() {
        for entry in [InstructionData::Nop, InstructionData::Return] {
            let instructions = [(0, entry), (1, InstructionData::Goto(10.into()))];
            let method = method(instructions, "()V", vec![]);
            let body = method.as_method().body().unwrap();
            let error = BlockLayout::of(body).err().unwrap();
            assert_eq!(error.kind, ErrorKind::MissingInstruction);
        }
    }

    #[test]
    fn validates_unreachable_instructions() {
        let instructions = [(0, InstructionData::Return), (1, InstructionData::Nop)];
        let method = method(instructions, "()V", vec![]);
        let body = method.as_method().body().unwrap();
        let error = BlockLayout::of(body).err().unwrap();
        assert_eq!(error.kind, ErrorKind::MissingFallthrough);
    }
}
