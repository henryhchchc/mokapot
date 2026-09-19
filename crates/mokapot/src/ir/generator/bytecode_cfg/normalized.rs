//! Reachable control-flow topology consumed by frame analysis.

use std::collections::{BTreeMap, BTreeSet};

use super::{BlockExit, ExceptionalTarget, JvmBlock, JvmBlockGraph, JvmBlockId};
use crate::{
    ir::{BlockId, EdgeId, generator::error::Error},
    jvm::{
        Method,
        code::{Instruction, MethodBody, ProgramCounter},
        references::ClassRef,
    },
};

/// A reachable graph whose synthetic nodes and identities are fixed before
/// frame propagation begins.
pub(crate) struct NormalizedCfg<'method> {
    bytecode: JvmBlockGraph<'method>,
    entry: BlockId,
    blocks: BTreeMap<BlockId, NormalizedBlock>,
}

impl<'method> NormalizedCfg<'method> {
    pub const fn method(&self) -> &'method Method {
        self.bytecode.method()
    }

    pub const fn body(&self) -> &'method MethodBody {
        self.bytecode.body()
    }

    pub const fn entry_block(&self) -> BlockId {
        self.entry
    }

    pub fn blocks(&self) -> impl Iterator<Item = (BlockId, &NormalizedBlock)> {
        self.blocks.iter().map(|(&id, block)| (id, block))
    }

    pub fn block(&self, id: BlockId) -> &NormalizedBlock {
        self.blocks
            .get(&id)
            .expect("a normalized block identity must belong to its graph")
    }

    pub fn bytecode_block(&self, id: JvmBlockId) -> &JvmBlock {
        self.bytecode.block(id)
    }

    pub fn block_instructions(
        &self,
        block_id: JvmBlockId,
    ) -> impl DoubleEndedIterator<Item = (ProgramCounter, &Instruction)> {
        self.bytecode.block_instructions(block_id)
    }
}

/// One normalized block, including its immutable successor topology.
pub(crate) struct NormalizedBlock {
    pub kind: NormalizedBlockKind,
    pub successors: Vec<NormalizedEdge>,
}

/// The executable role of a normalized block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NormalizedBlockKind {
    Bytecode(JvmBlockId),
    HandlerEntry(JvmBlockId),
}

/// A fixed edge in the normalized topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedEdge {
    pub id: EdgeId,
    pub target: NormalizedTarget,
    pub kind: EdgeKind,
}

/// A successor destination in the fixed, internal normalized topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NormalizedTarget {
    Block(BlockId),
    Unwind,
}

impl NormalizedEdge {
    pub const fn block_target(&self) -> Option<BlockId> {
        match self.target {
            NormalizedTarget::Block(target) => Some(target),
            NormalizedTarget::Unwind => None,
        }
    }
}

/// The part of an edge transfer known without executing bytecode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EdgeKind {
    Normal,
    Exception(Option<ClassRef>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Bytecode(JvmBlockId),
    Handler(JvmBlockId),
}

pub(super) fn normalize(bytecode: JvmBlockGraph<'_>) -> Result<NormalizedCfg<'_>, Error> {
    let bytecode_entry = Node::Bytecode(bytecode.entry_block());
    let mut reachable = BTreeSet::new();
    let mut pending = BTreeSet::from([bytecode_entry]);
    while let Some(node) = pending.pop_first() {
        if !reachable.insert(node) {
            continue;
        }
        pending.extend(successors(&bytecode, node).into_iter().filter_map(
            |(target, _)| match target {
                NodeTarget::Block(target) => Some(target),
                NodeTarget::Unwind => None,
            },
        ));
    }

    let ids = reachable
        .iter()
        .copied()
        .enumerate()
        .map(|(index, node)| block_id(index).map(|id| (node, id)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let bytecode_entry_id = ids[&bytecode_entry];
    let entry = bytecode_entry_id;

    let mut next_edge = 0_u32;
    let normalized = reachable.iter().copied().map(|node| {
        let id = ids[&node];
        let edges = successors(&bytecode, node)
            .into_iter()
            .map(|(target, kind)| {
                let target = match target {
                    NodeTarget::Block(target) => NormalizedTarget::Block(ids[&target]),
                    NodeTarget::Unwind => NormalizedTarget::Unwind,
                };
                allocate_edge(&mut next_edge, target, kind)
            })
            .collect::<Result<_, _>>()?;
        let kind = match node {
            Node::Bytecode(block) => NormalizedBlockKind::Bytecode(block),
            Node::Handler(target) => NormalizedBlockKind::HandlerEntry(target),
        };
        Ok((
            id,
            NormalizedBlock {
                kind,
                successors: edges,
            },
        ))
    });
    let blocks: BTreeMap<_, _> = normalized.collect::<Result<_, Error>>()?;

    Ok(NormalizedCfg {
        bytecode,
        entry,
        blocks,
    })
}

#[derive(Debug, Clone, Copy)]
enum NodeTarget {
    Block(Node),
    Unwind,
}

fn successors(bytecode: &JvmBlockGraph<'_>, node: Node) -> Vec<(NodeTarget, EdgeKind)> {
    match node {
        Node::Bytecode(id) => {
            let block = bytecode.block(id);
            ordinary_successors(&block.exit)
                .into_iter()
                .map(|target| (NodeTarget::Block(Node::Bytecode(target)), EdgeKind::Normal))
                .chain(block.exception_handlers.iter().map(|target| match target {
                    ExceptionalTarget::Handler { block, catch_type } => (
                        NodeTarget::Block(Node::Handler(*block)),
                        EdgeKind::Exception(catch_type.clone()),
                    ),
                    ExceptionalTarget::Unwind => (NodeTarget::Unwind, EdgeKind::Exception(None)),
                }))
                .collect()
        }
        Node::Handler(target) => {
            vec![(NodeTarget::Block(Node::Bytecode(target)), EdgeKind::Normal)]
        }
    }
}

fn ordinary_successors(exit: &BlockExit) -> Vec<JvmBlockId> {
    match exit {
        BlockExit::Fallthrough { target } | BlockExit::Goto { target } => vec![*target],
        BlockExit::Branch { taken, fallthrough } => vec![*taken, *fallthrough],
        BlockExit::Switch { cases, default } => cases.values().copied().chain([*default]).collect(),
        BlockExit::Terminal => Vec::new(),
    }
}

fn block_id(index: usize) -> Result<BlockId, Error> {
    u32::try_from(index)
        .map(BlockId::new)
        .map_err(|_| Error::internal("the block identity space is exhausted"))
}

fn allocate_edge(
    next: &mut u32,
    target: NormalizedTarget,
    kind: EdgeKind,
) -> Result<NormalizedEdge, Error> {
    let id = EdgeId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or_else(|| Error::internal("the edge identity space is exhausted"))?;
    Ok(NormalizedEdge { id, target, kind })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{NormalizedBlockKind, NormalizedTarget};
    use crate::{
        ir::{MokaIRMethod, Successor, generator::bytecode_cfg},
        jvm::{code::Instruction, method::AccessFlags},
    };

    #[test]
    fn normalization_assigns_parallel_edges_before_analysis() {
        let switch = Instruction::LookupSwitch {
            default: 10.into(),
            match_targets: BTreeMap::from([(1, 10.into()), (2, 10.into())]),
        };
        let method = crate::tests::method(
            [
                (0, Instruction::ILoad0),
                (1, switch),
                (10, Instruction::Return),
            ],
            "(I)V",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );
        let cfg = bytecode_cfg::build(&method).unwrap();
        let entry = cfg.entry_block();
        let successors = &cfg.block(entry).successors;
        let normalized_edge_ids = successors.iter().map(|edge| edge.id).collect::<Vec<_>>();

        assert_eq!(successors.len(), 3);
        assert!(successors.windows(2).all(|pair| pair[0].id < pair[1].id));
        assert!(
            successors
                .windows(2)
                .all(|pair| pair[0].target == pair[1].target)
        );
        let ir = MokaIRMethod::from_method(&method).unwrap();
        let finished_edge_ids = ir
            .block(ir.entry_block())
            .unwrap()
            .terminator
            .successors()
            .map(Successor::id)
            .collect::<Vec<_>>();
        assert_eq!(finished_edge_ids, normalized_edge_ids);
    }

    #[test]
    fn normalization_keeps_method_entry_external_to_a_self_loop() {
        let method = crate::tests::method(
            [(0, Instruction::Goto(0.into()))],
            "()V",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );
        let cfg = bytecode_cfg::build(&method).unwrap();
        let header = cfg.entry_block();
        assert!(matches!(
            cfg.block(header).kind,
            NormalizedBlockKind::Bytecode(_)
        ));
        let [backedge] = cfg.block(header).successors.as_slice() else {
            panic!("the self-loop header must have one successor");
        };

        assert_eq!(backedge.target, NormalizedTarget::Block(header));
    }

    #[test]
    fn parallel_edges_keep_distinct_internal_parameter_arguments() {
        let switch = Instruction::LookupSwitch {
            default: 8.into(),
            match_targets: BTreeMap::from([(1, 12.into()), (2, 12.into())]),
        };
        let method = crate::tests::method(
            [
                (0, Instruction::IConst0),
                (1, Instruction::IStore1),
                (2, Instruction::ILoad0),
                (3, switch),
                (8, Instruction::IConst1),
                (9, Instruction::IStore1),
                (10, Instruction::Goto(12.into())),
                (12, Instruction::ILoad1),
                (13, Instruction::IReturn),
            ],
            "(I)I",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );
        let cfg = bytecode_cfg::build(&method).unwrap();
        let mut draft = crate::ir::generator::bytecode_analysis::analyze(&cfg).unwrap();
        let (&target, target_block) = draft
            .blocks
            .iter()
            .find(|(_, block)| !block.parameters.is_empty())
            .expect("the local-variable join must have a block parameter");
        assert_eq!(target_block.parameters.len(), 1);
        let parameter_value = target_block.parameters[0].value;

        let incoming = draft
            .blocks
            .values()
            .flat_map(|block| block.terminator.arms())
            .filter(|edge| edge.block_target() == Some(target))
            .collect::<Vec<_>>();
        assert_eq!(incoming.len(), 3);
        assert!(incoming.iter().all(|edge| edge.arguments().len() == 1));
        let incoming_ids = incoming
            .iter()
            .map(|edge| edge.id())
            .collect::<BTreeSet<_>>();
        assert_eq!(incoming_ids.len(), 3);

        crate::ir::generator::canonicalize::canonicalize(&mut draft).unwrap();
        let ir = crate::ir::generator::finish::finish(&method, draft).unwrap();
        let [parameter] = ir.block(target).unwrap().parameters.as_slice() else {
            panic!("the public join must contain one parameter");
        };
        assert_eq!(parameter.value, parameter_value);
        let public_incoming = ir
            .blocks()
            .flat_map(|(_, block)| block.terminator.successors())
            .filter(|edge| edge.block_target() == Some(target))
            .collect::<Vec<_>>();
        assert_eq!(public_incoming.len(), 3);
        assert!(public_incoming.iter().all(|it| it.arguments().len() == 1));
        crate::ir::verify::verify(&ir).unwrap();
    }
}
