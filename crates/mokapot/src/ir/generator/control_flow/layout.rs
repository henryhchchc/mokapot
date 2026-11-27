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
        let exits = body
            .instructions()
            .iter_with_next_pc()
            .map(|(pc, instruction, next_pc)| {
                BlockExit::of(body, pc, next_pc, &instruction)
                    .at_pc(pc)
                    .map(|exit| (pc, exit))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let entry = exits
            .first()
            .map(|&(pc, _)| pc)
            .ok_or(ErrorKind::MissingOrEmptyBody)?;

        let next_pcs = exits.iter().skip(1).map(|&(pc, _)| Some(pc)).chain([None]);
        let mut leaders: Vec<_> = exits
            .iter()
            .zip(next_pcs)
            .flat_map(|((_, exit), next_pc)| leader_pcs(exit, next_pc))
            .chain([entry])
            .chain(body.exception_table().map(ExceptionTableEntry::handler_pc))
            .collect();
        leaders.sort_unstable();
        leaders.dedup();

        let bounds = leaders
            .array_windows::<2>()
            .map(|&[start_pc, next_start]| (start_pc, Some(next_start)))
            .chain(leaders.last().copied().map(|start_pc| (start_pc, None)));
        let blocks = bounds
            .scan(
                exits.into_iter().peekable(),
                |exits, (start_pc, next_start)| {
                    let Some(first) = exits.next_if(|(pc, _)| *pc == start_pc) else {
                        return Some(Err(ErrorKind::MissingInstruction).at_pc(start_pc));
                    };
                    let rest = std::iter::from_fn(|| {
                        exits.next_if(|(pc, _)| next_start.is_none_or(|next| *pc < next))
                    });
                    let (end_pc, exit) = rest.last().unwrap_or(first);
                    Some(Ok((start_pc, BlockShape { end_pc, exit })))
                },
            )
            .collect::<Result<_, _>>()?;
        Ok(Self { entry, blocks })
    }

    /// The leader of the entry block.
    pub(super) const fn entry(&self) -> PC {
        self.entry
    }

    /// Takes the block starting at `start_pc`.
    pub(super) fn take(&mut self, start_pc: PC) -> BlockShape<'class> {
        self.blocks
            .remove(&start_pc)
            .expect("a discovered block starts at a leader")
    }
}

fn leader_pcs<'exit>(
    exit: &'exit BlockExit<'_, PC>,
    next_pc: Option<PC>,
) -> impl Iterator<Item = PC> + 'exit {
    use BlockExit::{Branch, Continue, Goto, Return, Switch, Throw};

    let (cases, target): (&[(i32, PC)], _) = match exit {
        Continue { .. } | Return { .. } | Throw { .. } => (&[], None),
        Goto { target } | Branch { taken: target, .. } => (&[], Some(*target)),
        Switch { cases, default } => (cases, Some(*default)),
    };
    let next_leader = next_pc.filter(|_| exit.forces_block_boundary());
    cases
        .iter()
        .map(|&(_, target)| target)
        .chain(target)
        .chain(next_leader)
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
