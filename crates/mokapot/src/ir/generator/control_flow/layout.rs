//! The static partition of a decoded body into leader-delimited blocks.

use std::collections::BTreeMap;

use super::BlockExit;
use crate::{
    ir::generator::error::{Error, ErrorKind, ResultExt},
    jvm::code::{ExceptionTableEntry, MethodBody, ProgramCounter as PC},
};

pub(super) struct BlockLayout<'class> {
    entry: PC,
    blocks: BTreeMap<PC, BlockShape<'class>>,
}

pub(super) struct BlockShape<'class> {
    pub end_pc: PC,
    pub exit: BlockExit<'class, PC>,
}

impl<'class> BlockLayout<'class> {
    /// Partitions `body`, validating every decoded instruction and leader.
    pub(super) fn of(body: MethodBody<'class>) -> Result<Self, Error> {
        let mut instructions = body.instructions().iter().peekable();
        let entry = instructions
            .peek()
            .map(|&(pc, _)| pc)
            .ok_or(ErrorKind::MissingOrEmptyBody)?;

        let mut leaders = vec![entry];
        leaders.extend(body.exception_table().map(ExceptionTableEntry::handler_pc));
        let mut exits = Vec::with_capacity(instructions.len());
        while let Some((pc, instruction)) = instructions.next() {
            let next_pc = instructions.peek().map(|&(pc, _)| pc);
            let exit = BlockExit::of(body, pc, next_pc, &instruction).at_pc(pc)?;
            add_leaders(&mut leaders, &exit, next_pc);
            exits.push((pc, exit));
        }

        leaders.sort_unstable();
        leaders.dedup();
        let mut leaders = leaders.into_iter().peekable();
        let mut exits = exits.into_iter().peekable();
        let blocks = std::iter::from_fn(|| {
            let start_pc = leaders.next()?;
            let next_start = leaders.peek().copied();
            let Some(mut last) = exits.next_if(|(pc, _)| *pc == start_pc) else {
                return Some(Err(ErrorKind::MissingInstruction).at_pc(start_pc));
            };
            while let Some(next) = exits.next_if(|(pc, _)| next_start.is_none_or(|next| *pc < next))
            {
                last = next;
            }
            let (end_pc, exit) = last;
            Some(Ok((start_pc, BlockShape { end_pc, exit })))
        })
        .collect::<Result<_, _>>()?;
        Ok(Self { entry, blocks })
    }

    /// The leader of the entry block.
    pub(super) const fn entry(&self) -> PC {
        self.entry
    }

    /// Returns the block starting at `start_pc`.
    pub(super) fn block(&self, start_pc: PC) -> &BlockShape<'class> {
        self.blocks
            .get(&start_pc)
            .expect("a discovered block starts at a leader")
    }

    /// Takes the block starting at `start_pc`.
    pub(super) fn take(&mut self, start_pc: PC) -> BlockShape<'class> {
        self.blocks
            .remove(&start_pc)
            .expect("a discovered block starts at a leader")
    }
}

fn add_leaders(leaders: &mut Vec<PC>, exit: &BlockExit<'_, PC>, next_pc: Option<PC>) {
    use BlockExit::{Branch, Continue, Goto, Return, Switch, Throw};

    match exit {
        Continue { .. } | Return { .. } | Throw { .. } => {}
        Goto { target } | Branch { taken: target, .. } => {
            leaders.push(*target);
        }
        Switch { cases, default } => {
            leaders.extend(cases.iter().map(|&(_, target)| target));
            leaders.push(*default);
        }
    }
    if exit.forces_block_boundary() {
        leaders.extend(next_pc);
    }
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
        let cases = [(InstructionData::Nop, 2), (InstructionData::Return, 10)];
        for (entry, target) in cases {
            let instructions = [
                (0, entry),
                (1, InstructionData::Goto(target.into())),
                (4, InstructionData::Return),
            ];
            let method = method(instructions, "()V", vec![]);
            let body = method.as_method().body().unwrap();
            let error = BlockLayout::of(body).err().unwrap();
            assert_eq!(error.kind, ErrorKind::MissingInstruction);
            assert_eq!(error.pc, Some(target.into()));
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
