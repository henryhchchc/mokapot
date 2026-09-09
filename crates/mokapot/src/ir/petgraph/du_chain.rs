//! Petgraph implementation of the [`DefUseChain`].
use std::collections::HashSet;

use petgraph::visit::{GraphBase, IntoNeighbors, Visitable};

use crate::ir::{DefUseChain, InstructionId, MokaIRMethod, ValueDefinition, ValueId};

impl GraphBase for DefUseChain<'_> {
    type NodeId = ValueId;
    type EdgeId = (ValueId, ValueId);
}

impl IntoNeighbors for &DefUseChain<'_> {
    type Neighbors = <HashSet<ValueId> as IntoIterator>::IntoIter;

    fn neighbors(self, node: ValueId) -> Self::Neighbors {
        let Some(ValueDefinition::Instruction(instruction)) = self.defined_at(node) else {
            return HashSet::new().into_iter();
        };

        values_used_at(self.method, instruction).into_iter()
    }
}

fn values_used_at(method: &MokaIRMethod, id: InstructionId) -> HashSet<ValueId> {
    method.uses_at(id).unwrap_or_default()
}

/// A visit map for the def-use chain.
pub type Visited = HashSet<ValueId>;

impl Visitable for DefUseChain<'_> {
    type Map = Visited;

    fn visit_map(&self) -> Self::Map {
        Visited::default()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}
