use std::{
    collections::{BTreeMap, HashSet},
    iter,
};

use petgraph::{
    Direction,
    visit::{EdgeRef, IntoEdgeReferences, IntoNeighborsDirected, NodeIndexable},
};

use super::*;
use crate::ir::{BlockKind, Successor, SuccessorTarget, Terminator, TerminatorKind, ValueId};

#[test]
fn sparse_nodes_and_parallel_edges_are_preserved() {
    let source_id = BlockId::new(7);
    let target = BlockId::new(42);
    let source = BasicBlock {
        kind: BlockKind::Code,
        parameters: vec![],
        operations: vec![],
        terminator: Terminator {
            kind: TerminatorKind::Switch {
                match_value: ValueId::new(0),
            },
            successors: (0..3)
                .map(|id| Successor {
                    id: EdgeId::new(id),
                    target: SuccessorTarget::Block(target),
                    arguments: vec![],
                    transfer: ControlTransfer::Unconditional,
                })
                .collect(),
        },
    };
    let exit = BasicBlock {
        kind: BlockKind::Code,
        parameters: vec![],
        operations: vec![],
        terminator: Terminator {
            kind: TerminatorKind::Return(None),
            successors: vec![],
        },
    };
    let blocks = BTreeMap::from([(source_id, source), (target, exit)]);
    let cfg = ControlFlowGraph::new(&blocks, source_id);

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
        [source_id, source_id, source_id]
    );
}

#[test]
fn exceptional_edge_kinds_and_identities_are_preserved() {
    let source = BasicBlock {
        kind: BlockKind::Code,
        parameters: vec![],
        operations: vec![],
        terminator: Terminator {
            kind: TerminatorKind::Fallible,
            successors: vec![
                Successor {
                    id: EdgeId::new(0),
                    target: SuccessorTarget::Block(BlockId::new(1)),
                    arguments: vec![],
                    transfer: ControlTransfer::Unconditional,
                },
                Successor {
                    id: EdgeId::new(1),
                    target: SuccessorTarget::Block(BlockId::new(2)),
                    arguments: vec![],
                    transfer: ControlTransfer::Exception(Some(
                        "java/lang/RuntimeException".parse().unwrap(),
                    )),
                },
                Successor {
                    id: EdgeId::new(2),
                    target: SuccessorTarget::Unwind,
                    arguments: vec![],
                    transfer: ControlTransfer::Unwind,
                },
            ],
        },
    };
    let exits = (1..=2)
        .map(|id| {
            let kind = TerminatorKind::Return(None);
            let terminator = Terminator {
                kind,
                successors: vec![],
            };
            let basic_block = BasicBlock {
                kind: BlockKind::Code,
                parameters: vec![],
                operations: vec![],
                terminator,
            };
            (BlockId::new(id), basic_block)
        })
        .collect::<Vec<_>>();
    let blocks = iter::once((BlockId::new(0), source))
        .chain(exits)
        .collect::<BTreeMap<_, _>>();
    let cfg = ControlFlowGraph::new(&blocks, BlockId::new(0));
    let edges = (&cfg).edge_references().collect::<Vec<_>>();

    assert_eq!(
        edges.iter().map(EdgeRef::id).collect::<HashSet<_>>().len(),
        2
    );
    assert!(matches!(edges[0].weight(), ControlTransfer::Unconditional));
    assert!(matches!(
        edges[1].weight(),
        ControlTransfer::Exception(Some(_))
    ));
    assert!(source_has_explicit_unwind(&blocks));
}

fn source_has_explicit_unwind(blocks: &BTreeMap<BlockId, BasicBlock>) -> bool {
    blocks[&BlockId::new(0)]
        .terminator
        .successors()
        .iter()
        .any(|successor| successor.target() == SuccessorTarget::Unwind)
}
