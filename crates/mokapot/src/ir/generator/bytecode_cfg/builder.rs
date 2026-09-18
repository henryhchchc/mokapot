use std::collections::{BTreeMap, BTreeSet};

use super::{
    fallibility::Fallibility,
    model::{Block, BlockExit, BytecodeCfg, ExceptionalTarget, HandlerId, StructuralBlockId},
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

    pub(super) fn build(self) -> Result<BytecodeCfg, Error> {
        let (entry_pc, _) = self
            .body
            .instructions
            .entry_point()
            .ok_or_else(|| Error::malformed(None, MalformedBytecode::MissingEntry))?;
        let block_leaders = self.block_leaders(entry_pc)?;
        let groups = self.partition(&block_leaders);
        let blocks = self.build_blocks(groups)?;

        Ok(BytecodeCfg {
            entry: StructuralBlockId::from_pc(entry_pc),
            blocks,
        })
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

    /// Groups the decoded instruction PCs under the leader that starts each block.
    fn partition(&self, leaders: &BTreeSet<ProgramCounter>) -> Vec<BlockGroup> {
        let mut groups: Vec<BlockGroup> = Vec::with_capacity(leaders.len());
        for (pc, _) in self.body.instructions.iter() {
            if leaders.contains(&pc) {
                groups.push(BlockGroup {
                    start_pc: pc,
                    end_pc: pc,
                });
            } else {
                // The entry PC is always a leader, so a non-leader always has a block to extend.
                let group = groups.last_mut().expect("the entry PC is always a leader");
                group.end_pc = pc;
            }
        }
        groups
    }

    /// Builds every block together with its exit and exceptional
    /// successors, which follow from the block's final instruction alone.
    fn build_blocks(
        &self,
        groups: Vec<BlockGroup>,
    ) -> Result<BTreeMap<ProgramCounter, Block>, Error> {
        groups
            .into_iter()
            .map(|group| {
                let final_pc = group.end_pc;
                let instruction = self
                    .body
                    .instruction_at(final_pc)
                    .expect("a structural block PC comes from decoded bytecode");
                let exit = self.build_exit(final_pc, instruction)?;
                let exceptional_successors = if self.fallibility.can_throw(instruction) {
                    self.exceptional_successors(final_pc)
                } else {
                    Vec::new()
                };
                Ok((
                    group.start_pc,
                    Block {
                        end_pc: group.end_pc,
                        exit,
                        exceptional_successors,
                    },
                ))
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
                taken: StructuralBlockId::from_pc(target),
                fallthrough: StructuralBlockId::from_pc(self.require_next_pc(pc)?),
            },
            ControlFlow::Goto(target) => BlockExit::Goto {
                target: StructuralBlockId::from_pc(target),
            },
            ControlFlow::Switch(targets, default) => {
                let cases = targets
                    .into_iter()
                    .map(|(case, target)| (case, StructuralBlockId::from_pc(target)))
                    .collect();
                BlockExit::Switch {
                    cases,
                    default: StructuralBlockId::from_pc(default),
                }
            }
            ControlFlow::Terminal => BlockExit::Terminal,
            ControlFlow::Fallthrough => BlockExit::Fallthrough {
                target: StructuralBlockId::from_pc(self.require_next_pc(pc)?),
            },
            ControlFlow::Legacy => unreachable!("Rejected"),
        };
        Ok(block_exit)
    }

    fn exceptional_successors(&self, pc: ProgramCounter) -> Vec<ExceptionalTarget> {
        let mut successors = Vec::new();
        let mut has_catch_all = false;
        for entry in self
            .body
            .exception_table
            .iter()
            .filter(|entry| entry.covers(pc))
        {
            let catch_type = entry.catch_type.clone();
            successors.push(ExceptionalTarget::Handler {
                id: HandlerId::from_pc(entry.handler_pc),
                catch_type,
            });
            has_catch_all = entry.catches_all();
            if has_catch_all {
                break;
            }
        }
        if !has_catch_all {
            successors.push(ExceptionalTarget::Unwind);
        }
        successors
    }
}

/// The decoded instruction PCs of one structural block, before its identity is
/// resolved.
struct BlockGroup {
    start_pc: ProgramCounter,
    end_pc: ProgramCounter,
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
