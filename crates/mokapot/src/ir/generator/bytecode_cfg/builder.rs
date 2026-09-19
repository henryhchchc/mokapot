use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use super::{Block, Cfg, ControlFlow, Handler, Target};
use crate::{
    ir::{
        BlockId, IdAllocator,
        generator::error::{Error, MalformedBytecode},
    },
    jvm::{
        Method,
        code::{MethodBody, ProgramCounter},
    },
};

/// Builds the structural CFG of one method in a single discovery walk.
///
/// The builder owns both the validated method and the graph under construction.
/// Discovery and identity allocation are one step: reaching a block target
/// allocates its identity and schedules its block, so `ids`, `pending`, and
/// `block_ids` are only ever updated together, and only by [`Self::discover`].
pub(super) struct Builder<'method> {
    /// The validated method.
    method: &'method Method,
    /// The body being partitioned.
    body: &'method MethodBody,
    /// The identity of every discovered node.
    ids: HashMap<RawNode, BlockId>,
    /// Discovered nodes whose block is not yet constructed, in discovery order.
    pending: VecDeque<(RawNode, BlockId)>,
    /// Constructed blocks, keyed by identity.
    blocks: HashMap<BlockId, Block>,
    /// Allocates block identities in discovery order.
    block_ids: IdAllocator<BlockId>,
}

impl<'method> Builder<'method> {
    /// Starts a builder over the decoded body of `method`.
    pub(super) fn for_method(method: &'method Method) -> Result<Self, Error> {
        let body = method.body.as_ref().ok_or(Error::NoMethodBody)?;
        Ok(Self {
            method,
            body,
            ids: HashMap::new(),
            pending: VecDeque::new(),
            blocks: HashMap::new(),
            block_ids: IdAllocator::default(),
        })
    }

    /// Builds the reachable graph, allocating one identity per block.
    ///
    /// Every instruction is classified and every leader validated before
    /// reachability is considered, so successor resolution and block spans are
    /// infallible below.
    pub(super) fn build(mut self) -> Result<Cfg<'method>, Error> {
        let entry_pc = self
            .body
            .instructions
            .entry_point()
            .map(|(pc, _)| pc)
            .ok_or_else(|| Error::malformed(None, MalformedBytecode::MissingEntry))?;
        let body = self.body;
        let flows = body
            .instructions
            .iter()
            .map(|(pc, instruction)| ControlFlow::of(body, pc, instruction).map(|flow| (pc, flow)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let spans = self.block_spans(entry_pc, &flows)?;

        let entry = self.discover(RawNode::Bytecode(entry_pc));
        while let Some((node, id)) = self.pending.pop_front() {
            let block = match node {
                RawNode::Bytecode(pc) => {
                    let control = self.resolve_flow(&flows[&spans[&pc]]);
                    Block::Bytecode {
                        start_pc: pc,
                        end_pc: spans[&pc],
                        control,
                    }
                }
                RawNode::Handler(pc) => Block::HandlerEntry {
                    successor: self.discover(RawNode::Bytecode(pc)),
                },
            };
            self.blocks.insert(id, block);
        }

        Ok(Cfg {
            method: self.method,
            entry,
            blocks: self.blocks,
        })
    }

    /// The identity of `node`, allocating one and scheduling its block on first
    /// discovery.
    fn discover(&mut self, node: RawNode) -> BlockId {
        if let Some(&id) = self.ids.get(&node) {
            return id;
        }
        let id = self.block_ids.new_id();
        self.ids.insert(node, id);
        self.pending.push_back((node, id));
        id
    }

    /// Rewrites a final flow from bytecode locations to block identities.
    ///
    /// Ordinary targets discover bytecode nodes; handler targets discover
    /// handler entries, so both are scheduled on first reach.
    fn resolve_flow(&mut self, flow: &ControlFlow<ProgramCounter>) -> ControlFlow<BlockId> {
        use ControlFlow::{Branch, Fallthrough, Goto, Return, Switch, Throw};
        match flow {
            Fallthrough { next, handlers } => Fallthrough {
                next: self.discover(RawNode::Bytecode(*next)),
                handlers: self.resolve_handlers(handlers),
            },
            Goto { target } => Goto {
                target: self.discover(RawNode::Bytecode(*target)),
            },
            Branch { taken, otherwise } => Branch {
                taken: self.discover(RawNode::Bytecode(*taken)),
                otherwise: self.discover(RawNode::Bytecode(*otherwise)),
            },
            Switch { cases, default } => Switch {
                cases: cases
                    .iter()
                    .map(|(&value, &target)| (value, self.discover(RawNode::Bytecode(target))))
                    .collect(),
                default: self.discover(RawNode::Bytecode(*default)),
            },
            Return { handlers } => Return {
                handlers: self.resolve_handlers(handlers),
            },
            Throw { handlers } => Throw {
                handlers: self.resolve_handlers(handlers),
            },
        }
    }

    /// Rewrites exception outcomes from bytecode locations to block identities.
    fn resolve_handlers(&mut self, handlers: &[Handler<ProgramCounter>]) -> Vec<Handler<BlockId>> {
        handlers
            .iter()
            .map(|handler| {
                let target = match handler.target {
                    Target::Block(pc) => Target::Block(self.discover(RawNode::Handler(pc))),
                    Target::Unwind => Target::Unwind,
                };
                Handler {
                    target,
                    catch: handler.catch.clone(),
                }
            })
            .collect()
    }

    /// Maps every PC that starts a block to its final PC, and verifies each is
    /// decoded.
    ///
    /// Leaders are the block targets of every flow, the effective exception
    /// locations, and the instruction after a transfer that ends its block.
    /// Every successor target is a leader, so validating leaders here makes
    /// later block resolution and span lookup infallible.
    fn block_spans(
        &self,
        entry_pc: ProgramCounter,
        flows: &BTreeMap<ProgramCounter, ControlFlow<ProgramCounter>>,
    ) -> Result<BTreeMap<ProgramCounter, ProgramCounter>, Error> {
        let mut leaders = BTreeSet::from([entry_pc]);
        leaders.extend(self.body.exception_table.iter().map(|it| it.handler_pc));

        for (&pc, flow) in flows {
            match flow {
                ControlFlow::Fallthrough { next, .. } if flow.may_throw() => {
                    leaders.insert(*next);
                }
                ControlFlow::Fallthrough { .. } => {}
                ControlFlow::Goto { target } => {
                    leaders.insert(*target);
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Branch { taken, otherwise } => {
                    leaders.insert(*taken);
                    leaders.insert(*otherwise);
                }
                ControlFlow::Switch { cases, default } => {
                    leaders.extend(cases.values());
                    leaders.insert(*default);
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Return { .. } | ControlFlow::Throw { .. } => {
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
            }
        }

        for &leader in &leaders {
            if self.body.instruction_at(leader).is_none() {
                let kind = MalformedBytecode::MissingInstruction;
                return Err(Error::malformed(Some(leader), kind));
            }
        }

        let last_pc = *flows
            .keys()
            .next_back()
            .expect("a body with an entry point has instructions");
        let spans = leaders
            .iter()
            .copied()
            .map(|start_pc| (start_pc, self.block_end(start_pc, &leaders, last_pc)))
            .collect();
        Ok(spans)
    }

    /// The final instruction of the block starting at `start_pc`.
    ///
    /// A block spans from its leader up to the instruction before the next
    /// leader, so the leaders alone determine every block span.
    fn block_end(
        &self,
        start_pc: ProgramCounter,
        leaders: &BTreeSet<ProgramCounter>,
        last_pc: ProgramCounter,
    ) -> ProgramCounter {
        leaders.range(start_pc..).nth(1).map_or(last_pc, |&next| {
            self.body
                .instructions
                .prev_pc_of(&next)
                .expect("a later leader is preceded by the previous leader")
        })
    }
}

/// A topology node whose block identity has not yet been allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RawNode {
    /// A bytecode block starting at the given location.
    Bytecode(ProgramCounter),
    /// A handler entry landing into the bytecode block at the given location.
    Handler(ProgramCounter),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        ir::{
            MalformedBytecode, MokaIRBuildError,
            generator::{bytecode_cfg, tests::reachable_blocks},
        },
        jvm::{code::Instruction, method::AccessFlags},
    };

    #[test]
    fn rejects_a_missing_fallthrough_even_when_its_source_is_unreachable() {
        let method = crate::tests::method(
            [(0, Instruction::Return), (1, Instruction::Nop)],
            "()V",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );

        assert!(matches!(
            bytecode_cfg::build(&method),
            Err(MokaIRBuildError::MalformedBytecode {
                pc: Some(pc),
                kind: MalformedBytecode::MissingFallthrough,
            }) if pc == 1.into()
        ));
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

        crate::ir::generator::canonicalize::canonicalize(&mut draft).unwrap();
        let ir = crate::ir::generator::finish::finish(&method, draft).unwrap();
        let [parameter] = ir.block(target).unwrap().parameters.as_slice() else {
            panic!("the public join must contain one parameter");
        };
        assert_eq!(parameter.value, parameter_value);
        let public_incoming = reachable_blocks(&ir)
            .into_iter()
            .map(|(_, block)| block)
            .flat_map(|block| block.terminator.successors())
            .filter(|edge| edge.block_target() == Some(target))
            .collect::<Vec<_>>();
        assert_eq!(public_incoming.len(), 3);
        assert!(public_incoming.iter().all(|it| it.arguments().len() == 1));
        ir.verify();
    }
}
