use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;

use super::{
    BlockExit, ExceptionalTarget, JvmBlock, JvmBlockGraph, JvmBlockId, fallibility::Fallibility,
};
use crate::{
    ir::generator::error::{Error, MalformedBytecode, UnsupportedBytecode},
    jvm::{
        Method,
        code::{Instruction, MethodBody, ProgramCounter, WideInstruction},
    },
};

pub(super) struct Builder<'method> {
    body: &'method MethodBody,
    fallibility: Fallibility,
}

impl<'method> Builder<'method> {
    pub(super) fn for_method(method: &'method Method) -> Result<Self, Error> {
        let body = method.body.as_ref().ok_or(Error::NoMethodBody)?;
        let fallibility = Fallibility::for_method(method);
        Ok(Self { body, fallibility })
    }

    pub(super) fn build(self) -> Result<JvmBlockGraph, Error> {
        let (entry_pc, _) = self
            .body
            .instructions
            .entry_point()
            .ok_or_else(|| Error::malformed(None, MalformedBytecode::MissingEntry))?;
        let block_leaders = self.block_leaders(entry_pc)?;
        let blocks = self.build_blocks(&block_leaders)?;
        let entry = JvmBlockId::from(entry_pc);
        Ok(JvmBlockGraph { entry, blocks })
    }

    /// Collects every PC that starts a block, and verifies each is decoded.
    ///
    /// Every successor target is a leader, so validating leaders here makes
    /// later successor resolution infallible.
    fn block_leaders(&self, entry_pc: ProgramCounter) -> Result<BTreeSet<ProgramCounter>, Error> {
        let mut leaders = BTreeSet::from([entry_pc]);
        let exception_handlers = self.body.exception_table.iter().map(|it| it.handler_pc);
        leaders.extend(exception_handlers);

        for (pc, instruction) in self.body.instructions.iter() {
            match instruction.control_flow() {
                ControlFlow::Goto(target) | ControlFlow::Branch(target) => {
                    leaders.insert(target);
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Switch(targets, default) => {
                    leaders.extend(targets.values());
                    leaders.insert(default);
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Terminal => {
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                ControlFlow::Fallthrough if self.fallibility.can_throw(instruction) => {
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

    /// Builds every block, deriving each block's exit and exceptional
    /// successors from its final instruction alone.
    ///
    /// A block spans from its leader up to the instruction before the next
    /// leader, so the leaders alone determine both the blocks and their spans.
    fn build_blocks(
        &self,
        leaders: &BTreeSet<ProgramCounter>,
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
                let instruction = self
                    .body
                    .instruction_at(end_pc)
                    .expect("a structural block PC comes from decoded bytecode");
                let exit = self.build_exit(end_pc, instruction)?;
                let exception_handlers = if self.fallibility.can_throw(instruction) {
                    self.exceptional_successors(end_pc)
                } else {
                    Vec::default()
                };
                let block = JvmBlock {
                    start_pc,
                    end_pc,
                    exit,
                    exception_handlers,
                };
                Ok((start_pc.into(), block))
            })
            .collect()
    }

    fn build_exit(
        &self,
        pc: ProgramCounter,
        instruction: &Instruction,
    ) -> Result<BlockExit, Error> {
        let block_exit = match instruction.control_flow() {
            ControlFlow::Branch(target) => BlockExit::Branch {
                taken: target.into(),
                fallthrough: self.require_next_pc(pc)?.into(),
            },
            ControlFlow::Goto(target) => BlockExit::Goto {
                target: target.into(),
            },
            ControlFlow::Switch(targets, default) => {
                let cases = targets
                    .into_iter()
                    .map(|(case, target)| (case, target.into()))
                    .collect();
                BlockExit::Switch {
                    cases,
                    default: default.into(),
                }
            }
            ControlFlow::Terminal => BlockExit::Terminal,
            ControlFlow::Fallthrough => BlockExit::Fallthrough {
                target: self.require_next_pc(pc)?.into(),
            },
            ControlFlow::Legacy => unreachable!("Rejected"),
        };
        Ok(block_exit)
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

trait ControlFlowClassification {
    fn control_flow(&self) -> ControlFlow;
}

enum ControlFlow {
    Fallthrough,
    Goto(ProgramCounter),
    Branch(ProgramCounter),
    Switch(BTreeMap<i32, ProgramCounter>, ProgramCounter),
    Terminal,
    Legacy,
}

impl ControlFlowClassification for Instruction {
    fn control_flow(&self) -> ControlFlow {
        use Instruction::{
            AReturn, AThrow, DReturn, FReturn, Goto, GotoW, IReturn, IfACmpEq, IfACmpNe, IfEq,
            IfGe, IfGt, IfICmpEq, IfICmpGe, IfICmpGt, IfICmpLe, IfICmpLt, IfICmpNe, IfLe, IfLt,
            IfNe, IfNonNull, IfNull, Jsr, JsrW, LReturn, Ret, Return, Wide,
        };
        match self {
            IReturn | LReturn | FReturn | DReturn | AReturn | Return | AThrow => {
                ControlFlow::Terminal
            }
            Goto(target) | GotoW(target) => ControlFlow::Goto(*target),
            IfEq(pc) | IfNe(pc) | IfLt(pc) | IfGe(pc) | IfGt(pc) | IfLe(pc) | IfICmpEq(pc)
            | IfICmpNe(pc) | IfICmpLt(pc) | IfICmpGe(pc) | IfICmpGt(pc) | IfICmpLe(pc)
            | IfACmpEq(pc) | IfACmpNe(pc) | IfNull(pc) | IfNonNull(pc) => ControlFlow::Branch(*pc),
            Jsr(_) | JsrW(_) | Ret(_) | Wide(WideInstruction::Ret(_)) => ControlFlow::Legacy,
            Instruction::TableSwitch {
                jump_targets,
                default,
                range,
            } => {
                let matches = range.clone().zip(jump_targets.clone()).collect();
                ControlFlow::Switch(matches, *default)
            }
            Instruction::LookupSwitch {
                default,
                match_targets,
            } => ControlFlow::Switch(match_targets.clone(), *default),
            _ => ControlFlow::Fallthrough,
        }
    }
}
