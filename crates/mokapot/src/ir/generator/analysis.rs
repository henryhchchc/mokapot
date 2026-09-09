use super::{
    BTreeMap, BooleanVariable, BranchGuard, ConstantValue, DataflowProblem, Entry, FrameOperand,
    GeneratedMethod, HashMap, Identifier, JvmStackFrame, LiftedCondition, LiftedControlTransfer,
    LiftedInstruction, LiftedValue, Location, Method, MokaIRBrewingError, MokaIRGenerator, Operand,
    OutgoingState, ProgramCounter, ValueId, fallibility, method,
};

impl DataflowProblem for MokaIRGenerator<'_> {
    type Location = Location;
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
        let (instruction, outgoing) = match location {
            Location::Handler {
                handler_pc,
                context,
            } => {
                let target = self.legacy.bytecode(handler_pc, context)?;
                (
                    LiftedInstruction::HandlerEntry,
                    vec![(
                        target,
                        LiftedControlTransfer::Unconditional,
                        fact.same_frame(),
                    )],
                )
            }
            Location::Unwind => (LiftedInstruction::Unwind, Vec::new()),
            Location::Bytecode { pc, .. } => {
                let pre_frame = fact.same_frame();
                let mut normal_frame = fact.same_frame();
                let jvm_instruction = self
                    .body
                    .instruction_at(pc)
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?
                    .clone();
                let instruction =
                    self.lift_instruction(&jvm_instruction, location, &mut normal_frame)?;
                let outgoing = self.analyze_frame_and_conditions(
                    location,
                    &pre_frame,
                    normal_frame,
                    &instruction,
                    fallibility::is_synchronously_fallible(&jvm_instruction),
                    &|id| Operand::just(Identifier::CaughtException(id)),
                )?;
                (instruction, outgoing)
            }
        };

        self.lifted.insert(location, instruction);
        self.outgoing.insert(
            location,
            outgoing
                .iter()
                .map(|(target, transfer, _)| (*target, transfer.clone()))
                .collect(),
        );
        self.outgoing_frames.insert(
            location,
            outgoing.iter().map(|(_, _, frame)| frame.clone()).collect(),
        );
        Ok(outgoing
            .into_iter()
            .map(|(target, _, frame)| (target, frame))
            .collect::<Vec<_>>())
    }
}

impl<'method> MokaIRGenerator<'method> {
    pub(super) fn for_method(method: &'method Method) -> Result<Self, MokaIRBrewingError> {
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
        let entry = Location::entry(first_pc);

        Ok(Self {
            lifted: BTreeMap::default(),
            outgoing: BTreeMap::default(),
            outgoing_frames: BTreeMap::default(),
            value_ids: BTreeMap::default(),
            caught_exception_ids: BTreeMap::default(),
            method,
            body,
            legacy: super::LegacyNormalizer::new(first_pc),
            discovering: true,
            next_lifted_value: 0,
            initial_seed: Some((entry, initial_frame)),
        })
    }

    pub(super) fn value_at(&mut self, location: Location) -> Result<ValueId, MokaIRBrewingError> {
        if let Some(value) = self.value_ids.get(&location) {
            return Ok(*value);
        }
        if !self.discovering || !matches!(location, Location::Bytecode { .. }) {
            return Err(MokaIRBrewingError::MalformedControlFlow);
        }
        let value = self.next_temporary_value()?;
        self.value_ids.insert(location, value);
        Ok(value)
    }

    fn caught_exception_at(&mut self, location: Location) -> Result<ValueId, MokaIRBrewingError> {
        if let Some(value) = self.caught_exception_ids.get(&location) {
            return Ok(*value);
        }
        if !self.discovering || !matches!(location, Location::Handler { .. }) {
            return Err(MokaIRBrewingError::MalformedControlFlow);
        }
        let value = self.next_temporary_value()?;
        self.caught_exception_ids.insert(location, value);
        Ok(value)
    }

    fn next_temporary_value(&mut self) -> Result<ValueId, MokaIRBrewingError> {
        let value = ValueId::new(self.next_lifted_value);
        self.next_lifted_value = self
            .next_lifted_value
            .checked_add(1)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        Ok(value)
    }

    pub(super) fn next_pc_of(
        &self,
        pc: ProgramCounter,
    ) -> Result<ProgramCounter, MokaIRBrewingError> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)
    }

    fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBrewingError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        self.legacy.bytecode(self.next_pc_of(pc)?, context)
    }

    fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBrewingError> {
        let context = location
            .context()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        self.legacy.bytecode(target, context)
    }

    fn exception_edges<OP: FrameOperand>(
        &mut self,
        location: Location,
        pre_frame: &JvmStackFrame<OP>,
        caught_value: &impl Fn(ValueId) -> OP,
    ) -> Result<Vec<OutgoingState<OP>>, MokaIRBrewingError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let entries = self
            .body
            .exception_table
            .iter()
            .filter(|entry| entry.covers(pc))
            .cloned()
            .collect::<Vec<_>>();
        let mut outgoing = Vec::with_capacity(entries.len() + 1);
        let mut exhaustive = false;
        for entry in entries {
            let handler = self.legacy.handler(entry.handler_pc, context)?;
            let caught = caught_value(self.caught_exception_at(handler)?);
            outgoing.push((
                handler,
                LiftedControlTransfer::Exception(entry.catch_type.clone()),
                pre_frame.same_locals_1_stack_item_frame(Entry::Value(caught)),
            ));
            exhaustive = entry
                .catch_type
                .as_ref()
                .is_none_or(|caught_type| caught_type.0.as_ref() == "java/lang/Throwable");
            if exhaustive {
                break;
            }
        }
        if !exhaustive {
            let unwind = self.legacy.register(Location::Unwind)?;
            outgoing.push((
                unwind,
                LiftedControlTransfer::Unwind,
                pre_frame.same_locals_empty_stack_frame(),
            ));
        }
        Ok(outgoing)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "all control-flow forms are classified together"
    )]
    pub(super) fn analyze_frame_and_conditions<OP: FrameOperand>(
        &mut self,
        location: Location,
        pre_frame: &JvmStackFrame<OP>,
        normal_frame: JvmStackFrame<OP>,
        instruction: &LiftedInstruction<OP>,
        fallible: bool,
        caught_value: &impl Fn(ValueId) -> OP,
    ) -> Result<Vec<OutgoingState<OP>>, MokaIRBrewingError> {
        use LiftedControlTransfer::{Conditional, Normal, Unconditional};

        Ok(match instruction {
            LiftedInstruction::HandlerEntry => {
                let Location::Handler {
                    handler_pc,
                    context,
                } = location
                else {
                    return Err(MokaIRBrewingError::MalformedControlFlow);
                };
                vec![(
                    self.legacy.bytecode(handler_pc, context)?,
                    Unconditional,
                    normal_frame,
                )]
            }
            LiftedInstruction::Unwind | LiftedInstruction::Return(_) => Vec::new(),
            LiftedInstruction::Throw(_) => {
                self.exception_edges(location, pre_frame, caught_value)?
            }
            LiftedInstruction::Subroutine { target, .. } => {
                vec![(*target, Unconditional, normal_frame)]
            }
            LiftedInstruction::Definition { .. } | LiftedInstruction::Effect(_) if fallible => {
                let mut outgoing = vec![(self.next_location(location)?, Normal, normal_frame)];
                outgoing.extend(self.exception_edges(location, pre_frame, caught_value)?);
                outgoing
            }
            LiftedInstruction::Nop
            | LiftedInstruction::Definition { .. }
            | LiftedInstruction::Effect(_) => {
                vec![(self.next_location(location)?, Unconditional, normal_frame)]
            }
            LiftedInstruction::Jump {
                condition: None,
                target,
            } => vec![(
                self.target_location(location, *target)?,
                Unconditional,
                normal_frame,
            )],
            LiftedInstruction::Jump {
                condition: Some(condition),
                target,
            } => {
                let condition: BooleanVariable<_> = condition.clone().into();
                vec![
                    (
                        self.target_location(location, *target)?,
                        Conditional(BranchGuard::of(condition.clone())),
                        normal_frame.same_frame(),
                    ),
                    (
                        self.next_location(location)?,
                        Conditional(BranchGuard::of(!condition)),
                        normal_frame,
                    ),
                ]
            }
            LiftedInstruction::Switch {
                default,
                branches,
                match_value,
            } => {
                let mut outgoing = Vec::with_capacity(branches.len() + 1);
                for (&case, &target) in branches {
                    let value = LiftedValue::Constant(ConstantValue::Integer(case));
                    let condition = BooleanVariable::Positive(LiftedCondition::Equal(
                        match_value.clone().into(),
                        value,
                    ));
                    outgoing.push((
                        self.target_location(location, target)?,
                        Conditional(BranchGuard::of(condition)),
                        normal_frame.same_frame(),
                    ));
                }
                let default_guard = branches
                    .keys()
                    .map(|case| {
                        BooleanVariable::Negative(LiftedCondition::Equal(
                            match_value.clone().into(),
                            LiftedValue::Constant(ConstantValue::Integer(*case)),
                        ))
                    })
                    .collect();
                outgoing.push((
                    self.target_location(location, *default)?,
                    Conditional(default_guard),
                    normal_frame,
                ));
                outgoing
            }
            LiftedInstruction::SubroutineReturn(value) => {
                let address = value
                    .return_address()
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
                vec![(
                    self.legacy.return_from(location, address)?,
                    Unconditional,
                    normal_frame,
                )]
            }
        })
    }

    pub(super) fn generate(mut self) -> Result<GeneratedMethod, MokaIRBrewingError> {
        use crate::analysis::fixed_point::solve;

        let _: HashMap<_, _> = solve(&mut self)?;
        let reachable = self.lifted.keys().copied().collect::<Vec<_>>();
        self.value_ids = reachable
            .iter()
            .filter(|location| matches!(location, Location::Bytecode { .. }))
            .enumerate()
            .map(|(index, &location)| {
                u32::try_from(index)
                    .map(|index| (location, ValueId::new(index)))
                    .map_err(|_| MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<_, _>>()?;
        let first_caught_id = u32::try_from(self.value_ids.len())
            .map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
        self.caught_exception_ids = reachable
            .iter()
            .filter(|location| matches!(location, Location::Handler { .. }))
            .enumerate()
            .map(|(offset, &handler)| {
                let offset =
                    u32::try_from(offset).map_err(|_| MokaIRBrewingError::MalformedControlFlow)?;
                first_caught_id
                    .checked_add(offset)
                    .map(|id| (handler, ValueId::new(id)))
                    .ok_or(MokaIRBrewingError::MalformedControlFlow)
            })
            .collect::<Result<_, _>>()?;
        self.next_lifted_value = first_caught_id
            .checked_add(
                u32::try_from(self.caught_exception_ids.len())
                    .map_err(|_| MokaIRBrewingError::MalformedControlFlow)?,
            )
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        self.discovering = false;
        self.lifted.clear();
        self.outgoing.clear();
        self.outgoing_frames.clear();
        let facts: HashMap<_, _> = solve(&mut self)?;
        self.assemble_blocks(&facts)
    }
}
