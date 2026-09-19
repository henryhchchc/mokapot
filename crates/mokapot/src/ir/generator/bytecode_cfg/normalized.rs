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

/// One normalized block, including its immutable predecessor and successor
/// topology.
pub(crate) struct NormalizedBlock {
    pub kind: NormalizedBlockKind,
    pub predecessors: BTreeSet<BlockId>,
    pub successors: Vec<NormalizedEdge>,
}

/// The executable role of a normalized block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NormalizedBlockKind {
    EntryPreheader,
    Bytecode(JvmBlockId),
    HandlerEntry(JvmBlockId),
    Unwind,
}

/// A fixed edge in the normalized topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedEdge {
    pub id: EdgeId,
    pub target: BlockId,
    pub kind: EdgeKind,
}

/// The part of an edge transfer known without executing bytecode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EdgeKind {
    Normal,
    Exception(Option<ClassRef>),
    Unwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Bytecode(JvmBlockId),
    Handler(JvmBlockId),
    Unwind,
}

pub(super) fn normalize(bytecode: JvmBlockGraph<'_>) -> Result<NormalizedCfg<'_>, Error> {
    let bytecode_entry = Node::Bytecode(bytecode.entry_block());
    let mut reachable = BTreeSet::new();
    let mut pending = BTreeSet::from([bytecode_entry]);
    while let Some(node) = pending.pop_first() {
        if !reachable.insert(node) {
            continue;
        }
        pending.extend(
            successors(&bytecode, node)
                .into_iter()
                .map(|(target, _)| target),
        );
    }

    let has_preheader = reachable.iter().copied().any(|node| {
        successors(&bytecode, node)
            .iter()
            .any(|(target, _)| *target == bytecode_entry)
    });
    let offset = usize::from(has_preheader);
    let ids = reachable
        .iter()
        .copied()
        .enumerate()
        .map(|(index, node)| block_id(index + offset).map(|id| (node, id)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let bytecode_entry_id = ids[&bytecode_entry];
    let entry = if has_preheader {
        BlockId::new(0)
    } else {
        bytecode_entry_id
    };

    let mut next_edge = 0_u32;
    let preheader = has_preheader.then(|| {
        let edge = allocate_edge(&mut next_edge, bytecode_entry_id, EdgeKind::Normal)?;
        Ok((
            BlockId::new(0),
            NormalizedBlock {
                kind: NormalizedBlockKind::EntryPreheader,
                predecessors: BTreeSet::new(),
                successors: vec![edge],
            },
        ))
    });
    let normalized = reachable.iter().copied().map(|node| {
        let id = ids[&node];
        let edges = successors(&bytecode, node)
            .into_iter()
            .map(|(target, kind)| allocate_edge(&mut next_edge, ids[&target], kind))
            .collect::<Result<_, _>>()?;
        let kind = match node {
            Node::Bytecode(block) => NormalizedBlockKind::Bytecode(block),
            Node::Handler(target) => NormalizedBlockKind::HandlerEntry(target),
            Node::Unwind => NormalizedBlockKind::Unwind,
        };
        Ok((
            id,
            NormalizedBlock {
                kind,
                predecessors: BTreeSet::new(),
                successors: edges,
            },
        ))
    });
    let mut blocks: BTreeMap<_, _> = preheader
        .into_iter()
        .chain(normalized)
        .collect::<Result<_, Error>>()?;

    let predecessors = blocks
        .iter()
        .flat_map(|(&source, block)| {
            block
                .successors
                .iter()
                .map(move |edge| (source, edge.target))
        })
        .collect::<Vec<_>>();
    for (source, target) in predecessors {
        blocks
            .get_mut(&target)
            .expect("a normalized edge target must belong to its graph")
            .predecessors
            .insert(source);
    }

    Ok(NormalizedCfg {
        bytecode,
        entry,
        blocks,
    })
}

fn successors(bytecode: &JvmBlockGraph<'_>, node: Node) -> Vec<(Node, EdgeKind)> {
    match node {
        Node::Bytecode(id) => {
            let block = bytecode.block(id);
            ordinary_successors(&block.exit)
                .into_iter()
                .map(|target| (Node::Bytecode(target), EdgeKind::Normal))
                .chain(block.exception_handlers.iter().map(|target| match target {
                    ExceptionalTarget::Handler { block, catch_type } => (
                        Node::Handler(*block),
                        EdgeKind::Exception(catch_type.clone()),
                    ),
                    ExceptionalTarget::Unwind => (Node::Unwind, EdgeKind::Unwind),
                }))
                .collect()
        }
        Node::Handler(target) => vec![(Node::Bytecode(target), EdgeKind::Normal)],
        Node::Unwind => Vec::new(),
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

fn allocate_edge(next: &mut u32, target: BlockId, kind: EdgeKind) -> Result<NormalizedEdge, Error> {
    let id = EdgeId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or_else(|| Error::internal("the edge identity space is exhausted"))?;
    Ok(NormalizedEdge { id, target, kind })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::NormalizedBlockKind;
    use crate::{
        ir::{MokaIRMethod, Successor, generator::bytecode_cfg},
        jvm::{code::Instruction, method::AccessFlags},
    };

    #[test]
    fn normalization_assigns_parallel_edges_before_analysis() {
        let method = crate::tests::method(
            [
                (0, Instruction::ILoad0),
                (
                    1,
                    Instruction::LookupSwitch {
                        default: 10.into(),
                        match_targets: BTreeMap::from([(1, 10.into()), (2, 10.into())]),
                    },
                ),
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
        assert_eq!(
            cfg.block(successors[0].target).predecessors,
            BTreeSet::from([entry])
        );

        let ir = MokaIRMethod::from_method(&method).unwrap();
        let finished_edge_ids = ir
            .block(ir.entry_block())
            .unwrap()
            .terminator
            .successors()
            .iter()
            .map(Successor::id)
            .collect::<Vec<_>>();
        assert_eq!(finished_edge_ids, normalized_edge_ids);
    }

    #[test]
    fn normalization_materializes_entry_preheader_and_backedge() {
        let method = crate::tests::method(
            [(0, Instruction::Goto(0.into()))],
            "()V",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );
        let cfg = bytecode_cfg::build(&method).unwrap();
        let preheader = cfg.entry_block();
        assert_eq!(
            cfg.block(preheader).kind,
            NormalizedBlockKind::EntryPreheader
        );
        let [entry_edge] = cfg.block(preheader).successors.as_slice() else {
            panic!("the preheader must have one successor");
        };
        let header = entry_edge.target;
        let [backedge] = cfg.block(header).successors.as_slice() else {
            panic!("the self-loop header must have one successor");
        };

        assert_eq!(backedge.target, header);
        assert_eq!(
            cfg.block(header).predecessors,
            BTreeSet::from([preheader, header])
        );
        assert_ne!(entry_edge.id, backedge.id);
    }
}
