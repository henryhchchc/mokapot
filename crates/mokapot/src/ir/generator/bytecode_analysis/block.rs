//! Reachable block-level frame analysis and direct scalar-phi construction.

use std::collections::{BTreeMap, BTreeSet};

use super::{Executor, RegisterInstruction, Value, jvm};
use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind, TryMapValues,
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, PathValue},
        },
        expression::{Condition, Expression},
        generator::{
            bytecode_cfg::{self, BytecodeCfg},
            error::{Error, MalformedBytecode},
            identity::SsaValueId,
            ssa::{PhiCandidate, ScalarBlock, UnfinalizedGraph},
        },
    },
    jvm::{Method, code::ProgramCounter},
};

type Frame = jvm::Frame<Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Location {
    Bytecode(bytecode_cfg::BlockId),
    Handler(bytecode_cfg::HandlerId),
    Unwind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Predecessor {
    Entry,
    Location(Location),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PhiSite {
    location: Location,
    position: jvm::Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhiKind {
    Scalar,
    ReturnAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawSuccessor {
    target: Location,
    transfer: ControlTransfer<Value>,
    frame: Frame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawBlock {
    caught_exception: Option<SsaValueId>,
    operations: Vec<(ProgramCounter, OperationKind<Value>)>,
    terminator: TerminatorKind<Value>,
    terminator_source: Option<ProgramCounter>,
    successors: Vec<RawSuccessor>,
}

struct Analyzer<'method, 'cfg> {
    cfg: &'cfg BytecodeCfg,
    executor: Executor<'method>,
    contributions: BTreeMap<Location, BTreeMap<Predecessor, Frame>>,
    entry_frames: BTreeMap<Location, Frame>,
    executed: BTreeMap<Location, RawBlock>,
    output_frames: BTreeMap<Location, BTreeMap<Location, Frame>>,
    phi_values: BTreeMap<PhiSite, SsaValueId>,
    phi_kinds: BTreeMap<PhiSite, PhiKind>,
    return_tokens: BTreeMap<SsaValueId, BTreeSet<ProgramCounter>>,
    caught_exceptions: BTreeMap<bytecode_cfg::HandlerId, SsaValueId>,
}

pub(super) fn analyze(method: &Method, cfg: &BytecodeCfg) -> Result<UnfinalizedGraph, Error> {
    Analyzer::new(method, cfg)?.run()
}

impl<'method, 'cfg> Analyzer<'method, 'cfg> {
    fn new(method: &'method Method, cfg: &'cfg BytecodeCfg) -> Result<Self, Error> {
        Ok(Self {
            cfg,
            executor: Executor::for_method(method)?,
            contributions: BTreeMap::new(),
            entry_frames: BTreeMap::new(),
            executed: BTreeMap::new(),
            output_frames: BTreeMap::new(),
            phi_values: BTreeMap::new(),
            phi_kinds: BTreeMap::new(),
            return_tokens: BTreeMap::new(),
            caught_exceptions: BTreeMap::new(),
        })
    }

    fn run(mut self) -> Result<UnfinalizedGraph, Error> {
        let entry = Location::Bytecode(self.cfg.entry_block());
        self.contributions
            .entry(entry)
            .or_default()
            .insert(Predecessor::Entry, self.executor.initial_frame.clone());
        self.recompute_entry(entry)?;

        let mut pending = BTreeSet::from([entry]);
        while let Some(location) = pending.pop_first() {
            let input = self
                .entry_frames
                .get(&location)
                .cloned()
                .ok_or_else(|| Error::internal("a pending block has no entry frame"))?;
            let block = self.execute(location, input)?;
            let new_outputs = coalesce_output_frames(&block.successors)?;
            let old_outputs = self
                .output_frames
                .insert(location, new_outputs.clone())
                .unwrap_or_default();
            self.executed.insert(location, block);

            let affected = old_outputs
                .keys()
                .chain(new_outputs.keys())
                .copied()
                .collect::<BTreeSet<_>>();
            for target in affected {
                let inputs = self.contributions.entry(target).or_default();
                if let Some(frame) = new_outputs.get(&target) {
                    inputs.insert(Predecessor::Location(location), frame.clone());
                } else {
                    inputs.remove(&Predecessor::Location(location));
                }
                if inputs.is_empty() {
                    self.contributions.remove(&target);
                    self.entry_frames.remove(&target);
                    continue;
                }
                let old_tokens = self.return_tokens.clone();
                if self.recompute_entry(target)? {
                    pending.insert(target);
                }
                if self.return_tokens != old_tokens {
                    pending.extend(self.entry_frames.keys().copied());
                }
            }
        }

        self.finish(entry)
    }

    fn recompute_entry(&mut self, location: Location) -> Result<bool, Error> {
        let inputs = self
            .contributions
            .get(&location)
            .ok_or_else(|| Error::internal("a reachable block has no frame contributions"))?;
        let mut frames = inputs.values();
        let mut merged = frames
            .next()
            .cloned()
            .ok_or_else(|| Error::internal("a reachable block has no predecessor frame"))?;
        for contribution in frames.cloned() {
            let phi_values = &mut self.phi_values;
            let phi_kinds = &mut self.phi_kinds;
            let return_tokens = &mut self.return_tokens;
            let allocator = &mut self.executor.value_id_allocator;
            let mut merge_error = None;
            merged
                .merge_from_with(contribution, |position, lhs, rhs| {
                    match merge_value(
                        PhiSite { location, position },
                        lhs,
                        rhs,
                        phi_values,
                        phi_kinds,
                        return_tokens,
                        allocator,
                    ) {
                        Ok(changed) => changed,
                        Err(error) => {
                            merge_error = Some(error);
                            false
                        }
                    }
                })
                .map_err(|source| {
                    Error::from(source).at_instruction_if_present(self.pc(location))
                })?;
            if let Some(error) = merge_error {
                return Err(error.at_instruction_if_present(self.pc(location)));
            }
        }
        let changed = self.entry_frames.get(&location) != Some(&merged);
        self.entry_frames.insert(location, merged);
        Ok(changed)
    }

    fn execute(&mut self, location: Location, input: Frame) -> Result<RawBlock, Error> {
        match location {
            Location::Bytecode(id) => self.execute_bytecode(id, input),
            Location::Handler(id) => self.execute_handler(id, input),
            Location::Unwind => Ok(RawBlock {
                caught_exception: None,
                operations: Vec::new(),
                terminator: TerminatorKind::Unwind,
                terminator_source: None,
                successors: Vec::new(),
            }),
        }
    }

    fn execute_handler(
        &mut self,
        id: bytecode_cfg::HandlerId,
        input: Frame,
    ) -> Result<RawBlock, Error> {
        let handler = self
            .cfg
            .handler(id)
            .ok_or_else(|| Error::internal("a handler location has no structural entry"))?;
        let caught = input
            .handler_exception()
            .map_err(Error::from)
            .and_then(|value| scalar_value(*value, true))?;
        Ok(RawBlock {
            caught_exception: Some(caught),
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            successors: vec![RawSuccessor {
                target: Location::Bytecode(handler.target),
                transfer: ControlTransfer::Unconditional,
                frame: input,
            }],
        })
    }

    fn execute_bytecode(
        &mut self,
        id: bytecode_cfg::BlockId,
        input: Frame,
    ) -> Result<RawBlock, Error> {
        let block = self
            .cfg
            .block(id)
            .ok_or_else(|| Error::internal("a bytecode location has no structural block"))?
            .clone();
        let mut frame = input;
        let mut operations = Vec::new();
        let mut final_instruction = None;
        let mut final_input = None;
        let jsr_continuation = self.jsr_continuation(&block)?;
        for &pc in &block.instruction_pcs {
            let instruction = self
                .executor
                .body
                .instruction_at(pc)
                .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingInstruction))?
                .clone();
            let instruction_input = frame.clone();
            let lifted = self
                .executor
                .lift_register_instruction(&instruction, pc, &mut frame, jsr_continuation)
                .map_err(|error| error.at_instruction(pc))?;
            if let Some(operation) = operation_of(&lifted) {
                if let RegisterInstruction::Subroutine {
                    value,
                    continuation,
                    ..
                } = lifted
                {
                    self.return_tokens
                        .insert(value, BTreeSet::from([continuation]));
                }
                operations.push((pc, operation));
            }
            final_instruction = Some(lifted);
            final_input = Some(instruction_input);
        }
        let pc = *block
            .instruction_pcs
            .last()
            .ok_or_else(|| Error::internal("a structural bytecode block is empty"))?;
        let lifted = final_instruction
            .ok_or_else(|| Error::internal("a structural bytecode block was not lifted"))?;
        let exceptional_input = final_input
            .ok_or_else(|| Error::internal("a structural bytecode block has no final input"))?;

        let (terminator, mut successors) =
            self.normal_successors(&block, &lifted, &frame, jsr_continuation)?;
        for target in &block.exceptional_successors {
            successors.push(self.exception_successor(*target, &exceptional_input)?);
        }
        let explicit = !matches!(
            block.terminator,
            bytecode_cfg::Terminator::Fallthrough { .. }
        );
        Ok(RawBlock {
            caught_exception: None,
            operations,
            terminator,
            terminator_source: explicit.then_some(pc),
            successors,
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the match keeps every structural terminator shape in one exhaustive dispatch"
    )]
    fn normal_successors(
        &self,
        block: &bytecode_cfg::Block,
        lifted: &RegisterInstruction,
        frame: &Frame,
        jsr_continuation: Option<ProgramCounter>,
    ) -> Result<(TerminatorKind<Value>, Vec<RawSuccessor>), Error> {
        use bytecode_cfg::Terminator as T;
        let result = match (&block.terminator, lifted) {
            (T::Fallthrough { target }, _) => (
                if block.exceptional_successors.is_empty() {
                    TerminatorKind::Goto
                } else {
                    TerminatorKind::Fallible
                },
                vec![unconditional(*target, frame)],
            ),
            (
                T::Goto { target },
                RegisterInstruction::Jump {
                    condition: None, ..
                },
            ) => (TerminatorKind::Goto, vec![unconditional(*target, frame)]),
            (
                T::Branch { taken, fallthrough },
                RegisterInstruction::Jump {
                    condition: Some(condition),
                    ..
                },
            ) => {
                let condition: BooleanVariable<_> = condition.clone().into();
                (
                    TerminatorKind::Branch,
                    vec![
                        RawSuccessor {
                            target: Location::Bytecode(*taken),
                            transfer: ControlTransfer::Conditional(BranchGuard::of(
                                condition.clone(),
                            )),
                            frame: frame.clone(),
                        },
                        RawSuccessor {
                            target: Location::Bytecode(*fallthrough),
                            transfer: ControlTransfer::Conditional(BranchGuard::of(!condition)),
                            frame: frame.clone(),
                        },
                    ],
                )
            }
            (T::Switch { cases, default }, RegisterInstruction::Switch { match_value, .. }) => {
                let mut successors = cases
                    .iter()
                    .map(|(&case, &target)| RawSuccessor {
                        target: Location::Bytecode(target),
                        transfer: ControlTransfer::Conditional(BranchGuard::of(
                            BooleanVariable::Positive(Condition::Equal(
                                (*match_value).into(),
                                PathValue::Constant(crate::jvm::ConstantValue::Integer(case)),
                            )),
                        )),
                        frame: frame.clone(),
                    })
                    .collect::<Vec<_>>();
                let default_guard = cases
                    .keys()
                    .map(|case| {
                        BooleanVariable::Negative(Condition::Equal(
                            (*match_value).into(),
                            PathValue::Constant(crate::jvm::ConstantValue::Integer(*case)),
                        ))
                    })
                    .collect();
                successors.push(RawSuccessor {
                    target: Location::Bytecode(*default),
                    transfer: ControlTransfer::Conditional(default_guard),
                    frame: frame.clone(),
                });
                (
                    TerminatorKind::Switch {
                        match_value: *match_value,
                    },
                    successors,
                )
            }
            (
                T::Jsr { target, .. },
                RegisterInstruction::Subroutine {
                    continuation: lifted_continuation,
                    ..
                },
            ) => {
                let continuation = jsr_continuation.ok_or_else(|| {
                    Error::internal("a structural jsr has no resolved continuation")
                })?;
                if continuation != *lifted_continuation {
                    return Err(Error::internal_at(
                        block.start_pc,
                        "structural and lifted jsr continuations disagree",
                    ));
                }
                (
                    TerminatorKind::SubroutineCall,
                    vec![RawSuccessor {
                        target: Location::Bytecode(*target),
                        transfer: ControlTransfer::SubroutineCall { continuation },
                        frame: frame.clone(),
                    }],
                )
            }
            (T::Ret { .. }, RegisterInstruction::SubroutineReturn(address)) => {
                let Value::ReturnAddress(address) = *address else {
                    return Err(Error::malformed(
                        block.instruction_pcs.last().copied(),
                        MalformedBytecode::InvalidSubroutineReturn,
                    ));
                };
                let tokens = self.return_tokens.get(&address).ok_or_else(|| {
                    Error::malformed(
                        block.instruction_pcs.last().copied(),
                        MalformedBytecode::InvalidSubroutineReturn,
                    )
                })?;
                let successors = tokens
                    .iter()
                    .map(|&continuation| {
                        let target = self.block_at_pc(continuation)?;
                        Ok(RawSuccessor {
                            target: Location::Bytecode(target),
                            transfer: ControlTransfer::SubroutineReturn {
                                continuation,
                                guard: BranchGuard::of(BooleanVariable::Positive(
                                    Condition::Equal(
                                        PathValue::Variable(Value::Ssa(address)),
                                        PathValue::ReturnAddress(continuation),
                                    ),
                                )),
                            },
                            frame: frame.clone(),
                        })
                    })
                    .collect::<Result<_, Error>>()?;
                (
                    TerminatorKind::SubroutineReturn {
                        address: Value::Ssa(address),
                    },
                    successors,
                )
            }
            (T::Return, RegisterInstruction::Return(value)) => {
                (TerminatorKind::Return(*value), Vec::new())
            }
            (T::Throw, RegisterInstruction::Throw(value)) => {
                (TerminatorKind::Throw(*value), Vec::new())
            }
            _ => {
                return Err(Error::internal_at(
                    block.start_pc,
                    "structural and lifted block terminators disagree",
                ));
            }
        };
        Ok(result)
    }

    fn exception_successor(
        &mut self,
        target: bytecode_cfg::Target,
        input: &Frame,
    ) -> Result<RawSuccessor, Error> {
        match target {
            bytecode_cfg::Target::Handler(id) => {
                let handler = self
                    .cfg
                    .handler(id)
                    .ok_or_else(|| Error::internal("an exception edge has no handler entry"))?;
                let caught = if let Some(&value) = self.caught_exceptions.get(&id) {
                    value
                } else {
                    let value = self.executor.new_value_id()?;
                    self.caught_exceptions.insert(id, value);
                    value
                };
                Ok(RawSuccessor {
                    target: Location::Handler(id),
                    transfer: ControlTransfer::Exception(handler.catch_type.clone()),
                    frame: input.clone().exception_handler_frame(Value::Ssa(caught))?,
                })
            }
            bytecode_cfg::Target::Unwind => Ok(RawSuccessor {
                target: Location::Unwind,
                transfer: ControlTransfer::Unwind,
                frame: input.clone().into_unwind_frame(),
            }),
        }
    }

    fn block_at_pc(&self, pc: ProgramCounter) -> Result<bytecode_cfg::BlockId, Error> {
        self.cfg
            .blocks()
            .find(|block| block.start_pc == pc)
            .map(|block| block.id)
            .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingInstruction))
    }

    fn jsr_continuation(
        &self,
        block: &bytecode_cfg::Block,
    ) -> Result<Option<ProgramCounter>, Error> {
        match block.terminator {
            bytecode_cfg::Terminator::Jsr { continuation, .. } => self
                .cfg
                .block(continuation)
                .map(|block| Some(block.start_pc))
                .ok_or_else(|| Error::internal("a jsr continuation has no structural block")),
            _ => Ok(None),
        }
    }

    fn pc(&self, location: Location) -> Option<ProgramCounter> {
        match location {
            Location::Bytecode(id) => self.cfg.block(id).map(|block| block.start_pc),
            Location::Handler(id) => self.cfg.handler(id).map(|handler| handler.handler_pc),
            Location::Unwind => None,
        }
    }

    fn finish(self, structural_entry: Location) -> Result<UnfinalizedGraph, Error> {
        let has_preheader = self
            .contributions
            .get(&structural_entry)
            .is_some_and(|inputs| inputs.keys().any(|pred| *pred != Predecessor::Entry));
        let locations = self.executed.keys().copied().collect::<Vec<_>>();
        let offset = usize::from(has_preheader);
        let public_ids = locations
            .iter()
            .enumerate()
            .map(|(index, &location)| public_block_id(index + offset).map(|id| (location, id)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let preheader = has_preheader.then(|| BlockId::new(0));
        let entry = preheader.unwrap_or(public_ids[&structural_entry]);

        let mut blocks = Vec::with_capacity(locations.len() + offset);
        if let Some(id) = preheader {
            blocks.push(ScalarBlock {
                id,
                caught_exception: None,
                operations: Vec::new(),
                terminator: TerminatorKind::Goto,
                terminator_source: None,
                successors: vec![super::super::ssa::Successor {
                    target: public_ids[&structural_entry],
                    transfer: ControlTransfer::Unconditional,
                }],
            });
        }
        for location in &locations {
            let raw = self
                .executed
                .get(location)
                .cloned()
                .ok_or_else(|| Error::internal("a reachable location was not executed"))?;
            blocks.push(lower_block(raw, public_ids[location], &public_ids)?);
        }

        let mut phi_candidates = BTreeMap::new();
        for (site, &value) in &self.phi_values {
            if !public_ids.contains_key(&site.location) {
                continue;
            }
            let inputs = self
                .contributions
                .get(&site.location)
                .ok_or_else(|| Error::internal("a phi site has no frame contributions"))?
                .iter()
                .map(|(predecessor, frame)| {
                    let predecessor = match predecessor {
                        Predecessor::Entry => preheader.ok_or_else(|| {
                            Error::internal("an entry phi has no synthetic predecessor")
                        })?,
                        Predecessor::Location(location) => public_ids[location],
                    };
                    let input = frame.value_at(site.position).copied().ok_or_else(|| {
                        Error::internal("a phi input frame lacks its merged slot")
                    })?;
                    Ok((predecessor, scalar_value(input, true)?))
                })
                .collect::<Result<Vec<_>, Error>>()?;
            phi_candidates.insert(
                value,
                PhiCandidate {
                    placement: public_ids[&site.location],
                    inputs,
                },
            );
        }

        Ok(UnfinalizedGraph {
            entry,
            blocks,
            phi_candidates,
            this_value: self.executor.receiver_value,
            parameter_values: self.executor.parameter_values,
        })
    }
}

fn merge_value(
    site: PhiSite,
    lhs: &mut Value,
    rhs: Value,
    phi_values: &mut BTreeMap<PhiSite, SsaValueId>,
    phi_kinds: &mut BTreeMap<PhiSite, PhiKind>,
    return_tokens: &mut BTreeMap<SsaValueId, BTreeSet<ProgramCounter>>,
    allocator: &mut super::execution::ValueIdAllocator,
) -> Result<bool, Error> {
    if *lhs == rhs {
        return Ok(false);
    }
    let kind = match (*lhs, rhs) {
        (Value::Ssa(_), Value::Ssa(_)) => PhiKind::Scalar,
        (Value::ReturnAddress(_), Value::ReturnAddress(_)) => PhiKind::ReturnAddress,
        _ => {
            let changed = *lhs != Value::Invalid;
            *lhs = Value::Invalid;
            return Ok(changed);
        }
    };
    if phi_kinds
        .get(&site)
        .is_some_and(|existing| *existing != kind)
    {
        let changed = *lhs != Value::Invalid;
        *lhs = Value::Invalid;
        return Ok(changed);
    }
    let value = if let Some(&value) = phi_values.get(&site) {
        value
    } else {
        let value = allocator.new_value_id()?;
        phi_values.insert(site, value);
        phi_kinds.insert(site, kind);
        value
    };
    if kind == PhiKind::ReturnAddress {
        let tokens = [*lhs, rhs]
            .into_iter()
            .filter_map(|value| match value {
                Value::ReturnAddress(value) => return_tokens.get(&value),
                Value::Ssa(_) | Value::Invalid => None,
            })
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        return_tokens.entry(value).or_default().extend(tokens);
    }
    let merged = match kind {
        PhiKind::Scalar => Value::Ssa(value),
        PhiKind::ReturnAddress => Value::ReturnAddress(value),
    };
    let changed = *lhs != merged;
    *lhs = merged;
    Ok(changed)
}

fn operation_of(instruction: &RegisterInstruction) -> Option<OperationKind<Value>> {
    match instruction {
        RegisterInstruction::Definition { value, expr } => Some(OperationKind::Definition {
            value: Value::Ssa(*value),
            expr: expr.clone(),
        }),
        RegisterInstruction::Effect(expr) => Some(OperationKind::Effect { expr: expr.clone() }),
        RegisterInstruction::Subroutine {
            value,
            continuation,
            ..
        } => Some(OperationKind::Definition {
            value: Value::Ssa(*value),
            expr: Expression::ReturnAddress(*continuation),
        }),
        _ => None,
    }
}

fn unconditional(target: bytecode_cfg::BlockId, frame: &Frame) -> RawSuccessor {
    RawSuccessor {
        target: Location::Bytecode(target),
        transfer: ControlTransfer::Unconditional,
        frame: frame.clone(),
    }
}

fn coalesce_output_frames(successors: &[RawSuccessor]) -> Result<BTreeMap<Location, Frame>, Error> {
    let mut outputs = BTreeMap::new();
    for successor in successors {
        if let Some(existing) = outputs.insert(successor.target, successor.frame.clone())
            && existing != successor.frame
        {
            return Err(Error::internal(
                "parallel edges from one block carry different JVM frames",
            ));
        }
    }
    Ok(outputs)
}

const fn scalar_value(value: Value, allow_return_address: bool) -> Result<SsaValueId, Error> {
    match value {
        Value::Ssa(value) => Ok(value),
        Value::ReturnAddress(value) if allow_return_address => Ok(value),
        Value::ReturnAddress(_) | Value::Invalid => {
            Err(Error::malformed(None, MalformedBytecode::InvalidFrameValue))
        }
    }
}

fn lower_block(
    raw: RawBlock,
    id: BlockId,
    public_ids: &BTreeMap<Location, BlockId>,
) -> Result<ScalarBlock, Error> {
    Ok(ScalarBlock {
        id,
        caught_exception: raw.caught_exception,
        operations: raw
            .operations
            .into_iter()
            .map(|(pc, operation)| {
                operation
                    .try_map_values(|value| scalar_value(value, false))
                    .map(|operation| (pc, operation))
                    .map_err(|error| error.at_instruction(pc))
            })
            .collect::<Result<_, _>>()?,
        terminator: raw
            .terminator
            .try_map_values(|value| scalar_value(value, false))?,
        terminator_source: raw.terminator_source,
        successors: raw
            .successors
            .into_iter()
            .map(|successor| {
                Ok(super::super::ssa::Successor {
                    target: public_ids[&successor.target],
                    transfer: successor
                        .transfer
                        .try_map_values(|value| scalar_value(value, false))?,
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}

fn public_block_id(index: usize) -> Result<BlockId, Error> {
    u32::try_from(index)
        .map(BlockId::new)
        .map_err(|_| Error::internal("the block identity space is exhausted"))
}
