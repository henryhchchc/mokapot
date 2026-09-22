use std::collections::{HashMap, HashSet, VecDeque};

use super::{
    BlockExit, Cfg, CfgNode, ExceptionArm, ExceptionTarget,
    layout::{BlockLayout, BlockShape},
};
use crate::{
    ir::{BlockId, IdAllocator, generator::error::Error},
    jvm::{Method, code::ProgramCounter},
};

/// Builds the reachable control-flow graph of one method.
pub(super) fn build(method: &Method) -> Result<Cfg<'_>, Error> {
    let body = method.body.as_ref().ok_or(Error::NoMethodBody)?;
    let mut layout = BlockLayout::of(body)?;
    let reachable = ReachableNodes::discover(&layout);
    let ids = NodeIds::allocate(&reachable);
    let blocks = materialize(&mut layout, &reachable, &ids);

    Ok(Cfg {
        method,
        body,
        entry: ids.entry,
        blocks,
    })
}

/// Reachable topology nodes in deterministic breadth-first order.
struct ReachableNodes {
    entry: NodeKey,
    order: Vec<NodeKey>,
}

impl ReachableNodes {
    /// Discovers topology without allocating identities or consuming the layout.
    fn discover(layout: &BlockLayout) -> Self {
        let entry = NodeKey::Code(layout.entry());
        let mut seen = HashSet::from([entry]);
        let mut pending = VecDeque::from([entry]);
        let mut order = Vec::new();

        while let Some(node) = pending.pop_front() {
            order.push(node);
            node.visit_successors(layout, |successor| {
                if seen.insert(successor) {
                    pending.push_back(successor);
                }
            });
        }
        Self { entry, order }
    }
}

/// Dense identities assigned after the reachable topology is fixed.
struct NodeIds {
    entry: BlockId,
    by_key: HashMap<NodeKey, BlockId>,
}

impl NodeIds {
    fn allocate(reachable: &ReachableNodes) -> Self {
        let mut allocator = IdAllocator::default();
        let by_key: HashMap<_, _> = reachable
            .order
            .iter()
            .copied()
            .map(|node| (node, allocator.new_id()))
            .collect();
        let entry = by_key[&reachable.entry];
        Self { entry, by_key }
    }

    fn get(&self, node: NodeKey) -> BlockId {
        self.by_key[&node]
    }
}

/// Materializes reachable nodes now that every target identity is known.
fn materialize(
    layout: &mut BlockLayout,
    reachable: &ReachableNodes,
    ids: &NodeIds,
) -> HashMap<BlockId, CfgNode> {
    reachable
        .order
        .iter()
        .copied()
        .map(|node| {
            let id = ids.get(node);
            let block = match node {
                NodeKey::Code(pc) => {
                    let BlockShape { end_pc, exit } = layout.take(pc);
                    CfgNode::Code {
                        start_pc: pc,
                        end_pc,
                        exit: link_exit(exit, ids),
                    }
                }
                NodeKey::LandingPad(pc) => CfgNode::LandingPad {
                    successor: ids.get(NodeKey::Code(pc)),
                },
            };
            (id, block)
        })
        .collect()
}

/// Links a block exit without changing graph topology.
fn link_exit(exit: BlockExit<ProgramCounter>, ids: &NodeIds) -> BlockExit<BlockId> {
    use BlockExit::{Branch, Continue, Goto, Return, Switch, Throw};
    match exit {
        Continue {
            next,
            exception_arms,
        } => Continue {
            next: ids.get(NodeKey::Code(next)),
            exception_arms: link_exception_arms(exception_arms, ids),
        },
        Goto { target } => Goto {
            target: ids.get(NodeKey::Code(target)),
        },
        Branch { taken, otherwise } => Branch {
            taken: ids.get(NodeKey::Code(taken)),
            otherwise: ids.get(NodeKey::Code(otherwise)),
        },
        Switch { cases, default } => Switch {
            cases: cases
                .into_iter()
                .map(|(value, target)| (value, ids.get(NodeKey::Code(target))))
                .collect(),
            default: ids.get(NodeKey::Code(default)),
        },
        Return { exception_arms } => Return {
            exception_arms: link_exception_arms(exception_arms, ids),
        },
        Throw { exception_arms } => Throw {
            exception_arms: link_exception_arms(exception_arms, ids),
        },
    }
}

fn link_exception_arms(
    exception_arms: Vec<ExceptionArm<ProgramCounter>>,
    ids: &NodeIds,
) -> Vec<ExceptionArm<BlockId>> {
    exception_arms
        .into_iter()
        .map(|arm| ExceptionArm {
            target: match arm.target {
                ExceptionTarget::Handler(pc) => {
                    ExceptionTarget::Handler(ids.get(NodeKey::LandingPad(pc)))
                }
                ExceptionTarget::Unwind => ExceptionTarget::Unwind,
            },
            catch_type: arm.catch_type,
        })
        .collect()
}

/// A topology node whose block identity has not yet been allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NodeKey {
    /// A bytecode block starting at the given location.
    Code(ProgramCounter),
    /// A handler entry landing into the bytecode block at the given location.
    LandingPad(ProgramCounter),
}

impl NodeKey {
    /// Visits each structural successor in arm order.
    fn visit_successors(self, layout: &BlockLayout, mut visit: impl FnMut(Self)) {
        match self {
            Self::Code(pc) => visit_exit_successors(&layout.block(pc).exit, visit),
            Self::LandingPad(pc) => visit(Self::Code(pc)),
        }
    }
}

fn visit_exit_successors(exit: &BlockExit<ProgramCounter>, mut visit: impl FnMut(NodeKey)) {
    use BlockExit::{Branch, Continue, Goto, Return, Switch, Throw};

    match exit {
        Continue {
            next,
            exception_arms,
        } => {
            visit(NodeKey::Code(*next));
            visit_exception_successors(exception_arms, &mut visit);
        }
        Goto { target } => visit(NodeKey::Code(*target)),
        Branch { taken, otherwise } => {
            visit(NodeKey::Code(*taken));
            visit(NodeKey::Code(*otherwise));
        }
        Switch { cases, default } => {
            for target in cases.values() {
                visit(NodeKey::Code(*target));
            }
            visit(NodeKey::Code(*default));
        }
        Return { exception_arms } | Throw { exception_arms } => {
            visit_exception_successors(exception_arms, &mut visit);
        }
    }
}

fn visit_exception_successors(
    arms: &[ExceptionArm<ProgramCounter>],
    visit: &mut impl FnMut(NodeKey),
) {
    for arm in arms {
        if let ExceptionTarget::Handler(pc) = arm.target {
            visit(NodeKey::LandingPad(pc));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        ir::generator::cfg,
        jvm::{code::Instruction, method::AccessFlags},
    };

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
        let cfg = cfg::build(&method).unwrap();
        let parts = crate::ir::generator::dataflow::analyze(&cfg).unwrap();
        let (&target, target_block) = parts
            .blocks
            .iter()
            .find(|(_, block)| !block.parameters.is_empty())
            .expect("the local-variable join has a block parameter");
        assert_eq!(target_block.parameters.len(), 1);

        let incoming = parts
            .blocks
            .values()
            .flat_map(|block| block.terminator.arms())
            .filter(|edge| edge.block_target() == Some(target))
            .collect::<Vec<_>>();
        assert_eq!(incoming.len(), 3);
        assert!(incoming.iter().all(|edge| edge.arguments().len() == 1));
    }
}
