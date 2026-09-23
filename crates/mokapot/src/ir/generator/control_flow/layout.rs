//! The static partition of a decoded body into leader-delimited blocks.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Bound,
};

use super::{BlockExit, Error};
use crate::{
    ir::generator::error::MalformedControlFlow,
    jvm::code::{MethodBody, ProgramCounter as PC},
};

pub(super) struct BlockLayout {
    entry: PC,
    blocks: BTreeMap<PC, BlockShape>,
}

pub(super) struct BlockShape {
    pub end_pc: PC,
    pub exit: BlockExit<PC>,
}

impl BlockLayout {
    /// Partitions `body`, validating every decoded instruction and leader.
    pub(super) fn of(body: &MethodBody) -> Result<Self, Error> {
        let entry = body
            .instructions
            .entry_point()
            .map(|(pc, _)| pc)
            .ok_or(Error::MissingOrEmptyBody)?;
        let mut exits = body
            .instructions
            .iter()
            .map(|(pc, instruction)| BlockExit::of(body, pc, instruction).map(|exit| (pc, exit)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let leaders = leaders(body, entry, &exits)?;
        let last_pc = exits
            .last_key_value()
            .map(|(pc, _)| *pc)
            .expect("a body with an entry point has instructions");

        let blocks = leaders
            .iter()
            .copied()
            .map(|start_pc| {
                let end_pc = block_end(body, &leaders, last_pc, start_pc);
                let exit = exits
                    .remove(&end_pc)
                    .expect("a block's final instruction is decoded");
                (start_pc, BlockShape { end_pc, exit })
            })
            .collect();
        Ok(Self { entry, blocks })
    }

    /// The leader of the entry block.
    pub(super) const fn entry(&self) -> PC {
        self.entry
    }

    /// Returns the block starting at `start_pc`.
    pub(super) fn block(&self, start_pc: PC) -> &BlockShape {
        self.blocks
            .get(&start_pc)
            .expect("a discovered block starts at a leader")
    }

    /// Takes the block starting at `start_pc`.
    pub(super) fn take(&mut self, start_pc: PC) -> BlockShape {
        self.blocks
            .remove(&start_pc)
            .expect("a discovered block starts at a leader")
    }
}

/// Collects every block leader, checking each names a decoded instruction.
fn leaders(
    body: &MethodBody,
    entry: PC,
    exits: &BTreeMap<PC, BlockExit<PC>>,
) -> Result<BTreeSet<PC>, Error> {
    use BlockExit::{Branch, Continue, Goto, Return, Switch, Throw};

    let mut leaders = BTreeSet::from([entry]);
    leaders.extend(body.exception_table.iter().map(|it| it.handler_pc));

    for (&pc, exit) in exits {
        match exit {
            Continue { .. } | Return { .. } | Throw { .. } => {}
            Goto { target } => {
                leaders.insert(*target);
            }
            Branch { taken, otherwise } => {
                leaders.insert(*taken);
                leaders.insert(*otherwise);
            }
            Switch { cases, default } => {
                leaders.extend(cases.values());
                leaders.insert(*default);
            }
        }
        if exit.forces_block_boundary() {
            leaders.extend(body.instructions.next_pc_of(&pc));
        }
    }

    if let Some(&pc) = leaders
        .iter()
        .find(|pc| body.instruction_at(**pc).is_none())
    {
        return Err(MalformedControlFlow::MissingInstruction(pc).into());
    }
    Ok(leaders)
}

/// The final instruction of the block starting at `start_pc`.
///
/// A block spans from its leader up to the instruction before the next leader,
/// so the leaders alone determine every block span.
fn block_end(body: &MethodBody, leaders: &BTreeSet<PC>, last_pc: PC, start_pc: PC) -> PC {
    leaders
        .range((Bound::Excluded(start_pc), Bound::Unbounded))
        .next()
        .map_or(last_pc, |&next| {
            body.instructions
                .prev_pc_of(&next)
                .expect("a later leader is preceded by the previous leader")
        })
}
