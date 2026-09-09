use super::{
    BTreeMap, BTreeSet, BooleanVariable, BranchGuard, ConstantValue, DataflowProblem, Entry,
    GeneratedMethod, HashMap, Identifier, JvmStackFrame, LiftedCondition, LiftedControlTransfer,
    LiftedInstruction, LiftedValue, Method, MokaIRBrewingError, MokaIRGenerator, Operand,
    OutgoingState, ProgramCounter, ValueId, fmt, mem, method, once,
};

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
            self.analyze_frame_and_conditions(location, frame, &ir_instruction, &|id| {
                Operand::just(Identifier::CaughtException(id))
            })?;
        self.lifted.insert(location, ir_instruction);
        self.outgoing.insert(
            location,
            edges_and_frames
                .iter()
                .map(|(target, transfer, _)| (*target, transfer.clone()))
                .collect(),
        );
        self.outgoing_frames.insert(
            location,
            edges_and_frames
                .iter()
                .map(|(_, _, frame)| frame.clone())
                .collect(),
        );
        Ok(edges_and_frames
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
            outgoing_frames: BTreeMap::new(),
            value_ids,
            caught_exception_ids,
            method,
            body,
            initial_seed: Some((first_pc, initial_frame)),
        })
    }

    pub(super) fn value_at(&self, pc: ProgramCounter) -> Result<ValueId, MokaIRBrewingError> {
        self.value_ids
            .get(&pc)
            .copied()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)
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

    fn exception_edges<OP: Clone + Eq + std::hash::Hash>(
        &self,
        pc: ProgramCounter,
        frame: &JvmStackFrame<OP>,
        caught_value: &impl Fn(ValueId) -> OP,
    ) -> Result<Vec<OutgoingState<OP>>, MokaIRBrewingError> {
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
                let caught = caught_value(caught_id);
                let handler_frame = frame.same_locals_1_stack_item_frame(Entry::Value(caught));
                Ok((
                    entry.handler_pc,
                    LiftedControlTransfer::Exception(BTreeSet::from([caught_type])),
                    handler_frame,
                ))
            })
            .collect()
    }

    pub(super) fn analyze_frame_and_conditions<OP>(
        &self,
        location: ProgramCounter,
        mut frame: JvmStackFrame<OP>,
        instruction: &LiftedInstruction<OP>,
        caught_value: &impl Fn(ValueId) -> OP,
    ) -> Result<Vec<OutgoingState<OP>>, MokaIRBrewingError>
    where
        OP: Clone + Eq + std::hash::Hash + fmt::Display,
    {
        use LiftedControlTransfer::{Conditional, SubroutineReturn, Unconditional};

        Ok(match instruction {
            LiftedInstruction::Nop => vec![(self.next_pc_of(location)?, Unconditional, frame)],
            LiftedInstruction::Return(_) => Vec::new(),
            LiftedInstruction::Throw(_) => self.exception_edges(location, &frame, caught_value)?,
            LiftedInstruction::Subroutine {
                target,
                return_address,
                ..
            } => {
                frame.possible_ret_addresses.insert(*return_address);
                vec![(*target, Unconditional, frame)]
            }
            LiftedInstruction::Definition { .. } | LiftedInstruction::Effect(_) => once((
                self.next_pc_of(location)?,
                Unconditional,
                frame.same_frame(),
            ))
            .chain(self.exception_edges(location, &frame, caught_value)?)
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
                    let value: LiftedValue<OP> =
                        LiftedValue::Constant(ConstantValue::Integer(case));
                    let condition = BooleanVariable::Positive(LiftedCondition::Equal(
                        match_value.clone().into(),
                        value,
                    ));
                    (
                        target,
                        Conditional(BranchGuard::of(condition)),
                        frame.same_frame(),
                    )
                });
                let default_guard: BranchGuard<LiftedCondition<LiftedValue<OP>>> = branches
                    .keys()
                    .map(|case| {
                        BooleanVariable::Negative(LiftedCondition::Equal(
                            match_value.clone().into(),
                            LiftedValue::Constant(ConstantValue::Integer(*case)),
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

    pub(super) fn generate(mut self) -> Result<GeneratedMethod, MokaIRBrewingError> {
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
        self.outgoing_frames.clear();
        let facts: HashMap<_, _> = solve(&mut self)?;
        self.assemble_blocks(&facts)
    }
}
