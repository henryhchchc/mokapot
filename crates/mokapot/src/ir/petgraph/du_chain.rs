//! Petgraph implementation of the [`DefUseChain`].
use std::collections::BTreeSet;

use petgraph::visit::{GraphBase, IntoNeighbors, VisitMap, Visitable};

use crate::ir::{DefUseChain, Identifier};

impl GraphBase for DefUseChain<'_> {
    type NodeId = Identifier;
    type EdgeId = (Identifier, Identifier);
}

impl IntoNeighbors for &DefUseChain<'_> {
    type Neighbors = <BTreeSet<Identifier> as IntoIterator>::IntoIter;

    fn neighbors(self, node: Identifier) -> Self::Neighbors {
        if let Identifier::Local(loc) = node
            && let Some(pc) = self.defined_at(&loc)
            && let Some(insn) = self.method.instructions.get(&pc)
        {
            return insn.uses().into_iter();
        }
        BTreeSet::default().into_iter()
    }
}

/// A visit map for the def-use chain.
pub type Visited = BTreeSet<Identifier>;

impl VisitMap<Identifier> for Visited {
    fn visit(&mut self, a: Identifier) -> bool {
        self.insert(a)
    }

    fn is_visited(&self, a: &Identifier) -> bool {
        self.contains(a)
    }

    fn unvisit(&mut self, a: Identifier) -> bool {
        self.remove(&a)
    }
}

impl Visitable for DefUseChain<'_> {
    type Map = Visited;

    fn visit_map(&self) -> Self::Map {
        Visited::default()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}
