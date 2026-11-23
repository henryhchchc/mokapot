use std::collections::{HashMap, VecDeque, hash_map::Entry};

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
    let mut discovery = Discovery::default();
    let entry = discovery.id_for(NodeKey::Code(layout.entry()));
    let mut blocks = HashMap::new();
    while let Some((node, id)) = discovery.pending.pop_front() {
        let block = match node {
            NodeKey::Code(pc) => {
                let BlockShape { end_pc, exit } = layout.take(pc);
                CfgNode::Code {
                    start_pc: pc,
                    end_pc,
                    exit: link_exit(exit, &mut discovery),
                }
            }
            NodeKey::LandingPad(pc) => CfgNode::LandingPad {
                successor: discovery.id_for(NodeKey::Code(pc)),
            },
        };
        blocks.insert(id, block);
    }

    Ok(Cfg {
        method,
        body,
        entry,
        blocks,
    })
}

#[derive(Default)]
struct Discovery {
    allocator: IdAllocator<BlockId>,
    ids: HashMap<NodeKey, BlockId>,
    pending: VecDeque<(NodeKey, BlockId)>,
}

impl Discovery {
    /// Returns a node's identity, queueing it on the first visit.
    fn id_for(&mut self, node: NodeKey) -> BlockId {
        match self.ids.entry(node) {
            Entry::Occupied(slot) => *slot.get(),
            Entry::Vacant(slot) => {
                let id = self.allocator.new_id();
                slot.insert(id);
                self.pending.push_back((node, id));
                id
            }
        }
    }
}

/// Links a block exit, discovering new targets in arm order.
fn link_exit<'class>(
    exit: BlockExit<'class, ProgramCounter>,
    discovery: &mut Discovery,
) -> BlockExit<'class, BlockId> {
    use BlockExit::{Branch, Continue, Goto, Return, Switch, Throw};
    match exit {
        Continue {
            next,
            exception_arms,
        } => Continue {
            next: discovery.id_for(NodeKey::Code(next)),
            exception_arms: link_exception_arms(exception_arms, discovery),
        },
        Goto { target } => Goto {
            target: discovery.id_for(NodeKey::Code(target)),
        },
        Branch { taken, otherwise } => Branch {
            taken: discovery.id_for(NodeKey::Code(taken)),
            otherwise: discovery.id_for(NodeKey::Code(otherwise)),
        },
        Switch { cases, default } => Switch {
            cases: cases
                .into_iter()
                .map(|(value, target)| (value, discovery.id_for(NodeKey::Code(target))))
                .collect(),
            default: discovery.id_for(NodeKey::Code(default)),
        },
        Return { exception_arms } => Return {
            exception_arms: link_exception_arms(exception_arms, discovery),
        },
        Throw { exception_arms } => Throw {
            exception_arms: link_exception_arms(exception_arms, discovery),
        },
    }
}

fn link_exception_arms<'class>(
    exception_arms: ExceptionArms<'class, ProgramCounter>,
    discovery: &mut Discovery,
) -> ExceptionArms<'class, BlockId> {
    exception_arms
        .into_iter()
        .map(|arm| ExceptionArm {
            target: match arm.target {
                ExceptionTarget::Handler(pc) => {
                    ExceptionTarget::Handler(discovery.id_for(NodeKey::LandingPad(pc)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test::prelude::*;

    #[test]
    fn rejects_methods_without_bodies() {
        let mut method = method([(0, InstructionData::Return)], "()V", vec![]);
        method.data_mut().body = None;
        let error = build(method.as_method()).err().unwrap();
        assert_eq!(error.kind, ErrorKind::MissingOrEmptyBody);
    }

    #[test]
    fn assigns_ids_in_breadth_first_arm_order() {
        let instructions = [
            (0, InstructionData::IConst0),
            (1, InstructionData::IfEq(10.into())),
            (4, InstructionData::Return),
            (10, InstructionData::Goto(20.into())),
            (20, InstructionData::Return),
        ];
        let method = method(instructions, "()V", vec![]);
        let cfg = build(method.as_method()).unwrap();
        let mut allocator = IdAllocator::default();
        let [entry, taken, otherwise, next] = std::array::from_fn(|_| allocator.new_id());

        assert_eq!(cfg.entry_block(), entry);
        let starts = [taken, otherwise, next].map(|id| match cfg.block(id) {
            CfgNode::Code { start_pc, .. } => *start_pc,
            CfgNode::LandingPad { .. } => panic!("expected a code block"),
        });
        assert_eq!(starts, [10.into(), 4.into(), 20.into()]);
    }
}
