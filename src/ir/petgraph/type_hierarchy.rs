//! Type hierarchy graph implementations.
//!
use std::collections::HashSet;

use petgraph::{
    visit::{GraphBase, GraphRef, IntoNeighbors, IntoNeighborsDirected, Visitable},
    Direction,
};

use crate::{
    ir::{ClassHierarchy, InterfaceImplHierarchy},
    jvm::references::ClassRef,
};

impl<'a> GraphBase for &'a ClassHierarchy {
    type EdgeId = (&'a ClassRef, &'a ClassRef);

    type NodeId = &'a ClassRef;
}

impl GraphRef for &ClassHierarchy {}

impl<'a> IntoNeighbors for &'a ClassHierarchy {
    type Neighbors = <HashSet<&'a ClassRef> as IntoIterator>::IntoIter;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        self.inheritance
            .get(a)
            .into_iter()
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
    }
}

/// A visit map for the class hierarchy.
pub type Visited<'a> = HashSet<&'a ClassRef>;

impl<'a> Visitable for &'a ClassHierarchy {
    type Map = Visited<'a>;

    fn visit_map(&self) -> Self::Map {
        HashSet::default()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}

impl<'a> GraphBase for &'a InterfaceImplHierarchy {
    type EdgeId = (&'a ClassRef, &'a ClassRef);

    type NodeId = &'a ClassRef;
}

impl GraphRef for &InterfaceImplHierarchy {}

impl<'a> IntoNeighbors for &'a InterfaceImplHierarchy {
    type Neighbors = <HashSet<&'a ClassRef> as IntoIterator>::IntoIter;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        self.implementations
            .get(a)
            .into_iter()
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
    }
}

impl<'a> IntoNeighborsDirected for &'a InterfaceImplHierarchy {
    type NeighborsDirected = <HashSet<&'a ClassRef> as IntoIterator>::IntoIter;

    fn neighbors_directed(self, a: Self::NodeId, d: Direction) -> Self::NeighborsDirected {
        if d == Direction::Outgoing {
            self.neighbors(a)
        } else {
            self.implementors
                .get(a)
                .into_iter()
                .flatten()
                .collect::<HashSet<_>>()
                .into_iter()
        }
    }
}

impl<'a> Visitable for &'a InterfaceImplHierarchy {
    type Map = Visited<'a>;

    fn visit_map(&self) -> Self::Map {
        HashSet::default()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}
