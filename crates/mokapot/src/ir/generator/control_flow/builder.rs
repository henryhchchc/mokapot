use std::collections::{HashMap, hash_map::Entry};

use super::{
    BlockExit, Cfg, CfgNode, Error, ExceptionArm, ExceptionTarget,
    block_exit::ExceptionArms,
    layout::{BlockLayout, BlockShape},
};
use crate::{
    ir::{BlockId, IdAllocator, generator::error::ErrorKind},
    jvm::{Method, code::ProgramCounter},
};

/// Builds the reachable control-flow graph of one method.
pub(super) fn build(method: Method<'_>) -> Result<Cfg<'_>, Error> {
    let body = method.body().ok_or(ErrorKind::MissingOrEmptyBody)?;
    let mut layout = BlockLayout::of(body)?;
    let ids = NodeIds::discover(&layout);
    let blocks = materialize(&mut layout, &ids);

    Ok(Cfg {
        method,
        body,
        entry: ids.entry,
        blocks,
    })
}

/// Identities assigned to reachable nodes in deterministic breadth-first order.
struct NodeIds {
    entry: BlockId,
    by_key: HashMap<NodeKey, BlockId>,
    order: Vec<(NodeKey, BlockId)>,
}

impl NodeIds {
    fn discover(layout: &BlockLayout) -> Self {
        let mut allocator = IdAllocator::default();
        let entry = allocator.new_id();
        let entry_node = NodeKey::Code(layout.entry());
        let mut by_key = HashMap::from([(entry_node, entry)]);
        let mut order = vec![(entry_node, entry)];
        let mut next = 0;

        while let Some(&(node, _)) = order.get(next) {
            node.visit_successors(layout, |successor| {
                if let Entry::Vacant(slot) = by_key.entry(successor) {
                    let id = allocator.new_id();
                    slot.insert(id);
                    order.push((successor, id));
                }
            });
            next += 1;
        }
        Self {
            entry,
            by_key,
            order,
        }
    }

    fn get(&self, node: NodeKey) -> BlockId {
        self.by_key[&node]
    }
}

/// Materializes reachable nodes now that every target identity is known.
fn materialize<'class>(
    layout: &mut BlockLayout<'class>,
    ids: &NodeIds,
) -> HashMap<BlockId, CfgNode<'class>> {
    ids.order
        .iter()
        .map(|&(node, id)| {
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
fn link_exit<'class>(
    exit: BlockExit<'class, ProgramCounter>,
    ids: &NodeIds,
) -> BlockExit<'class, BlockId> {
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

fn link_exception_arms<'class>(
    exception_arms: ExceptionArms<'class, ProgramCounter>,
    ids: &NodeIds,
) -> ExceptionArms<'class, BlockId> {
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

/// A topology node identified by its kind and bytecode location.
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

fn visit_exit_successors(exit: &BlockExit<'_, ProgramCounter>, mut visit: impl FnMut(NodeKey)) {
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
    arms: &[ExceptionArm<'_, ProgramCounter>],
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
    use crate::ir::test::prelude::*;

    #[test]
    fn rejects_methods_without_bodies() {
        let mut method = method([(0, InstructionData::Return)], "()V", vec![]);
        method.data_mut().body = None;
        let error = super::build(method.as_method()).err().unwrap();
        assert_eq!(error.kind, super::ErrorKind::MissingOrEmptyBody);
    }
}
