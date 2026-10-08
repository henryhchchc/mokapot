//! Type hierarchy graph implementations.
//!
use std::collections::HashSet;

use petgraph::{
    Direction,
    visit::{GraphBase, GraphRef, IntoNeighbors, IntoNeighborsDirected, Visitable},
};

use crate::{
    analysis::{ClassHierarchy, InterfaceImplHierarchy},
    types::class_name::ClassName,
};

impl<'a> GraphBase for &'a ClassHierarchy {
    type EdgeId = (&'a ClassName, &'a ClassName);

    type NodeId = &'a ClassName;
}

impl GraphRef for &ClassHierarchy {}

impl<'a> IntoNeighbors for &'a ClassHierarchy {
    type Neighbors = <HashSet<&'a ClassName> as IntoIterator>::IntoIter;

    fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
        self.inheritance
            .get(a)
            .into_iter()
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
    }
}

impl<'a> Visitable for &'a ClassHierarchy {
    type Map = HashSet<&'a ClassName>;

    fn visit_map(&self) -> Self::Map {
        HashSet::default()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}

impl<'a> GraphBase for &'a InterfaceImplHierarchy {
    type EdgeId = (&'a ClassName, &'a ClassName);

    type NodeId = &'a ClassName;
}

impl GraphRef for &InterfaceImplHierarchy {}

impl<'a> IntoNeighbors for &'a InterfaceImplHierarchy {
    type Neighbors = <HashSet<&'a ClassName> as IntoIterator>::IntoIter;

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
    type NeighborsDirected = <HashSet<&'a ClassName> as IntoIterator>::IntoIter;

    fn neighbors_directed(self, a: Self::NodeId, d: Direction) -> Self::NeighborsDirected {
        if d == Direction::Outgoing {
            self.neighbors(a)
        } else {
            self.implementers
                .get(a)
                .into_iter()
                .flatten()
                .collect::<HashSet<_>>()
                .into_iter()
        }
    }
}

impl<'a> Visitable for &'a InterfaceImplHierarchy {
    type Map = HashSet<&'a ClassName>;

    fn visit_map(&self) -> Self::Map {
        HashSet::new()
    }

    fn reset_map(&self, map: &mut Self::Map) {
        map.clear();
    }
}
