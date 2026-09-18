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
        code::{ExceptionTableEntry, Instruction, MethodBody, ProgramCounter, WideInstruction},
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
        self.reject_legacy_subroutines()?;
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
        use Instruction::{
            AReturn, AThrow, DReturn, FReturn, Goto, GotoW, IReturn, Jsr, JsrW, LReturn,
            LookupSwitch, Ret, Return, TableSwitch, Wide,
        };

        let mut leaders = BTreeSet::from([entry_pc]);
        let iter = self.body.exception_table.iter().map(|it| it.handler_pc);
        leaders.extend(iter);

        for (pc, instruction) in self.body.instructions.iter() {
            if let Some(target) = conditional_target(instruction) {
                leaders.insert(target);
                leaders.insert(self.require_next_pc(pc)?);
                continue;
            }
            match instruction {
                Jsr(_) | JsrW(_) | Ret(_) | Wide(WideInstruction::Ret(_)) => {
                    unreachable!("explicitly rejected")
                }
                Goto(target) | GotoW(target) => {
                    leaders.insert(*target);
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                TableSwitch {
                    jump_targets,
                    default,
                    ..
                } => {
                    leaders.extend(jump_targets.iter().copied().chain([*default]));
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                LookupSwitch {
                    match_targets,
                    default,
                } => {
                    leaders.extend(match_targets.values().copied().chain([*default]));
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                IReturn | LReturn | FReturn | DReturn | AReturn | Return | AThrow => {
                    leaders.extend(self.body.instructions.next_pc_of(&pc));
                }
                _ if self.fallibility.can_throw(instruction) => {
                    leaders.insert(self.require_next_pc(pc)?);
                }
                _ => {}
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

    fn reject_legacy_subroutines(&self) -> Result<(), Error> {
        use Instruction::{Jsr, JsrW, Ret, Wide};
        use WideInstruction::Ret as WRet;
        if let Some(pc) = self.body.instructions.iter().find_map(|(pc, it)| {
            matches!(it, Jsr(_) | JsrW(_) | Ret(_) | Wide(WRet(_))).then_some(pc)
        }) {
            let kind = UnsupportedBytecode::LegacySubroutine;
            return Err(Error::UnsupportedBytecode { pc, kind });
        }
        Ok(())
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
        if let Some(target) = conditional_target(instruction) {
            return Ok(BlockExit::Branch {
                taken: block_at(target)?,
                fallthrough: block_at(self.require_next_pc(pc)?)?,
            });
        }
        let exit = match instruction {
            Instruction::Goto(target) | Instruction::GotoW(target) => BlockExit::Goto {
                target: block_at(*target)?,
            },
            Instruction::TableSwitch {
                range,
                jump_targets,
                default,
            } => BlockExit::Switch {
                cases: range
                    .clone()
                    .zip(jump_targets)
                    .map(|(case, &target)| block_at(target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?,
                default: block_at(*default)?,
            },
            Instruction::LookupSwitch {
                match_targets,
                default,
            } => BlockExit::Switch {
                cases: match_targets
                    .iter()
                    .map(|(&case, &target)| block_at(target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?,
                default: block_at(*default)?,
            },
            Instruction::IReturn
            | Instruction::LReturn
            | Instruction::FReturn
            | Instruction::DReturn
            | Instruction::AReturn
            | Instruction::Return
            | Instruction::AThrow => BlockExit::Terminal,
            _ => BlockExit::Fallthrough {
                target: block_at(self.require_next_pc(pc)?)?,
            },
        };
        Ok(exit)
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
            successors.push(ExceptionalTarget::Handler {
                id,
                catch_type: entry.catch_type.clone(),
            });
            has_catch_all = catches_everything(entry);
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

fn catches_everything(entry: &ExceptionTableEntry) -> bool {
    entry
        .catch_type
        .as_ref()
        .is_none_or(|caught| caught.0.as_ref() == "java/lang/Throwable")
}

const fn conditional_target(instruction: &Instruction) -> Option<ProgramCounter> {
    match instruction {
        Instruction::IfEq(target)
        | Instruction::IfNe(target)
        | Instruction::IfLt(target)
        | Instruction::IfGe(target)
        | Instruction::IfGt(target)
        | Instruction::IfLe(target)
        | Instruction::IfICmpEq(target)
        | Instruction::IfICmpNe(target)
        | Instruction::IfICmpLt(target)
        | Instruction::IfICmpGe(target)
        | Instruction::IfICmpGt(target)
        | Instruction::IfICmpLe(target)
        | Instruction::IfACmpEq(target)
        | Instruction::IfACmpNe(target)
        | Instruction::IfNull(target)
        | Instruction::IfNonNull(target) => Some(*target),
        _ => None,
    }
}
