//! Data flow analysis.

use std::collections::{BTreeSet, HashMap};

use itertools::Itertools;

use crate::jvm::code::ProgramCounter;

use super::{Identifier, LocalValue, MokaIRMethod};

/// A def-use chain in data flow analysis.
#[derive(Debug)]
pub struct DefUseChain {
    defs: HashMap<LocalValue, ProgramCounter>,
    uses: HashMap<Identifier, BTreeSet<ProgramCounter>>,
}

impl DefUseChain {
    /// Create a new def-use graph from a method.
    #[must_use]
    pub fn new(method: &MokaIRMethod) -> Self {
        let defs = method
            .instructions
            .iter()
            .filter_map(|(pc, insn)| insn.def().map(|it| (it, *pc)))
            .collect();
        let uses = method
            .instructions
            .iter()
            .flat_map(|(pc, insn)| insn.uses().into_iter().map(|it| (it, *pc)))
            .into_group_map()
            .into_iter()
            .map(|(id, uses)| (id, uses.into_iter().collect()))
            .collect();
        Self { defs, uses }
    }

    /// Get the location where an identifier is defined.
    #[must_use]
    pub fn defined_at(&self, value: &LocalValue) -> Option<ProgramCounter> {
        self.defs.get(value).copied()
    }

    /// Get the locations where an identifier is used.
    #[must_use]
    pub fn used_at(&self, id: &Identifier) -> Option<BTreeSet<ProgramCounter>> {
        self.uses.get(id).cloned()
    }
}
