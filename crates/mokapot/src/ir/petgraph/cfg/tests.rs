use std::collections::HashSet;

use petgraph::{
    Direction,
    visit::{EdgeRef, IntoEdgeReferences, IntoNeighborsDirected, NodeIndexable},
};

use super::*;
use crate::ir::{InstructionId, Successor, Terminator, TerminatorKind, ValueId};

#[test]
fn dense_nodes_and_parallel_edges_are_preserved() {
    let target = BlockId::new(1);
    let source = BasicBlock {
        id: BlockId::new(0),
        phis: vec![],
        operations: vec![],
        terminator: Terminator {
            id: InstructionId::new(0),
            kind: TerminatorKind::Switch {
                match_value: ValueId::new(0),
            },
            successors: (0..3)
                .map(|id| Successor {
                    id: EdgeId::new(id),
                    target,
                    transfer: ControlTransfer::Unconditional,
                })
                .collect(),
        },
    };
    let exit = BasicBlock {
        id: target,
        phis: vec![],
        operations: vec![],
        terminator: Terminator {
            id: InstructionId::new(1),
            kind: TerminatorKind::Return(None),
            successors: vec![],
        },
    };
    let blocks = [source, exit];
    let cfg = ControlFlowGraph::new(&blocks, BlockId::new(0));

    assert_eq!(cfg.node_bound(), 2);
    assert_eq!(cfg.from_index(cfg.to_index(target)), target);
    let edges = (&cfg).edge_references().collect::<Vec<_>>();
    assert_eq!(edges.len(), 3);
    assert_eq!(
        edges.iter().map(EdgeRef::id).collect::<HashSet<_>>().len(),
        3
    );
    assert_eq!(
        (&cfg)
            .neighbors_directed(target, Direction::Incoming)
            .collect::<Vec<_>>(),
        [BlockId::new(0), BlockId::new(0), BlockId::new(0)]
    );
}

#[test]
fn exceptional_edge_kinds_and_identities_are_preserved() {
    let source = BasicBlock {
        id: BlockId::new(0),
        phis: vec![],
        operations: vec![],
        terminator: Terminator {
            id: InstructionId::new(0),
            kind: TerminatorKind::Fallible,
            successors: vec![
                Successor {
                    id: EdgeId::new(0),
                    target: BlockId::new(1),
                    transfer: ControlTransfer::Normal,
                },
                Successor {
                    id: EdgeId::new(1),
                    target: BlockId::new(2),
                    transfer: ControlTransfer::Exception(Some(
                        "java/lang/RuntimeException".parse().unwrap(),
                    )),
                },
                Successor {
                    id: EdgeId::new(2),
                    target: BlockId::new(3),
                    transfer: ControlTransfer::Unwind,
                },
            ],
        },
    };
    let exits = (1..=3)
        .map(|id| BasicBlock {
            id: BlockId::new(id),
            phis: vec![],
            operations: vec![],
            terminator: Terminator {
                id: InstructionId::new(id),
                kind: if id == 3 {
                    TerminatorKind::Unwind
                } else {
                    TerminatorKind::Return(None)
                },
                successors: vec![],
            },
        })
        .collect::<Vec<_>>();
    let blocks = std::iter::once(source).chain(exits).collect::<Vec<_>>();
    let cfg = ControlFlowGraph::new(&blocks, BlockId::new(0));
    let edges = (&cfg).edge_references().collect::<Vec<_>>();

    assert_eq!(
        edges.iter().map(EdgeRef::id).collect::<HashSet<_>>().len(),
        3
    );
    assert!(matches!(edges[0].weight(), ControlTransfer::Normal));
    assert!(matches!(
        edges[1].weight(),
        ControlTransfer::Exception(Some(_))
    ));
    assert!(matches!(edges[2].weight(), ControlTransfer::Unwind));
    assert_eq!(
        (&cfg)
            .neighbors_directed(BlockId::new(3), Direction::Incoming)
            .collect::<Vec<_>>(),
        [BlockId::new(0)]
    );
}
