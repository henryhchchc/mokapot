//! Petgraph implementation of the [`DefUseChain`].
use std::collections::HashSet;

use petgraph::visit::{GraphBase, IntoNeighbors, Visitable};

use crate::ir::{DefUseChain, Identifier};

impl GraphBase for DefUseChain<'_> {
    type NodeId = Identifier;
    type EdgeId = (Identifier, Identifier);
}

impl IntoNeighbors for &DefUseChain<'_> {
    type Neighbors = <HashSet<Identifier> as IntoIterator>::IntoIter;

    fn neighbors(self, node: Identifier) -> Self::Neighbors {
        if let Identifier::Local(loc) = node
            && let Some(pc) = self.defined_at(&loc)
            && let Some(insn) = self.method.instructions.get(&pc)
        {
            return insn.uses().into_iter();
        }
        Self::Neighbors::default()
    }
}

/// A visit map for the def-use chain.
pub type Visited = HashSet<Identifier>;

impl Visitable for DefUseChain<'_> {
    type Map = Visited;

    fn visit_map(&self) -> Self::Map {
        Visited::default()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}
