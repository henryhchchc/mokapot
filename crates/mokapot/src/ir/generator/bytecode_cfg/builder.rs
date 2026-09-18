use std::collections::{BTreeMap, BTreeSet};

use super::{
    fallibility::Fallibility,
    instruction_flow::PcFlow,
    model::{Block, BytecodeCfg, ExceptionalTarget, HandlerEntry, HandlerId, StructuralBlockId},
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
        Ok(Self {
            body,
            fallibility: Fallibility::for_method(method),
        })
    }

    pub(super) fn build(self) -> Result<BytecodeCfg, Error> {
        let (entry_pc, _) = self
            .body
            .instructions
            .entry_point()
            .ok_or_else(|| Error::malformed(None, MalformedBytecode::MissingEntry))?;
        self.validate_instructions()?;
        self.validate_handler_targets()?;
        let flows = self.classify_instructions()?;

        let leaders = self.collect_leaders(entry_pc, &flows)?;
        let (groups, block_by_start_pc) = self.partition(&leaders)?;
        let (handlers, handler_by_pc) = self.build_handlers(&block_by_start_pc)?;
        let blocks = self.build_blocks(groups, &block_by_start_pc, &handler_by_pc, &flows)?;

        let entry = Self::block_id_at_pc(&block_by_start_pc, entry_pc)?;
        Ok(BytecodeCfg {
            entry,
            blocks,
            handlers,
        })
    }

    fn classify_instructions(&self) -> Result<BTreeMap<ProgramCounter, PcFlow>, Error> {
        self.body
            .instructions
            .iter()
            .map(|(&pc, instruction)| {
                PcFlow::classify(self.body, pc, instruction).map(|flow| (pc, flow))
            })
            .collect()
    }

    fn collect_leaders(
        &self,
        entry_pc: ProgramCounter,
        flows: &BTreeMap<ProgramCounter, PcFlow>,
    ) -> Result<BTreeSet<ProgramCounter>, Error> {
        let mut leaders = BTreeSet::from([entry_pc]);
        leaders.extend(
            self.body
                .exception_table
                .iter()
                .map(|entry| entry.handler_pc),
        );

        for (&pc, instruction) in self.body.instructions.iter() {
            let flow = flows
                .get(&pc)
                .ok_or_else(|| Error::internal_at(pc, "a decoded instruction has no flow"))?;
            match flow {
                PcFlow::Fallthrough { target } => {
                    if self.fallibility.is_synchronously_fallible(instruction) {
                        leaders.insert(*target);
                    }
                }
                PcFlow::Goto { target } => {
                    leaders.insert(*target);
                    self.insert_instruction_after(&mut leaders, pc);
                }
                PcFlow::Branch {
                    taken, fallthrough, ..
                } => {
                    leaders.insert(*taken);
                    leaders.insert(*fallthrough);
                }
                PcFlow::Switch { cases, default } => {
                    for target in cases.values().copied().chain([*default]) {
                        leaders.insert(target);
                    }
                    self.insert_instruction_after(&mut leaders, pc);
                }
                PcFlow::Return { .. } | PcFlow::Throw => {
                    self.insert_instruction_after(&mut leaders, pc);
                }
            }
        }
        Ok(leaders)
    }

    fn insert_instruction_after(&self, leaders: &mut BTreeSet<ProgramCounter>, pc: ProgramCounter) {
        if let Some(next_pc) = self.body.instructions.next_pc_of(&pc) {
            leaders.insert(next_pc);
        }
    }

    fn validate_handler_targets(&self) -> Result<(), Error> {
        for entry in &self.body.exception_table {
            self.validate_target(entry.handler_pc)?;
            let range = &entry.covered_pc;
            if range.start >= range.end || self.body.instruction_at(range.start).is_none() {
                return Err(Error::malformed(
                    Some(range.start),
                    MalformedBytecode::InvalidExceptionRange,
                ));
            }
            if !self.is_instruction_boundary(range.end) {
                return Err(Error::malformed(
                    Some(range.end),
                    MalformedBytecode::InvalidExceptionRange,
                ));
            }
        }
        Ok(())
    }

    fn is_instruction_boundary(&self, pc: ProgramCounter) -> bool {
        self.body.instruction_at(pc).is_some()
            || self.body.instructions.iter().any(|(&start, instruction)| {
                instruction
                    .encoded_end_pc(start)
                    .is_some_and(|end| end == pc)
            })
    }

    fn validate_instructions(&self) -> Result<(), Error> {
        for (&pc, instruction) in self.body.instructions.iter() {
            if matches!(
                instruction,
                Instruction::Jsr(_)
                    | Instruction::JsrW(_)
                    | Instruction::Ret(_)
                    | Instruction::Wide(WideInstruction::Ret(_))
            ) {
                return Err(Error::UnsupportedBytecode {
                    pc,
                    kind: UnsupportedBytecode::LegacySubroutine,
                });
            }
            if let Instruction::TableSwitch {
                range,
                jump_targets,
                ..
            } = instruction
            {
                let count = (*range.start() <= *range.end())
                    .then(|| i64::from(*range.end()) - i64::from(*range.start()))
                    .and_then(|span| span.checked_add(1))
                    .and_then(|count| usize::try_from(count).ok());
                if count != Some(jump_targets.len()) {
                    return Err(Error::malformed(
                        Some(pc),
                        MalformedBytecode::InvalidTableSwitch,
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_target(&self, target: ProgramCounter) -> Result<(), Error> {
        self.body
            .instruction_at(target)
            .is_some()
            .then_some(())
            .ok_or_else(|| Error::malformed(Some(target), MalformedBytecode::MissingInstruction))
    }

    /// Assigns every block start PC its dense identity and groups the decoded
    /// instruction PCs under it.
    ///
    /// No block is built here: terminators resolve through
    /// `block_by_start_pc`, so they can only be derived once every start PC has
    /// an identity.
    fn partition(
        &self,
        leaders: &BTreeSet<ProgramCounter>,
    ) -> Result<(Vec<BlockGroup>, BTreeMap<ProgramCounter, StructuralBlockId>), Error> {
        let mut groups: Vec<BlockGroup> = Vec::with_capacity(leaders.len());
        let mut block_by_start_pc = BTreeMap::new();
        for (&pc, _) in self.body.instructions.iter() {
            if leaders.contains(&pc) {
                let id = StructuralBlockId::from_index(groups.len());
                groups.push(BlockGroup {
                    start_pc: pc,
                    instruction_pcs: vec![pc],
                });
                block_by_start_pc.insert(pc, id);
            } else {
                let group = groups
                    .last_mut()
                    .ok_or_else(|| Error::internal_at(pc, "decoded bytecode has no entry block"))?;
                group.instruction_pcs.push(pc);
            }
        }
        Ok((groups, block_by_start_pc))
    }

    /// Builds every block together with its terminator and exceptional
    /// successors, which follow from the block's final instruction alone.
    fn build_blocks(
        &self,
        groups: Vec<BlockGroup>,
        block_by_start_pc: &BTreeMap<ProgramCounter, StructuralBlockId>,
        handler_by_pc: &BTreeMap<ProgramCounter, HandlerId>,
        flows: &BTreeMap<ProgramCounter, PcFlow>,
    ) -> Result<Vec<Block>, Error> {
        groups
            .into_iter()
            .map(|group| {
                let final_pc = *group
                    .instruction_pcs
                    .last()
                    .expect("a structural block is never empty");
                let instruction = self
                    .body
                    .instruction_at(final_pc)
                    .expect("a structural block PC comes from decoded bytecode");
                let flow = flows.get(&final_pc).ok_or_else(|| {
                    Error::internal_at(final_pc, "a decoded instruction has no flow")
                })?;
                let terminator =
                    flow.resolve(|target| Self::block_id_at_pc(block_by_start_pc, target))?;
                let exceptional_successors =
                    if self.fallibility.is_synchronously_fallible(instruction) {
                        self.exceptional_successors(final_pc, handler_by_pc)?
                    } else {
                        Vec::new()
                    };
                Ok(Block {
                    start_pc: group.start_pc,
                    instruction_pcs: group.instruction_pcs,
                    terminator,
                    exceptional_successors,
                })
            })
            .collect()
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
                handler_pc: entry.handler_pc,
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
    instruction_pcs: Vec<ProgramCounter>,
}

fn catches_everything(entry: &ExceptionTableEntry) -> bool {
    entry
        .catch_type
        .as_ref()
        .is_none_or(|caught| caught.0.as_ref() == "java/lang/Throwable")
}
