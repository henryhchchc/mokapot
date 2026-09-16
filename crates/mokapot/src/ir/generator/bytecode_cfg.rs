//! A structural control-flow graph over decoded JVM bytecode.
//!
//! This phase deliberately precedes frame analysis.  It keeps every decoded
//! instruction, including bytecode unreachable from method entry, and records
//! legacy `jsr`/`ret` without inferring dynamic return targets.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    bytecode_analysis::fallibility,
    error::{Error, MalformedBytecode},
};
use crate::jvm::{
    Method,
    code::{ExceptionTableEntry, Instruction, MethodBody, ProgramCounter, WideInstruction},
    references::ClassRef,
};

/// Builds the decoded-bytecode CFG used by the later block analyzer.
pub(super) fn build(method: &Method) -> Result<BytecodeCfg, Error> {
    let body = method.body.as_ref().ok_or(Error::NoMethodBody)?;
    Builder {
        body,
        fallibility: fallibility::Context::for_method(method),
    }
    .build()
}

/// A dense identifier for a structural bytecode block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct BlockId(usize);

impl BlockId {
    const fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub(super) const fn index(self) -> usize {
        self.0
    }
}

/// A dense identifier for a synthetic exception-handler entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct HandlerId(usize);

impl HandlerId {
    const fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub(super) const fn index(self) -> usize {
        self.0
    }
}

/// The target of a structural control-flow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    /// A synthetic entry that installs the caught exception before its block.
    Handler(HandlerId),
    /// The synthetic exit for an exception that escapes the method.
    Unwind,
}

/// One synthetic exception-handler entry.
#[derive(Debug, Clone)]
pub(super) struct HandlerEntry {
    /// The handler's bytecode entry PC.
    pub handler_pc: ProgramCounter,
    /// The exception type selected by this table entry, or `None` for catch-all.
    pub catch_type: Option<ClassRef>,
    /// The decoded bytecode entered after materializing the caught exception.
    pub target: BlockId,
}

/// The ordinary transfer ending a structural block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Terminator {
    /// Ordinary execution continues at the following block.
    Fallthrough { target: BlockId },
    /// An unconditional static jump.
    Goto { target: BlockId },
    /// A conditional static jump and its required fallthrough.
    Branch {
        taken: BlockId,
        fallthrough: BlockId,
    },
    /// A static switch dispatch.  Each key is retained, including targets that coincide.
    Switch {
        cases: BTreeMap<i32, BlockId>,
        default: BlockId,
    },
    /// A legacy subroutine call and its static return continuation.
    Jsr {
        target: BlockId,
        continuation: BlockId,
    },
    /// A legacy subroutine return whose dynamic target is not yet resolved.
    Ret { local: u16 },
    /// A normal method exit.
    Return,
    /// An explicit `athrow`; exceptional successors select handlers or unwind.
    Throw,
}

/// A maximal bytecode block ending at an ordinary transfer or fallible instruction.
#[derive(Debug, Clone)]
pub(super) struct Block {
    /// Its dense identity.
    pub id: BlockId,
    /// The first decoded instruction in the block.
    pub start_pc: ProgramCounter,
    /// Every raw bytecode PC belonging to the block, in bytecode order.
    pub instruction_pcs: Vec<ProgramCounter>,
    /// The ordinary transfer after the final instruction.
    pub terminator: Terminator,
    /// Ordered exceptional successors of the final fallible instruction.
    pub exceptional_successors: Vec<Target>,
}

/// A block-first CFG that preserves decoded JVM bytecode structure.
#[derive(Debug, Clone)]
pub(super) struct BytecodeCfg {
    entry: BlockId,
    blocks: Vec<Block>,
    handlers: Vec<HandlerEntry>,
}

impl BytecodeCfg {
    /// The block containing the first decoded instruction.
    pub const fn entry_block(&self) -> BlockId {
        self.entry
    }

    /// Returns all bytecode blocks in dense ID order.
    pub fn blocks(&self) -> impl ExactSizeIterator<Item = &Block> {
        self.blocks.iter()
    }

    /// Looks up a bytecode block by its dense identity.
    pub fn block(&self, id: BlockId) -> Option<&Block> {
        self.blocks.get(id.index())
    }

    /// Looks up a synthetic handler entry by its dense identity.
    pub fn handler(&self, id: HandlerId) -> Option<&HandlerEntry> {
        self.handlers.get(id.index())
    }
}

struct Builder<'a> {
    body: &'a MethodBody,
    fallibility: fallibility::Context,
}

impl Builder<'_> {
    fn build(self) -> Result<BytecodeCfg, Error> {
        let (entry_pc, _) = self
            .body
            .instructions
            .entry_point()
            .ok_or_else(|| Error::malformed(None, MalformedBytecode::MissingEntry))?;
        self.validate_instructions()?;
        self.validate_handler_targets()?;

        let leaders = self.collect_leaders(entry_pc)?;
        let (mut blocks, block_ids) = self.partition(&leaders)?;
        let handlers = self.build_handlers(&block_ids)?;

        for block in &mut blocks {
            let final_pc = *block
                .instruction_pcs
                .last()
                .expect("a structural block is never empty");
            let instruction = self
                .body
                .instruction_at(final_pc)
                .expect("a structural block PC comes from decoded bytecode");
            block.terminator = self.terminator(final_pc, instruction, &block_ids)?;
            if self.fallibility.is_synchronously_fallible(instruction) {
                block.exceptional_successors = self.exceptional_successors(final_pc);
            }
        }

        let entry = Self::block_id(&block_ids, entry_pc)?;
        Ok(BytecodeCfg {
            entry,
            blocks,
            handlers,
        })
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
            for target in static_targets(instruction) {
                self.validate_target(pc, target)?;
                leaders.insert(target);
            }

            if requires_fallthrough(instruction) {
                // Every ordinary fallthrough must be valid, but only a
                // fallible instruction ends a maximal straight-line block.
                let next_pc = self.next_pc(pc)?;
                if self.fallibility.is_synchronously_fallible(instruction) {
                    leaders.insert(next_pc);
                }
            }
            if is_control_transfer(instruction)
                && let Some(next_pc) = self.body.instructions.next_pc_of(&pc)
            {
                leaders.insert(next_pc);
            }
        }
        Ok(leaders)
    }

    fn validate_handler_targets(&self) -> Result<(), Error> {
        for entry in &self.body.exception_table {
            self.validate_target(entry.handler_pc, entry.handler_pc)?;
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

    fn validate_target(
        &self,
        _source: ProgramCounter,
        target: ProgramCounter,
    ) -> Result<(), Error> {
        self.body
            .instruction_at(target)
            .is_some()
            .then_some(())
            .ok_or_else(|| Error::malformed(Some(target), MalformedBytecode::MissingInstruction))
    }

    fn next_pc(&self, pc: ProgramCounter) -> Result<ProgramCounter, Error> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingFallthrough))
    }

    fn partition(
        &self,
        leaders: &BTreeSet<ProgramCounter>,
    ) -> Result<(Vec<Block>, BTreeMap<ProgramCounter, BlockId>), Error> {
        let mut blocks = Vec::with_capacity(leaders.len());
        let mut block_ids = BTreeMap::new();
        for (&pc, _) in self.body.instructions.iter() {
            if leaders.contains(&pc) {
                let id = BlockId::from_index(blocks.len());
                blocks.push(Block {
                    id,
                    start_pc: pc,
                    instruction_pcs: Vec::new(),
                    // Filled after every PC has a block identity.
                    terminator: Terminator::Return,
                    exceptional_successors: Vec::new(),
                });
            }
            let block = blocks
                .last_mut()
                .ok_or_else(|| Error::internal_at(pc, "decoded bytecode has no entry block"))?;
            block.instruction_pcs.push(pc);
            block_ids.insert(pc, block.id);
        }
        Ok((blocks, block_ids))
    }

    fn build_handlers(
        &self,
        block_ids: &BTreeMap<ProgramCounter, BlockId>,
    ) -> Result<Vec<HandlerEntry>, Error> {
        self.body
            .exception_table
            .iter()
            .map(|entry| {
                Ok(HandlerEntry {
                    handler_pc: entry.handler_pc,
                    catch_type: entry.catch_type.clone(),
                    target: Self::block_id(block_ids, entry.handler_pc)?,
                })
            })
            .collect()
    }

    fn terminator(
        &self,
        pc: ProgramCounter,
        instruction: &Instruction,
        block_ids: &BTreeMap<ProgramCounter, BlockId>,
    ) -> Result<Terminator, Error> {
        let block_at = |target| Self::block_id(block_ids, target);
        let fallthrough = || self.next_pc(pc).and_then(&block_at);
        let terminator = match instruction {
            Instruction::Goto(target) | Instruction::GotoW(target) => Terminator::Goto {
                target: block_at(*target)?,
            },
            instruction if is_conditional_branch(instruction) => {
                let target = static_targets(instruction)
                    .next()
                    .expect("conditional branches have one static target");
                Terminator::Branch {
                    taken: block_at(target)?,
                    fallthrough: fallthrough()?,
                }
            }
            Instruction::TableSwitch {
                range,
                jump_targets,
                default,
            } => Terminator::Switch {
                cases: range
                    .clone()
                    .zip(jump_targets)
                    .map(|(case, target)| block_at(*target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?,
                default: block_at(*default)?,
            },
            Instruction::LookupSwitch {
                default,
                match_targets,
            } => Terminator::Switch {
                cases: match_targets
                    .iter()
                    .map(|(&case, &target)| block_at(target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?,
                default: block_at(*default)?,
            },
            Instruction::Jsr(target) | Instruction::JsrW(target) => Terminator::Jsr {
                target: block_at(*target)?,
                continuation: fallthrough()?,
            },
            Instruction::Ret(local) => Terminator::Ret {
                local: u16::from(*local),
            },
            Instruction::Wide(WideInstruction::Ret(local)) => Terminator::Ret { local: *local },
            Instruction::IReturn
            | Instruction::LReturn
            | Instruction::FReturn
            | Instruction::DReturn
            | Instruction::AReturn
            | Instruction::Return => Terminator::Return,
            Instruction::AThrow => Terminator::Throw,
            _ => Terminator::Fallthrough {
                target: fallthrough()?,
            },
        };
        Ok(terminator)
    }

    fn block_id(
        block_ids: &BTreeMap<ProgramCounter, BlockId>,
        target: ProgramCounter,
    ) -> Result<BlockId, Error> {
        block_ids
            .get(&target)
            .copied()
            .ok_or_else(|| Error::malformed(Some(target), MalformedBytecode::MissingInstruction))
    }

    fn exceptional_successors(&self, pc: ProgramCounter) -> Vec<Target> {
        let mut successors = Vec::new();
        let mut has_catch_all = false;
        for (index, entry) in self
            .body
            .exception_table
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.covers(pc))
        {
            successors.push(Target::Handler(HandlerId::from_index(index)));
            has_catch_all = catches_everything(entry);
            if has_catch_all {
                break;
            }
        }
        if !has_catch_all {
            successors.push(Target::Unwind);
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

fn static_targets(instruction: &Instruction) -> impl Iterator<Item = ProgramCounter> + '_ {
    let targets: Box<dyn Iterator<Item = ProgramCounter> + '_> = match instruction {
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
        | Instruction::IfNonNull(target)
        | Instruction::Goto(target)
        | Instruction::GotoW(target)
        | Instruction::Jsr(target)
        | Instruction::JsrW(target) => Box::new(std::iter::once(*target)),
        Instruction::TableSwitch {
            jump_targets,
            default,
            ..
        } => Box::new(
            jump_targets
                .iter()
                .copied()
                .chain(std::iter::once(*default)),
        ),
        Instruction::LookupSwitch {
            match_targets,
            default,
        } => Box::new(
            match_targets
                .values()
                .copied()
                .chain(std::iter::once(*default)),
        ),
        _ => Box::new(std::iter::empty()),
    };
    targets
}

const fn is_conditional_branch(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::IfEq(_)
            | Instruction::IfNe(_)
            | Instruction::IfLt(_)
            | Instruction::IfGe(_)
            | Instruction::IfGt(_)
            | Instruction::IfLe(_)
            | Instruction::IfICmpEq(_)
            | Instruction::IfICmpNe(_)
            | Instruction::IfICmpLt(_)
            | Instruction::IfICmpGe(_)
            | Instruction::IfICmpGt(_)
            | Instruction::IfICmpLe(_)
            | Instruction::IfACmpEq(_)
            | Instruction::IfACmpNe(_)
            | Instruction::IfNull(_)
            | Instruction::IfNonNull(_)
    )
}

const fn is_control_transfer(instruction: &Instruction) -> bool {
    is_conditional_branch(instruction)
        || matches!(
            instruction,
            Instruction::Goto(_)
                | Instruction::GotoW(_)
                | Instruction::Jsr(_)
                | Instruction::JsrW(_)
                | Instruction::Ret(_)
                | Instruction::Wide(WideInstruction::Ret(_))
                | Instruction::TableSwitch { .. }
                | Instruction::LookupSwitch { .. }
                | Instruction::IReturn
                | Instruction::LReturn
                | Instruction::FReturn
                | Instruction::DReturn
                | Instruction::AReturn
                | Instruction::Return
                | Instruction::AThrow
        )
}

const fn requires_fallthrough(instruction: &Instruction) -> bool {
    !matches!(
        instruction,
        Instruction::Goto(_)
            | Instruction::GotoW(_)
            | Instruction::Ret(_)
            | Instruction::Wide(WideInstruction::Ret(_))
            | Instruction::TableSwitch { .. }
            | Instruction::LookupSwitch { .. }
            | Instruction::IReturn
            | Instruction::LReturn
            | Instruction::FReturn
            | Instruction::DReturn
            | Instruction::AReturn
            | Instruction::Return
            | Instruction::AThrow
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        ir::generator::tests::method,
        jvm::code::{ExceptionTableEntry, Instruction},
    };

    #[test]
    fn partitions_unreachable_bytecode_after_control_transfers() {
        let method = method(
            [
                (0, Instruction::Nop),
                (1, Instruction::Goto(4.into())),
                (2, Instruction::Nop),
                (3, Instruction::Return),
                (4, Instruction::Return),
            ],
            "()V",
            vec![],
        );

        let cfg = build(&method).unwrap();
        let blocks = cfg.blocks().collect::<Vec<_>>();

        assert_eq!(
            blocks
                .iter()
                .map(|block| block.instruction_pcs.as_slice())
                .collect::<Vec<_>>(),
            vec![
                &[0.into(), 1.into()][..],
                &[2.into(), 3.into()][..],
                &[4.into()][..]
            ]
        );
        assert_eq!(cfg.entry_block(), blocks[0].id);
        assert_eq!(
            blocks[0].terminator,
            Terminator::Goto {
                target: blocks[2].id
            }
        );
    }

    #[test]
    fn keeps_fallible_boundaries_and_synthetic_exception_targets() {
        let method = method(
            [
                (0, Instruction::AConstNull),
                (
                    1,
                    Instruction::CheckCast("java/lang/String".parse().unwrap()),
                ),
                (2, Instruction::Return),
                (10, Instruction::AStore0),
                (11, Instruction::Return),
            ],
            "()V",
            vec![ExceptionTableEntry {
                covered_pc: 1.into()..2.into(),
                handler_pc: 10.into(),
                catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
            }],
        );

        let cfg = build(&method).unwrap();
        let blocks = cfg.blocks().collect::<Vec<_>>();
        let fallible = blocks
            .iter()
            .find(|block| block.instruction_pcs.contains(&1.into()))
            .unwrap();
        let handler_id = HandlerId::from_index(0);
        let handler = cfg.handler(handler_id).unwrap();

        assert_eq!(fallible.instruction_pcs, [0.into(), 1.into()]);
        assert_eq!(
            fallible.terminator,
            Terminator::Fallthrough {
                target: blocks
                    .iter()
                    .find(|block| block.start_pc == 2.into())
                    .unwrap()
                    .id,
            }
        );
        assert_eq!(
            fallible.exceptional_successors,
            vec![Target::Handler(handler_id), Target::Unwind]
        );
        assert_eq!(
            handler.target,
            blocks
                .iter()
                .find(|block| block.start_pc == 10.into())
                .unwrap()
                .id
        );
    }

    #[test]
    fn validates_targets_even_in_unreachable_bytecode() {
        let method = method(
            [(0, Instruction::Return), (1, Instruction::Goto(10.into()))],
            "()V",
            vec![],
        );

        let error = build(&method).unwrap_err();

        assert!(matches!(
            error,
            Error::MalformedBytecode {
                pc: Some(pc),
                kind: MalformedBytecode::MissingInstruction,
            } if pc == 10.into()
        ));
    }

    #[test]
    fn validates_required_fallthroughs_in_unreachable_bytecode() {
        let method = method(
            [(0, Instruction::Return), (1, Instruction::Nop)],
            "()V",
            vec![],
        );

        let error = build(&method).unwrap_err();

        assert!(matches!(
            error,
            Error::MalformedBytecode {
                pc: Some(pc),
                kind: MalformedBytecode::MissingFallthrough,
            } if pc == 1.into()
        ));
    }

    #[test]
    fn rejects_mismatched_table_switch_cardinality() {
        let method = method(
            [
                (
                    0,
                    Instruction::TableSwitch {
                        range: 1..=3,
                        jump_targets: vec![10.into(), 10.into()],
                        default: 10.into(),
                    },
                ),
                (10, Instruction::Return),
            ],
            "()V",
            vec![],
        );

        assert!(matches!(
            build(&method),
            Err(Error::MalformedBytecode {
                pc: Some(pc),
                kind: MalformedBytecode::InvalidTableSwitch,
            }) if pc == 0.into()
        ));
    }

    #[test]
    fn rejects_reversed_table_switch_ranges_even_without_targets() {
        let method = method(
            [
                (
                    0,
                    Instruction::TableSwitch {
                        range: std::ops::RangeInclusive::new(3, 1),
                        jump_targets: vec![],
                        default: 10.into(),
                    },
                ),
                (10, Instruction::Return),
            ],
            "()V",
            vec![],
        );

        assert!(matches!(
            build(&method),
            Err(Error::MalformedBytecode {
                pc: Some(pc),
                kind: MalformedBytecode::InvalidTableSwitch,
            }) if pc == 0.into()
        ));
    }

    #[test]
    fn rejects_reversed_exception_ranges() {
        let method = method(
            [(0, Instruction::Return), (10, Instruction::Return)],
            "()V",
            vec![ExceptionTableEntry {
                covered_pc: 10.into()..0.into(),
                handler_pc: 10.into(),
                catch_type: None,
            }],
        );

        assert!(matches!(
            build(&method),
            Err(Error::MalformedBytecode {
                pc: Some(pc),
                kind: MalformedBytecode::InvalidExceptionRange,
            }) if pc == 10.into()
        ));
    }

    #[test]
    fn rejects_exception_range_end_between_instruction_boundaries() {
        let method = method(
            [(0, Instruction::SiPush(0)), (3, Instruction::Return)],
            "()V",
            vec![ExceptionTableEntry {
                covered_pc: 0.into()..2.into(),
                handler_pc: 3.into(),
                catch_type: None,
            }],
        );

        assert!(matches!(
            build(&method),
            Err(Error::MalformedBytecode {
                pc: Some(pc),
                kind: MalformedBytecode::InvalidExceptionRange,
            }) if pc == 2.into()
        ));
    }

    #[test]
    fn accepts_exception_range_ending_after_the_last_instruction_start() {
        let method = method(
            [
                (0, Instruction::Nop),
                (3, Instruction::Return),
                (4, Instruction::Return),
            ],
            "()V",
            vec![ExceptionTableEntry {
                covered_pc: 0.into()..5.into(),
                handler_pc: 4.into(),
                catch_type: None,
            }],
        );

        build(&method).unwrap();
    }

    #[test]
    fn retains_switch_case_labels_and_parallel_targets() {
        let method = method(
            [
                (
                    0,
                    Instruction::LookupSwitch {
                        default: 3.into(),
                        match_targets: BTreeMap::from([(1, 2.into()), (2, 2.into())]),
                    },
                ),
                (1, Instruction::Return),
                (2, Instruction::Return),
                (3, Instruction::Return),
            ],
            "()V",
            vec![],
        );

        let cfg = build(&method).unwrap();
        let switch = cfg.blocks().next().unwrap();
        let target_at = |pc: u16| {
            cfg.blocks()
                .find(|block| block.start_pc == pc.into())
                .unwrap()
                .id
        };

        assert_eq!(
            switch.terminator,
            Terminator::Switch {
                cases: BTreeMap::from([(1, target_at(2)), (2, target_at(2))]),
                default: target_at(3),
            }
        );
    }

    #[test]
    fn represents_jsr_continuations_and_unresolved_ret() {
        let method = method(
            [
                (0, Instruction::Jsr(4.into())),
                (1, Instruction::Return),
                (4, Instruction::Ret(0)),
                (5, Instruction::Return),
            ],
            "()V",
            vec![],
        );

        let cfg = build(&method).unwrap();
        let jsr = cfg
            .blocks()
            .find(|block| block.start_pc == 0.into())
            .unwrap();
        let ret = cfg
            .blocks()
            .find(|block| block.start_pc == 4.into())
            .unwrap();

        assert_eq!(
            jsr.terminator,
            Terminator::Jsr {
                target: cfg
                    .blocks()
                    .find(|block| block.start_pc == 4.into())
                    .unwrap()
                    .id,
                continuation: cfg
                    .blocks()
                    .find(|block| block.start_pc == 1.into())
                    .unwrap()
                    .id,
            }
        );
        assert_eq!(ret.terminator, Terminator::Ret { local: 0 });
    }
}
