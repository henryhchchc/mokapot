use std::collections::{BTreeMap, BTreeSet};

use super::{
    fallibility::Fallibility,
    model::{
        Block, BlockExit, BytecodeCfg, ExceptionalTarget, HandlerEntry, HandlerId,
        StructuralBlockId,
    },
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
        let (groups, block_by_start_pc) = self.partition(&block_leaders)?;
        let (handlers, handler_by_pc) = self.build_handlers(&block_by_start_pc)?;
        let blocks = self.build_blocks(groups, &block_by_start_pc, &handler_by_pc)?;

        let entry = Self::block_id_at_pc(&block_by_start_pc, entry_pc)?;
        Ok(BytecodeCfg {
            entry,
            blocks,
            handlers,
        })
    }

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
        Ok(leaders)
    }

    fn require_next_pc(&self, pc: ProgramCounter) -> Result<ProgramCounter, Error> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingFallthrough))
    }

    /// Assigns every block start PC its dense identity and groups the decoded
    /// instruction PCs under it.
    ///
    /// No block is built here: exits resolve through
    /// `block_by_start_pc`, so they can only be derived once every start PC has
    /// an identity.
    fn partition(
        &self,
        leaders: &BTreeSet<ProgramCounter>,
    ) -> Result<(Vec<BlockGroup>, BTreeMap<ProgramCounter, StructuralBlockId>), Error> {
        let mut groups: Vec<BlockGroup> = Vec::with_capacity(leaders.len());
        let mut block_by_start_pc = BTreeMap::new();
        for (pc, _) in self.body.instructions.iter() {
            if leaders.contains(&pc) {
                let id = StructuralBlockId::from_index(groups.len());
                groups.push(BlockGroup {
                    start_pc: pc,
                    end_pc: pc,
                });
                block_by_start_pc.insert(pc, id);
            } else {
                let group = groups
                    .last_mut()
                    .ok_or_else(|| Error::internal_at(pc, "decoded bytecode has no entry block"))?;
                group.end_pc = pc;
            }
        }
        Ok((groups, block_by_start_pc))
    }

    /// Builds every block together with its exit and exceptional
    /// successors, which follow from the block's final instruction alone.
    fn build_blocks(
        &self,
        groups: Vec<BlockGroup>,
        block_by_start_pc: &BTreeMap<ProgramCounter, StructuralBlockId>,
        handler_by_pc: &BTreeMap<ProgramCounter, HandlerId>,
    ) -> Result<Vec<Block>, Error> {
        groups
            .into_iter()
            .map(|group| {
                let final_pc = group.end_pc;
                let instruction = self
                    .body
                    .instruction_at(final_pc)
                    .expect("a structural block PC comes from decoded bytecode");
                let exit = self.build_exit(final_pc, instruction, block_by_start_pc)?;
                let exceptional_successors = if self.fallibility.can_throw(instruction) {
                    self.exceptional_successors(final_pc, handler_by_pc)?
                } else {
                    Vec::new()
                };
                Ok(Block {
                    start_pc: group.start_pc,
                    end_pc: group.end_pc,
                    exit,
                    exceptional_successors,
                })
            })
            .collect()
    }

    fn build_exit(
        &self,
        pc: ProgramCounter,
        instruction: &Instruction,
        block_by_start_pc: &BTreeMap<ProgramCounter, StructuralBlockId>,
    ) -> Result<BlockExit, Error> {
        let block_at = |target| Self::block_id_at_pc(block_by_start_pc, target);
        let block_exit = match instruction.control_flow() {
            ControlFlow::Branch(target) => {
                let taken = block_at(target)?;
                let fallthrough = block_at(self.require_next_pc(pc)?)?;
                BlockExit::Branch { taken, fallthrough }
            }
            ControlFlow::Goto(target) => BlockExit::Goto {
                target: block_at(target)?,
            },
            ControlFlow::Switch(targets, default) => {
                let cases = targets
                    .into_iter()
                    .map(|(case, target)| block_at(target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?;
                let default = block_at(default)?;
                BlockExit::Switch { cases, default }
            }
            ControlFlow::Terminal => BlockExit::Terminal,
            ControlFlow::Fallthrough => BlockExit::Fallthrough {
                target: block_at(self.require_next_pc(pc)?)?,
            },
            ControlFlow::Legacy => unreachable!("Rejected"),
        };
        Ok(block_exit)
    }

    fn build_handlers(
        &self,
        block_by_start_pc: &BTreeMap<ProgramCounter, StructuralBlockId>,
    ) -> Result<(Vec<HandlerEntry>, BTreeMap<ProgramCounter, HandlerId>), Error> {
        let mut handlers = Vec::new();
        let mut handler_by_pc = BTreeMap::new();
        for entry in &self.body.exception_table {
            if handler_by_pc.contains_key(&entry.handler_pc) {
                continue;
            }
            let id = HandlerId::from_index(handlers.len());
            handlers.push(HandlerEntry {
                target: Self::block_id_at_pc(block_by_start_pc, entry.handler_pc)?,
            });
            handler_by_pc.insert(entry.handler_pc, id);
        }
        Ok((handlers, handler_by_pc))
    }

    fn block_id_at_pc(
        block_by_start_pc: &BTreeMap<ProgramCounter, StructuralBlockId>,
        target: ProgramCounter,
    ) -> Result<StructuralBlockId, Error> {
        block_by_start_pc
            .get(&target)
            .copied()
            .ok_or_else(|| Error::malformed(Some(target), MalformedBytecode::MissingInstruction))
    }

    fn exceptional_successors(
        &self,
        pc: ProgramCounter,
        handler_by_pc: &BTreeMap<ProgramCounter, HandlerId>,
    ) -> Result<Vec<ExceptionalTarget>, Error> {
        let mut successors = Vec::new();
        let mut has_catch_all = false;
        for entry in self
            .body
            .exception_table
            .iter()
            .filter(|entry| entry.covers(pc))
        {
            let id = *handler_by_pc.get(&entry.handler_pc).ok_or_else(|| {
                Error::internal_at(
                    entry.handler_pc,
                    "an exception-table arm has no handler entry",
                )
            })?;
            let catch_type = entry.catch_type.clone();
            successors.push(ExceptionalTarget::Handler { id, catch_type });
            has_catch_all = entry.catches_all();
            if has_catch_all {
                break;
            }
        }
        if !has_catch_all {
            successors.push(ExceptionalTarget::Unwind);
        }
        Ok(successors)
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
