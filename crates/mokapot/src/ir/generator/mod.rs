mod jvm_frame;
mod lifting;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    iter::once,
    mem,
};

use jvm_frame::Entry;
pub use jvm_frame::ExecutionError;

use self::jvm_frame::JvmStackFrame;
use super::{
    BasicBlock, BlockId, EdgeId, Identifier, InstructionId, InstructionKind, MokaIRMethod,
    MokaInstruction, Operand, SourceMap, Successor, Terminator, TerminatorKind, ValueId,
    control_flow::ControlTransfer,
    expression::{Condition, Expression},
};
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::control_flow::path_condition::{BooleanVariable, BranchGuard, Value},
    jvm::{
        ConstantValue, Method,
        code::{MethodBody, ProgramCounter},
        method,
    },
};

#[derive(Debug, Clone)]
enum LiftedInstruction {
    Nop,
    Definition {
        value: ValueId,
        expr: Expression,
    },
    Jump {
        condition: Option<Condition>,
        target: ProgramCounter,
    },
    Switch {
        match_value: Operand,
        branches: BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Return(Option<Operand>),
    Throw(Operand),
    Subroutine {
        value: ValueId,
        target: ProgramCounter,
        return_address: ProgramCounter,
    },
    SubroutineReturn(Operand),
}

impl LiftedInstruction {
    const fn is_explicit_transfer(&self) -> bool {
        matches!(
            self,
            Self::Jump { .. }
                | Self::Switch { .. }
                | Self::Return(_)
                | Self::Throw(_)
                | Self::Subroutine { .. }
                | Self::SubroutineReturn(_)
        )
    }
}

/// An error that occurs when generating Moka IR.
#[derive(Debug, thiserror::Error)]
pub enum MokaIRBrewingError {
    /// An error that occurs when executing bytecode on a JVM frame.
    #[error("Error when executing bytecode on a JVM frame: {0}")]
    ExecutionError(#[from] ExecutionError),
    /// An error that occurs when merging two stack frames.
    #[error("Error when merging two stack frames: {0}")]
    MergeError(ExecutionError),
    /// An error that occurs when a method does not have a body.
    #[error("The method does not have a body")]
    NoMethodBody,
    /// An error that occurs when the method contains malformed control flow.
    #[error("The method contains malformed control flow")]
    MalformedControlFlow,
}

struct MokaIRGenerator<'method> {
    lifted: BTreeMap<ProgramCounter, LiftedInstruction>,
    outgoing: BTreeMap<ProgramCounter, Vec<(ProgramCounter, ControlTransfer)>>,
    value_ids: BTreeMap<ProgramCounter, ValueId>,
    caught_exception_ids: BTreeMap<ProgramCounter, ValueId>,
    body: &'method MethodBody,
    initial_seed: Option<(ProgramCounter, JvmStackFrame)>,
}

impl DataflowProblem for MokaIRGenerator<'_> {
    type Location = ProgramCounter;
    type Fact = JvmStackFrame;
    type Err = MokaIRBrewingError;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        self.initial_seed.clone().into_iter().collect::<Vec<_>>()
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        let location = *location;
        let mut frame = fact.same_frame();
        let jvm_instruction = self
            .body
            .instruction_at(location)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let ir_instruction = self.lift_instruction(jvm_instruction, location, &mut frame)?;
        let edges_and_frames =
            self.analyze_frame_and_conditions(location, frame, &ir_instruction)?;
        self.lifted.insert(location, ir_instruction);
        self.outgoing.insert(
            location,
            edges_and_frames
                .iter()
                .map(|(target, transfer, _)| (*target, transfer.clone()))
                .collect(),
        );
        Ok(edges_and_frames
            .into_iter()
            .map(|(target, _, frame)| (target, frame))
            .collect::<Vec<_>>())
    }
}

impl<'method> MokaIRGenerator<'method> {
    fn for_method(method: &'method Method) -> Result<Self, MokaIRBrewingError> {
        let body = method
            .body
            .as_ref()
            .ok_or(MokaIRBrewingError::NoMethodBody)?;
        let first_pc = body
            .instructions
            .entry_point()
            .map(|(pc, _)| *pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let initial_frame = JvmStackFrame::new(
            method.access_flags.contains(method::AccessFlags::STATIC),
            &method.descriptor,
            body.max_locals,
            body.max_stack,
        )?;

        let value_ids = body
            .instructions
            .iter()
            .enumerate()
            .map(|(index, (pc, _))| {
                u32::try_from(index)
                    .map(|index| (*pc, ValueId::new(index)))
                    .map_err(|_| MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let first_caught_id =
            u32::try_from(value_ids.len()).map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
        let mut caught_exception_ids = BTreeMap::new();
        for handler_pc in body.exception_table.iter().map(|entry| entry.handler_pc) {
            if !caught_exception_ids.contains_key(&handler_pc) {
                let offset = u32::try_from(caught_exception_ids.len())
                    .map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
                let id = first_caught_id
                    .checked_add(offset)
                    .map(ValueId::new)
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                caught_exception_ids.insert(handler_pc, id);
            }
        }

        Ok(Self {
            lifted: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            value_ids,
            caught_exception_ids,
            body,
            initial_seed: Some((first_pc, initial_frame)),
        })
    }

    fn value_at(&self, pc: ProgramCounter) -> Result<ValueId, MokaIRBrewingError> {
        self.value_ids
            .get(&pc)
            .copied()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)
    }

    fn next_pc_of(&self, pc: ProgramCounter) -> Result<ProgramCounter, MokaIRBrewingError> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)
    }

    fn exception_edges(
        &self,
        pc: ProgramCounter,
        frame: &JvmStackFrame,
    ) -> Result<Vec<(ProgramCounter, ControlTransfer, JvmStackFrame)>, MokaIRBrewingError> {
        self.body
            .exception_table
            .iter()
            .filter(|entry| entry.covers(pc))
            .map(|entry| {
                let caught_type = entry
                    .catch_type
                    .clone()
                    .unwrap_or_else(|| "java/lang/Throwable".parse().expect("valid class name"));
                let caught_id = self
                    .caught_exception_ids
                    .get(&entry.handler_pc)
                    .copied()
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                let caught = Operand::just(Identifier::CaughtException(caught_id));
                let handler_frame = frame.same_locals_1_stack_item_frame(Entry::Value(caught));
                Ok((
                    entry.handler_pc,
                    ControlTransfer::Exception(BTreeSet::from([caught_type])),
                    handler_frame,
                ))
            })
            .collect()
    }

    fn analyze_frame_and_conditions(
        &self,
        location: ProgramCounter,
        mut frame: JvmStackFrame,
        instruction: &LiftedInstruction,
    ) -> Result<Vec<(ProgramCounter, ControlTransfer, JvmStackFrame)>, MokaIRBrewingError> {
        use ControlTransfer::{Conditional, SubroutineReturn, Unconditional};

        Ok(match instruction {
            LiftedInstruction::Nop => vec![(self.next_pc_of(location)?, Unconditional, frame)],
            LiftedInstruction::Return(_) => Vec::new(),
            LiftedInstruction::Throw(_) => self.exception_edges(location, &frame)?,
            LiftedInstruction::Subroutine {
                target,
                return_address,
                ..
            } => {
                frame.possible_ret_addresses.insert(*return_address);
                vec![(*target, Unconditional, frame)]
            }
            LiftedInstruction::Definition { .. } => once((
                self.next_pc_of(location)?,
                Unconditional,
                frame.same_frame(),
            ))
            .chain(self.exception_edges(location, &frame)?)
            .collect(),
            LiftedInstruction::Jump {
                condition: None,
                target,
            } => vec![(*target, Unconditional, frame.same_frame())],
            LiftedInstruction::Jump {
                condition: Some(condition),
                target,
            } => {
                let condition: BooleanVariable<_> = condition.clone().into();
                vec![
                    (
                        *target,
                        Conditional(BranchGuard::of(condition.clone())),
                        frame.same_frame(),
                    ),
                    (
                        self.next_pc_of(location)?,
                        Conditional(BranchGuard::of(!condition)),
                        frame.same_frame(),
                    ),
                ]
            }
            LiftedInstruction::Switch {
                default,
                branches,
                match_value,
            } => {
                let branch_edges = branches.iter().map(|(&case, &target)| {
                    let value = Value::Constant(ConstantValue::Integer(case));
                    let condition = BooleanVariable::Positive(Condition::Equal(
                        match_value.clone().into(),
                        value,
                    ));
                    (
                        target,
                        Conditional(BranchGuard::of(condition)),
                        frame.same_frame(),
                    )
                });
                let default_guard = branches
                    .keys()
                    .map(|case| {
                        BooleanVariable::Negative(Condition::Equal(
                            match_value.clone().into(),
                            ConstantValue::Integer(*case).into(),
                        ))
                    })
                    .collect();
                branch_edges
                    .chain(once((
                        *default,
                        Conditional(default_guard),
                        frame.same_frame(),
                    )))
                    .collect()
            }
            LiftedInstruction::SubroutineReturn(_) => mem::take(&mut frame.possible_ret_addresses)
                .into_iter()
                .map(|target| (target, SubroutineReturn, frame.same_frame()))
                .collect(),
        })
    }

    fn generate(mut self) -> Result<(BlockId, Vec<BasicBlock>, SourceMap), MokaIRBrewingError> {
        use crate::analysis::fixed_point::solve;

        let _: HashMap<_, _> = solve(&mut self)?;
        let reachable = self.lifted.keys().copied().collect::<Vec<_>>();
        self.value_ids = reachable
            .iter()
            .enumerate()
            .map(|(index, pc)| {
                u32::try_from(index)
                    .map(|index| (*pc, ValueId::new(index)))
                    .map_err(|_| MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<_, _>>()?;
        let first_caught_id = u32::try_from(self.value_ids.len())
            .map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
        self.caught_exception_ids = self
            .caught_exception_ids
            .keys()
            .filter(|handler| self.lifted.contains_key(handler))
            .copied()
            .enumerate()
            .map(|(offset, handler)| {
                let offset =
                    u32::try_from(offset).map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
                first_caught_id
                    .checked_add(offset)
                    .map(|id| (handler, ValueId::new(id)))
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<_, _>>()?;
        self.lifted.clear();
        self.outgoing.clear();
        let _: HashMap<_, _> = solve(&mut self)?;
        self.assemble_blocks()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "block partitioning and identity allocation form one invariant-preserving pass"
    )]
    fn assemble_blocks(self) -> Result<(BlockId, Vec<BasicBlock>, SourceMap), MokaIRBrewingError> {
        let entry_pc = self
            .initial_seed
            .as_ref()
            .map(|(pc, _)| *pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let reachable = self.lifted.keys().copied().collect::<Vec<_>>();
        if reachable.is_empty() {
            return Err(MokaIRBrewingError::MalformedControlFlow);
        }

        let mut leaders = BTreeSet::from([entry_pc]);
        leaders.extend(
            self.body
                .exception_table
                .iter()
                .map(|entry| entry.handler_pc)
                .filter(|handler| self.lifted.contains_key(handler)),
        );
        let mut predecessors: BTreeMap<ProgramCounter, BTreeSet<ProgramCounter>> = BTreeMap::new();
        for (&source, arms) in &self.outgoing {
            for (target, transfer) in arms {
                predecessors.entry(*target).or_default().insert(source);
                if matches!(transfer, ControlTransfer::Exception(_)) {
                    leaders.insert(*target);
                }
            }
        }
        leaders.extend(
            predecessors
                .iter()
                .filter(|(_, sources)| sources.len() > 1)
                .map(|(target, _)| *target),
        );

        for (pc, instruction) in &self.lifted {
            let arms = self
                .outgoing
                .get(pc)
                .map_or(&[] as &[(ProgramCounter, ControlTransfer)], Vec::as_slice);
            if instruction.is_explicit_transfer()
                || arms
                    .iter()
                    .any(|(_, transfer)| matches!(transfer, ControlTransfer::Exception(_)))
            {
                leaders.extend(arms.iter().map(|(target, _)| *target));
            }
            if let LiftedInstruction::Subroutine { return_address, .. } = instruction
                && self.lifted.contains_key(return_address)
            {
                leaders.insert(*return_address);
            }
        }

        for pair in reachable.windows(2) {
            let [current, next] = pair else {
                unreachable!()
            };
            let instruction = self
                .lifted
                .get(current)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            let arms = self
                .outgoing
                .get(current)
                .map_or(&[] as &[(ProgramCounter, ControlTransfer)], Vec::as_slice);
            let plain_fallthrough = !instruction.is_explicit_transfer()
                && arms.len() == 1
                && arms[0].0 == *next
                && matches!(arms[0].1, ControlTransfer::Unconditional);
            if !plain_fallthrough {
                leaders.insert(*next);
            }
        }
        leaders.retain(|pc| self.lifted.contains_key(pc));

        let block_ids = leaders
            .iter()
            .enumerate()
            .map(|(index, pc)| {
                u32::try_from(index)
                    .map(|index| (*pc, BlockId::new(index)))
                    .map_err(|_| MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let entry = *block_ids
            .get(&entry_pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;

        let mut pc_to_block = BTreeMap::new();
        let mut grouped: BTreeMap<BlockId, Vec<ProgramCounter>> = BTreeMap::new();
        let mut current_block = None;
        for pc in reachable {
            if let Some(id) = block_ids.get(&pc) {
                current_block = Some(*id);
            }
            let id = current_block.ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            pc_to_block.insert(pc, id);
            grouped.entry(id).or_default().push(pc);
        }

        let mut next_instruction = 0_u32;
        let mut next_edge = 0_u32;
        let mut source_map = SourceMap::default();
        let mut blocks = Vec::with_capacity(grouped.len());
        for (block_id, pcs) in grouped {
            let last_pc = *pcs.last().ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            let last = self
                .lifted
                .get(&last_pc)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            let mut instructions = Vec::new();
            for pc in &pcs {
                let lifted = self
                    .lifted
                    .get(pc)
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                let kind = match lifted {
                    LiftedInstruction::Nop => Some(InstructionKind::Nop),
                    LiftedInstruction::Definition { value, expr } => {
                        Some(InstructionKind::Definition {
                            value: *value,
                            expr: expr.clone(),
                        })
                    }
                    LiftedInstruction::Subroutine { value, .. } => {
                        Some(InstructionKind::Definition {
                            value: *value,
                            expr: Expression::SubroutineReturnAddress,
                        })
                    }
                    LiftedInstruction::Jump { .. }
                    | LiftedInstruction::Switch { .. }
                    | LiftedInstruction::Return(_)
                    | LiftedInstruction::Throw(_)
                    | LiftedInstruction::SubroutineReturn(_) => None,
                };
                if let Some(kind) = kind {
                    let id = InstructionId::new(next_instruction);
                    next_instruction = next_instruction
                        .checked_add(1)
                        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                    instructions.push(MokaInstruction::new(id, kind));
                    source_map.insert(*pc, id);
                }
            }

            let arms = self
                .outgoing
                .get(&last_pc)
                .map_or(&[] as &[(ProgramCounter, ControlTransfer)], Vec::as_slice);
            let mut successors = Vec::with_capacity(arms.len());
            for (target, transfer) in arms {
                let target = pc_to_block
                    .get(target)
                    .copied()
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                let edge_id = EdgeId::new(next_edge);
                next_edge = next_edge
                    .checked_add(1)
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                successors.push(Successor::new(edge_id, target, transfer.clone()));
            }

            let has_exception = arms
                .iter()
                .any(|(_, transfer)| matches!(transfer, ControlTransfer::Exception(_)));
            let (terminator_kind, source_backed) = match last {
                LiftedInstruction::Jump {
                    condition: Some(_), ..
                } => (TerminatorKind::Branch, true),
                LiftedInstruction::Jump {
                    condition: None, ..
                }
                | LiftedInstruction::Subroutine { .. } => (TerminatorKind::Goto, true),
                LiftedInstruction::Switch { match_value, .. } => (
                    TerminatorKind::Switch {
                        match_value: match_value.clone(),
                    },
                    true,
                ),
                LiftedInstruction::Return(value) => (TerminatorKind::Return(value.clone()), true),
                LiftedInstruction::Throw(value) => (TerminatorKind::Throw(value.clone()), true),
                LiftedInstruction::SubroutineReturn(value) => {
                    (TerminatorKind::SubroutineReturn(value.clone()), true)
                }
                LiftedInstruction::Definition { .. } if has_exception => {
                    (TerminatorKind::Fallible, false)
                }
                LiftedInstruction::Nop | LiftedInstruction::Definition { .. } => {
                    (TerminatorKind::Goto, false)
                }
            };
            let terminator_id = InstructionId::new(next_instruction);
            next_instruction = next_instruction
                .checked_add(1)
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            if source_backed {
                source_map.insert(last_pc, terminator_id);
            }
            blocks.push(BasicBlock::new(
                block_id,
                instructions,
                Terminator::new(terminator_id, terminator_kind, successors),
            ));
        }

        Ok((entry, blocks, source_map))
    }
}

/// An extension trait for [`Method`] that generates Moka IR.
pub trait MokaIRMethodExt {
    /// Generates Moka IR for the method.
    ///
    /// # Errors
    /// See [`MokaIRBrewingError`] for more information.
    fn brew(&self) -> Result<MokaIRMethod, MokaIRBrewingError>;
}

impl MokaIRMethodExt for Method {
    fn brew(&self) -> Result<MokaIRMethod, MokaIRBrewingError> {
        let (entry, blocks, source_map) = MokaIRGenerator::for_method(self)?.generate()?;
        Ok(MokaIRMethod::new(
            self.access_flags,
            self.name.clone(),
            self.descriptor.clone(),
            self.owner.clone(),
            entry,
            blocks,
            source_map,
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use super::*;
    use crate::jvm::code::{ExceptionTableEntry, Instruction, InstructionList};

    fn method(
        instructions: impl IntoIterator<Item = (ProgramCounter, Instruction)>,
        descriptor: &str,
        exception_table: Vec<ExceptionTableEntry>,
    ) -> Method {
        Method {
            access_flags: method::AccessFlags::PUBLIC | method::AccessFlags::STATIC,
            name: "test".to_owned(),
            descriptor: descriptor.parse().unwrap(),
            owner: "org/mokapot/Test".parse().unwrap(),
            body: Some(MethodBody {
                max_stack: 4,
                max_locals: 4,
                instructions: InstructionList::from_iter(instructions),
                exception_table,
                line_number_table: None,
                local_variable_table: None,
                stack_map_table: None,
                runtime_visible_type_annotations: vec![],
                runtime_invisible_type_annotations: vec![],
                other_attributes: vec![],
            }),
            exceptions: vec![],
            runtime_visible_annotations: vec![],
            runtime_invisible_annotations: vec![],
            runtime_visible_type_annotations: vec![],
            runtime_invisible_type_annotations: vec![],
            runtime_visible_parameter_annotations: vec![],
            runtime_invisible_parameter_annotations: vec![],
            annotation_default: None,
            parameters: vec![],
            is_synthetic: false,
            is_deprecated: false,
            signature: None,
            other_attributes: vec![],
        }
    }

    #[test]
    fn straight_line_instructions_coalesce_into_one_block() {
        let method = method(
            [
                (0.into(), Instruction::IConst0),
                (10.into(), Instruction::IStore0),
                (20.into(), Instruction::ILoad0),
                (30.into(), Instruction::IReturn),
            ],
            "()I",
            vec![],
        );
        let ir = method.brew().unwrap();
        let blocks = ir.blocks().collect::<Vec<_>>();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].instructions().len(), 3);
        assert!(matches!(
            blocks[0].terminator().kind(),
            TerminatorKind::Return(Some(_))
        ));
        let ids = blocks[0]
            .instructions()
            .iter()
            .map(MokaInstruction::id)
            .chain(once(blocks[0].terminator().id()))
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn value_identities_do_not_depend_on_sparse_program_counters() {
        let compact = method(
            [
                (0.into(), Instruction::IConst0),
                (1.into(), Instruction::Pop),
                (2.into(), Instruction::IConst1),
                (3.into(), Instruction::IReturn),
            ],
            "()I",
            vec![],
        )
        .brew()
        .unwrap();
        let sparse = method(
            [
                (0.into(), Instruction::IConst0),
                (100.into(), Instruction::Pop),
                (1000.into(), Instruction::IConst1),
                (5000.into(), Instruction::IReturn),
            ],
            "()I",
            vec![],
        )
        .brew()
        .unwrap();
        let values = |method: &MokaIRMethod| {
            method
                .blocks()
                .flat_map(BasicBlock::instructions)
                .filter_map(MokaInstruction::def)
                .collect::<Vec<_>>()
        };

        assert_eq!(values(&compact), values(&sparse));
    }

    #[test]
    fn unreachable_bytecode_is_omitted() {
        let method = method(
            [
                (0.into(), Instruction::Goto(100.into())),
                (10.into(), Instruction::IConst0),
                (11.into(), Instruction::IReturn),
                (100.into(), Instruction::Return),
            ],
            "()V",
            vec![],
        );
        let ir = method.brew().unwrap();

        assert_eq!(ir.blocks().len(), 2);
        assert_eq!(ir.source_map().instructions_at(10.into()).count(), 0);
        assert_eq!(ir.source_map().instructions_at(11.into()).count(), 0);
    }

    #[test]
    fn backward_target_starts_a_block_even_when_transfer_is_last() {
        let method = method(
            [
                (0.into(), Instruction::Nop),
                (1.into(), Instruction::Nop),
                (2.into(), Instruction::Goto(1.into())),
            ],
            "()V",
            vec![],
        );
        let ir = method.brew().unwrap();

        assert_eq!(ir.blocks().len(), 2);
        let loop_block = ir.blocks().nth(1).unwrap();
        assert_eq!(
            loop_block.terminator().successors()[0].target(),
            loop_block.id()
        );
    }

    #[test]
    fn diamond_has_an_unmapped_synthetic_fallthrough() {
        let method = method(
            [
                (0.into(), Instruction::ILoad0),
                (1.into(), Instruction::IfEq(3.into())),
                (2.into(), Instruction::Nop),
                (3.into(), Instruction::Return),
            ],
            "(I)V",
            vec![],
        );
        let ir = method.brew().unwrap();
        let fallthrough_instruction = ir.source_map().instructions_at(2.into()).next().unwrap();
        let synthetic = ir
            .blocks()
            .find(|block| {
                block
                    .instructions()
                    .iter()
                    .any(|instruction| instruction.id() == fallthrough_instruction)
            })
            .unwrap()
            .terminator();

        assert_eq!(synthetic.kind(), &TerminatorKind::Goto);
        assert_eq!(ir.source_map().origins_of(synthetic.id()).count(), 0);
    }

    #[test]
    fn switch_retains_parallel_successor_arms() {
        let method = method(
            [
                (0.into(), Instruction::ILoad0),
                (
                    1.into(),
                    Instruction::LookupSwitch {
                        default: 10.into(),
                        match_targets: BTreeMap::from([(1, 10.into()), (2, 10.into())]),
                    },
                ),
                (10.into(), Instruction::Return),
            ],
            "(I)V",
            vec![],
        );
        let ir = method.brew().unwrap();
        let switch = ir.block(ir.entry_block()).unwrap().terminator();

        assert!(matches!(switch.kind(), TerminatorKind::Switch { .. }));
        assert_eq!(switch.successors().len(), 3);
        assert_eq!(
            switch
                .successors()
                .iter()
                .map(Successor::id)
                .collect::<HashSet<_>>()
                .len(),
            3
        );
        assert!(
            switch
                .successors()
                .windows(2)
                .all(|pair| pair[0].target() == pair[1].target())
        );
        assert_eq!(ir.control_flow_graph().edges().count(), 3);
    }

    #[test]
    fn fallible_exit_keeps_normal_then_ordered_handler_arms() {
        let exception_table = vec![
            ExceptionTableEntry {
                covered_pc: 0.into()..1.into(),
                handler_pc: 3.into(),
                catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
            },
            ExceptionTableEntry {
                covered_pc: 0.into()..1.into(),
                handler_pc: 2.into(),
                catch_type: Some("java/lang/Exception".parse().unwrap()),
            },
        ];
        let method = method(
            [
                (0.into(), Instruction::IConst0),
                (1.into(), Instruction::Return),
                (2.into(), Instruction::Return),
                (3.into(), Instruction::Return),
            ],
            "()V",
            exception_table,
        );
        let ir = method.brew().unwrap();
        let fallible = ir.block(ir.entry_block()).unwrap().terminator();

        assert_eq!(fallible.kind(), &TerminatorKind::Fallible);
        assert_eq!(ir.source_map().origins_of(fallible.id()).count(), 0);
        assert!(matches!(
            fallible.successors()[0].transfer(),
            ControlTransfer::Unconditional
        ));
        let handler_types = fallible.successors()[1..]
            .iter()
            .map(|successor| match successor.transfer() {
                ControlTransfer::Exception(types) => types.iter().next().unwrap().0.as_ref(),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            handler_types,
            ["java/lang/RuntimeException", "java/lang/Exception"]
        );
    }

    #[test]
    fn normally_reachable_handler_still_starts_a_block() {
        let method = method(
            [
                (0.into(), Instruction::ALoad0),
                (1.into(), Instruction::AStore1),
                (2.into(), Instruction::Return),
            ],
            "(Ljava/lang/Throwable;)V",
            vec![ExceptionTableEntry {
                covered_pc: 0.into()..1.into(),
                handler_pc: 1.into(),
                catch_type: Some("java/lang/Throwable".parse().unwrap()),
            }],
        );
        let ir = method.brew().unwrap();
        let entry = ir.block(ir.entry_block()).unwrap();
        let handler = entry.terminator().successors()[0].target();

        assert_eq!(ir.blocks().len(), 2);
        assert_ne!(handler, ir.entry_block());
        assert_eq!(ir.block(handler).unwrap().instructions().len(), 1);
    }

    #[test]
    fn throw_is_a_source_backed_terminator() {
        let method = method(
            [
                (0.into(), Instruction::ALoad0),
                (1.into(), Instruction::AThrow),
            ],
            "(Ljava/lang/Throwable;)V",
            vec![],
        );
        let ir = method.brew().unwrap();
        let block = ir.block(ir.entry_block()).unwrap();

        assert!(matches!(
            block.terminator().kind(),
            TerminatorKind::Throw(_)
        ));
        assert_eq!(
            ir.source_map()
                .origins_of(block.terminator().id())
                .collect::<Vec<_>>(),
            [ProgramCounter::from(1)]
        );
    }
}
