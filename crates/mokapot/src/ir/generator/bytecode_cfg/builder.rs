use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;

use super::{
    ExceptionalTarget, JvmBlock, JvmBlockGraph, JvmBlockId,
    classification::{self, ControlFlow, InstructionDescription},
};
use crate::{
    ir::generator::error::{Error, MalformedBytecode, UnsupportedBytecode},
    jvm::{
        Method,
        code::{MethodBody, ProgramCounter},
    },
};

pub(super) struct Builder<'method> {
    method: &'method Method,
    body: &'method MethodBody,
}

impl<'method> Builder<'method> {
    pub(super) fn for_method(method: &'method Method) -> Result<Self, Error> {
        let body = method.body.as_ref().ok_or(Error::NoMethodBody)?;
        Ok(Self { method, body })
    }

    pub(super) fn build(self) -> Result<JvmBlockGraph<'method>, Error> {
        let (entry_pc, _) = self
            .body
            .instructions
            .entry_point()
            .ok_or_else(|| Error::malformed(None, MalformedBytecode::MissingEntry))?;
        let descriptions = self
            .body
            .instructions
            .iter()
            .map(|(pc, instruction)| (pc, classification::describe(instruction)))
            .collect();
        let block_leaders = self.block_leaders(entry_pc, &descriptions)?;
        let blocks = self.build_blocks(&block_leaders, &descriptions)?;
        let entry = JvmBlockId::from(entry_pc);
        Ok(JvmBlockGraph {
            method: self.method,
            entry,
            blocks,
        })
    }

    /// Collects every PC that starts a block, and verifies each is decoded.
    ///
    /// Every successor target is a leader, so validating leaders here makes
    /// later successor resolution infallible.
    fn block_leaders(
        &self,
        entry_pc: ProgramCounter,
        descriptions: &BTreeMap<ProgramCounter, InstructionDescription>,
    ) -> Result<BTreeSet<ProgramCounter>, Error> {
        let mut leaders = BTreeSet::from([entry_pc]);
        let exception_handlers = self.body.exception_table.iter().map(|it| it.handler_pc);
        leaders.extend(exception_handlers);

        for (&pc, description) in descriptions {
            match &description.control_flow {
                ControlFlow::Goto(target) | ControlFlow::Branch(target) => {
                    leaders.insert(*target);
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Switch(targets, default) => {
                    leaders.extend(targets.values());
                    leaders.insert(*default);
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Terminal => {
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Fallthrough if description.can_throw => {
                    leaders.insert(self.require_next_pc(pc)?);
                }
                ControlFlow::Fallthrough => {}
                ControlFlow::Legacy => {
                    let kind = UnsupportedBytecode::LegacySubroutine;
                    return Err(Error::UnsupportedBytecode { pc, kind });
                }
            }
        }

        for leader in &leaders {
            if self.body.instruction_at(*leader).is_none() {
                let kind = MalformedBytecode::MissingInstruction;
                return Err(Error::malformed(Some(*leader), kind));
            }
        }
        Ok(leaders)
    }

    fn require_next_pc(&self, pc: ProgramCounter) -> Result<ProgramCounter, Error> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingFallthrough))
    }

    /// Builds every block, deriving each block's ordinary flow and exceptional
    /// successors from its final instruction alone.
    ///
    /// A block spans from its leader up to the instruction before the next
    /// leader, so the leaders alone determine both the blocks and their spans.
    fn build_blocks(
        &self,
        leaders: &BTreeSet<ProgramCounter>,
        descriptions: &BTreeMap<ProgramCounter, InstructionDescription>,
    ) -> Result<BTreeMap<JvmBlockId, JvmBlock>, Error> {
        let instructions = &self.body.instructions;
        let last_pc = instructions
            .iter()
            .next_back()
            .expect("a body with an entry point has instructions")
            .0;
        leaders
            .iter()
            .map(|&start_pc| {
                let end_pc = leaders.range(start_pc..).nth(1).map_or(last_pc, |&next| {
                    instructions
                        .prev_pc_of(&next)
                        .expect("a later leader is preceded by the previous leader")
                });
                let description = descriptions
                    .get(&end_pc)
                    .expect("a structural block PC comes from decoded bytecode");
                let flow = description.control_flow.clone();
                let fallthrough = match &flow {
                    ControlFlow::Fallthrough | ControlFlow::Branch(_) => {
                        Some(self.require_next_pc(end_pc)?)
                    }
                    _ => None,
                };
                let exception_handlers = if description.can_throw {
                    self.exceptional_successors(end_pc)
                } else {
                    Vec::default()
                };
                let block = JvmBlock {
                    start_pc,
                    end_pc,
                    flow,
                    fallthrough,
                    exception_handlers,
                };
                Ok((start_pc.into(), block))
            })
            .collect()
    }

    fn exceptional_successors(&self, pc: ProgramCounter) -> Vec<ExceptionalTarget> {
        let effective_handlers: Vec<_> = self
            .body
            .exception_table
            .iter()
            .filter(|entry| entry.covers(pc))
            // Handler selection walks the table in order, so the first catch-all shadows later ones.
            .take_while_inclusive(|it| !it.catches_all())
            .collect();
        let unwind = effective_handlers
            .last()
            .is_none_or(|it| !it.catches_all())
            .then_some(ExceptionalTarget::Unwind);
        effective_handlers
            .into_iter()
            .map(|entry| ExceptionalTarget::Handler {
                block: entry.handler_pc.into(),
                catch_type: entry.catch_type.clone(),
            })
            .chain(unwind)
            .collect()
    }
}
