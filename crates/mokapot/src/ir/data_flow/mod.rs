//! Data flow analysis.

use std::{collections::BTreeSet, iter::once};

use itertools::Itertools;

use super::{DefUseChain, Identifier, InstructionId, MokaIRMethod, ValueId};

impl<'a> DefUseChain<'a> {
    /// Create a new def-use graph from a method.
    #[must_use]
    pub fn new(method: &'a MokaIRMethod) -> Self {
        let defs = method
            .blocks()
            .flat_map(super::BasicBlock::instructions)
            .filter_map(|instruction| instruction.def().map(|value| (value, instruction.id())))
            .collect();
        let uses = method
            .blocks()
            .flat_map(|block| {
                block
                    .instructions()
                    .iter()
                    .map(|instruction| (instruction.id(), instruction.uses()))
                    .chain(once((block.terminator().id(), block.terminator().uses())))
            })
            .flat_map(|(location, uses)| uses.into_iter().map(move |id| (id, location)))
            .into_group_map()
            .into_iter()
            .map(|(id, uses)| (id, uses.into_iter().collect()))
            .collect();
        Self { method, defs, uses }
    }

    /// Get the location where an identifier is defined.
    #[must_use]
    pub fn defined_at(&self, value: ValueId) -> Option<InstructionId> {
        self.defs.get(&value).copied()
    }

    /// Get the locations where an identifier is used.
    #[must_use]
    pub fn used_at(&self, id: Identifier) -> BTreeSet<InstructionId> {
        self.uses.get(&id).cloned().unwrap_or_default()
    }
}
