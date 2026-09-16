use std::collections::{BTreeMap, BTreeSet};

use super::{
    fallibility::Fallibility,
    instruction_flow::InstructionFlow,
    model::{
        Block, BytecodeCfg, ExceptionalTarget, HandlerEntry, HandlerId, StructuralBlockId,
        StructuralTerminator,
    },
};
use crate::{
    ir::generator::error::{Error, MalformedBytecode},
    jvm::{
        Method,
        code::{ExceptionTableEntry, Instruction, MethodBody, ProgramCounter},
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

        let leaders = self.collect_leaders(entry_pc)?;
        let (mut blocks, block_by_start_pc) = self.partition(&leaders)?;
        let handlers = self.build_handlers(&block_by_start_pc)?;

        for block in &mut blocks {
            let final_pc = *block
                .instruction_pcs
                .last()
                .expect("a structural block is never empty");
            let instruction = self
                .body
                .instruction_at(final_pc)
                .expect("a structural block PC comes from decoded bytecode");
            let flow = InstructionFlow::classify(self.body, final_pc, instruction)?;
            block.terminator =
                flow.resolve(|target| Self::block_id_at_pc(&block_by_start_pc, target))?;
            if self.fallibility.is_synchronously_fallible(instruction) {
                block.exceptional_successors = self.exceptional_successors(final_pc);
            }
        }
        Self::resolve_ret_continuations(&mut blocks, &handlers)?;

        let entry = Self::block_id_at_pc(&block_by_start_pc, entry_pc)?;
        Ok(BytecodeCfg {
            entry,
            blocks,
            handlers,
        })
    }

    fn resolve_ret_continuations(
        blocks: &mut [Block],
        handlers: &[HandlerEntry],
    ) -> Result<(), Error> {
        let calls = blocks
            .iter()
            .filter_map(|block| match block.terminator {
                StructuralTerminator::Jsr {
                    target,
                    continuation,
                } => Some((target, continuation)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut candidates =
            BTreeMap::<StructuralBlockId, BTreeMap<ProgramCounter, StructuralBlockId>>::new();

        for (subroutine_entry, continuation) in calls {
            let continuation_pc = blocks
                .get(continuation.index())
                .ok_or_else(|| Error::internal("a jsr continuation has no structural block"))?
                .start_pc;
            let mut pending = BTreeSet::from([subroutine_entry]);
            let mut visited = BTreeSet::new();
            while let Some(id) = pending.pop_first() {
                if !visited.insert(id) {
                    continue;
                }
                let block = blocks.get(id.index()).ok_or_else(|| {
                    Error::internal("a subroutine traversal reached no structural block")
                })?;
                for target in &block.exceptional_successors {
                    if let ExceptionalTarget::Handler(handler) = target {
                        pending.insert(
                            handlers
                                .get(handler.index())
                                .ok_or_else(|| {
                                    Error::internal("an exception edge has no handler entry")
                                })?
                                .target,
                        );
                    }
                }
                match &block.terminator {
                    StructuralTerminator::Fallthrough { target }
                    | StructuralTerminator::Goto { target } => {
                        pending.insert(*target);
                    }
                    StructuralTerminator::Branch {
                        taken, fallthrough, ..
                    } => {
                        pending.extend([*taken, *fallthrough]);
                    }
                    StructuralTerminator::Switch { cases, default } => {
                        pending.extend(cases.values().copied());
                        pending.insert(*default);
                    }
                    // Nested subroutines resume in the current subroutine at the continuation.
                    StructuralTerminator::Jsr { continuation, .. } => {
                        pending.insert(*continuation);
                    }
                    StructuralTerminator::Ret { .. } => {
                        candidates
                            .entry(id)
                            .or_default()
                            .insert(continuation_pc, continuation);
                    }
                    StructuralTerminator::Return { .. } | StructuralTerminator::Throw => {}
                }
            }
        }

        for (ret, continuations) in candidates {
            let block = blocks
                .get_mut(ret.index())
                .ok_or_else(|| Error::internal("a ret candidate has no structural block"))?;
            let StructuralTerminator::Ret {
                continuations: resolved,
                ..
            } = &mut block.terminator
            else {
                return Err(Error::internal("a ret candidate is not a ret block"));
            };
            *resolved = continuations;
        }
        Ok(())
    }

    fn collect_leaders(&self, entry_pc: ProgramCounter) -> Result<BTreeSet<ProgramCounter>, Error> {
        let mut leaders = BTreeSet::from([entry_pc]);
        leaders.extend(
            self.body
                .exception_table
                .iter()
                .map(|entry| entry.handler_pc),
        );

        for (&pc, instruction) in self.body.instructions.iter() {
            let flow = InstructionFlow::classify(self.body, pc, instruction)?;
            match flow {
                InstructionFlow::Fallthrough { target } => {
                    if self.fallibility.is_synchronously_fallible(instruction) {
                        leaders.insert(target);
                    }
                }
                InstructionFlow::Goto { target } => {
                    leaders.insert(target);
                    self.insert_instruction_after(&mut leaders, pc);
                }
                InstructionFlow::Branch {
                    taken, fallthrough, ..
                } => {
                    leaders.insert(taken);
                    leaders.insert(fallthrough);
                }
                InstructionFlow::TableSwitch {
                    jump_targets,
                    default,
                    ..
                } => {
                    for target in jump_targets.iter().copied().chain([default]) {
                        leaders.insert(target);
                    }
                    self.insert_instruction_after(&mut leaders, pc);
                }
                InstructionFlow::LookupSwitch {
                    match_targets,
                    default,
                } => {
                    for target in match_targets.values().copied().chain([default]) {
                        leaders.insert(target);
                    }
                    self.insert_instruction_after(&mut leaders, pc);
                }
                InstructionFlow::Jsr {
                    target,
                    continuation,
                } => {
                    leaders.insert(target);
                    leaders.insert(continuation);
                }
                InstructionFlow::Ret { .. }
                | InstructionFlow::Return { .. }
                | InstructionFlow::Throw => {
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

    fn partition(
        &self,
        leaders: &BTreeSet<ProgramCounter>,
    ) -> Result<(Vec<Block>, BTreeMap<ProgramCounter, StructuralBlockId>), Error> {
        let mut blocks = Vec::with_capacity(leaders.len());
        let mut block_by_start_pc = BTreeMap::new();
        for (&pc, _) in self.body.instructions.iter() {
            if leaders.contains(&pc) {
                let id = StructuralBlockId::from_index(blocks.len());
                blocks.push(Block {
                    id,
                    start_pc: pc,
                    instruction_pcs: Vec::new(),
                    // Filled after every block start has an identity.
                    terminator: StructuralTerminator::Return {
                        operand: super::model::ReturnOperand::Void,
                    },
                    exceptional_successors: Vec::new(),
                });
                block_by_start_pc.insert(pc, id);
            }
            let block = blocks
                .last_mut()
                .ok_or_else(|| Error::internal_at(pc, "decoded bytecode has no entry block"))?;
            block.instruction_pcs.push(pc);
        }
        Ok((blocks, block_by_start_pc))
    }

    fn build_handlers(
        &self,
        block_by_start_pc: &BTreeMap<ProgramCounter, StructuralBlockId>,
    ) -> Result<Vec<HandlerEntry>, Error> {
        self.body
            .exception_table
            .iter()
            .map(|entry| {
                Ok(HandlerEntry {
                    handler_pc: entry.handler_pc,
                    catch_type: entry.catch_type.clone(),
                    target: Self::block_id_at_pc(block_by_start_pc, entry.handler_pc)?,
                })
            })
            .collect()
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

    fn exceptional_successors(&self, pc: ProgramCounter) -> Vec<ExceptionalTarget> {
        let mut successors = Vec::new();
        let mut has_catch_all = false;
        for (index, entry) in self
            .body
            .exception_table
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.covers(pc))
        {
            successors.push(ExceptionalTarget::Handler(HandlerId::from_index(index)));
            has_catch_all = catches_everything(entry);
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

fn catches_everything(entry: &ExceptionTableEntry) -> bool {
    entry
        .catch_type
        .as_ref()
        .is_none_or(|caught| caught.0.as_ref() == "java/lang/Throwable")
}
