use std::{
    collections::{BTreeMap, HashSet},
    iter,
};

use petgraph::{
    Direction,
    visit::{EdgeRef, IntoEdgeReferences, IntoNeighborsDirected, NodeIndexable},
};

use super::*;
use crate::ir::{BlockKind, Successor, Terminator};

#[test]
fn sparse_nodes_and_parallel_edges_are_preserved() {
    let source_id = BlockId::new(7);
    let target = BlockId::new(42);
    let source = BasicBlock {
        kind: BlockKind::Code,
        parameters: vec![],
        operations: vec![],
        terminator: {
            let mut arms = (0..3)
                .map(|id| Successor::Block {
                    id: EdgeId::new(id),
                    target,
                    arguments: vec![],
                    transfer: ControlTransfer::Unconditional,
                })
                .collect::<Vec<_>>();
            let default = arms.pop().unwrap();
            Terminator::Switch {
                cases: arms,
                default,
            }
        },
    };
    let exit = BasicBlock {
        kind: BlockKind::Code,
        parameters: vec![],
        operations: vec![],
        terminator: Terminator::Return { value: None },
    };
    let blocks = BTreeMap::from([(source_id, source), (target, exit)]);
    let cfg = ControlFlowGraph::new(&blocks, source_id);

    assert_eq!(cfg.node_bound(), 2);
    assert_eq!(cfg.from_index(cfg.to_index(target)), target);
    let edges = (&cfg).edge_references().collect::<Vec<_>>();
    assert_eq!(edges.len(), 3);
    let ids = edges.iter().map(EdgeRef::id).collect::<HashSet<_>>();
    assert_eq!(ids.len(), 3);
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
        terminator: {
            let runtime_exception = "java/lang/RuntimeException".parse().unwrap();
            let mut arms = vec![
                Successor::Block {
                    id: EdgeId::new(0),
                    target: BlockId::new(1),
                    arguments: vec![],
                    transfer: ControlTransfer::Unconditional,
                },
                Successor::Block {
                    id: EdgeId::new(1),
                    target: BlockId::new(2),
                    arguments: vec![],
                    transfer: ControlTransfer::Exception(Some(runtime_exception)),
                },
                Successor::Unwind { id: EdgeId::new(2) },
            ];
            Terminator::Try {
                operation: crate::ir::Operation::Effect {
                    expr: crate::ir::expression::Expression::Const(crate::jvm::ConstantValue::Null),
                },
                normal: arms.remove(0),
                exceptional: arms,
            }
        },
    };
    let exits = (1..=2)
        .map(|id| {
            let terminator = Terminator::Return { value: None };
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

    let ids = edges.iter().map(EdgeRef::id).collect::<HashSet<_>>();
    assert_eq!(ids.len(), 2);
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
        .any(|successor| successor.block_target().is_none())
}
