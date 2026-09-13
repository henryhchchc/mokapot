use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            frame::JvmStackFrame,
            instruction::RegisterInstruction,
            lifting::{
                fallibility::FallibilityContext, lift_instruction, semantics::outgoing_from,
            },
            normalization::{Location, Normalizer},
            symbolic_execution::fact::{AnalyzedJvmCfg, AnalyzedLocation, OperandState},
            symbolic_execution::solver,
        },
    },
    jvm::{
        Method,
        code::{MethodBody, ProgramCounter},
        method,
    },
};

/// Mutable state used only while performing symbolic execution.
pub(crate) struct JvmSymbolicExecutor<'method> {
    pub(super) body: &'method MethodBody,
    fallibility: FallibilityContext,
    pub(super) normalizer: Normalizer,
    pub(super) definition_ids: BTreeMap<Location, SsaValueId>,
    pub(super) caught_exception_ids: BTreeMap<Location, SsaValueId>,
    pub(super) value_id_allocator: ValueIdAllocator,
    this_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    entry_location: Location,
    initial_frame: JvmStackFrame<OperandState>,
}

impl<'method> JvmSymbolicExecutor<'method> {
    pub fn transfer(
        &mut self,
        location: Location,
        incoming: JvmStackFrame<OperandState>,
    ) -> Result<AnalyzedLocation, MokaIRBuildError> {
        let (instruction, outgoing) = match location {
            Location::Handler { .. } => {
                let instruction = RegisterInstruction::HandlerEntry;
                let normal_frame = incoming.same_frame();
                let outgoing =
                    outgoing_from(self, location, &incoming, normal_frame, &instruction, false)?;
                (instruction, outgoing)
            }
            Location::Unwind => (RegisterInstruction::Unwind, Vec::new()),
            Location::Bytecode { pc, .. } => {
                let mut normal_frame = incoming.same_frame();
                let jvm_instruction = self
                    .body
                    .instruction_at(pc)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?
                    .clone();
                let fallible = self.fallibility.is_synchronously_fallible(&jvm_instruction);
                let instruction =
                    lift_instruction(self, &jvm_instruction, location, &mut normal_frame)?;
                let outgoing = outgoing_from(
                    self,
                    location,
                    &incoming,
                    normal_frame,
                    &instruction,
                    fallible,
                )?;
                (instruction, outgoing)
            }
        };

        Ok(AnalyzedLocation {
            incoming,
            instruction,
            outgoing,
            caught_exception: self.caught_exception_ids.get(&location).copied(),
        })
    }

    pub const fn body(&self) -> &MethodBody {
        self.body
    }

    pub fn for_method(method: &'method Method) -> Result<Self, MokaIRBuildError> {
        let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
        let (first_pc, _) = body
            .instructions
            .entry_point()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let mut value_id_allocator = ValueIdAllocator::default();
        let this_value = (!method.access_flags.contains(method::AccessFlags::STATIC))
            .then(|| value_id_allocator.new_value_id())
            .transpose()?;
        let parameter_values: Vec<SsaValueId> = method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| value_id_allocator.new_value_id())
            .collect::<Result<_, _>>()?;
        let frame_parameters = parameter_values
            .iter()
            .copied()
            .map(OperandState::Value)
            .collect::<Vec<_>>();
        let initial_frame = JvmStackFrame::with_inputs(
            &method.descriptor,
            body.max_locals,
            body.max_stack,
            this_value.map(OperandState::Value),
            &frame_parameters,
        )?;
        let entry_location = Location::entry(first_pc);
        let analyzer = Self {
            body,
            fallibility: FallibilityContext::for_method(method),
            normalizer: Normalizer::new(first_pc),
            definition_ids: BTreeMap::new(),
            caught_exception_ids: BTreeMap::new(),
            value_id_allocator,
            this_value,
            parameter_values,
            entry_location,
            initial_frame,
        };
        Ok(analyzer)
    }

    pub fn analyze(mut self) -> Result<AnalyzedJvmCfg, MokaIRBuildError> {
        let locations = self.solve_locations()?;
        let merge_identities = locations
            .values()
            .flat_map(|location| location.incoming.values())
            .filter_map(|value| match value {
                OperandState::Merged(identity) => Some(*identity),
                OperandState::Value(_) | OperandState::ReturnAddress(_) | OperandState::Invalid => {
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        let phi_values = merge_identities
            .into_iter()
            .map(|identity| self.new_value_id().map(|value| (identity, value)))
            .collect::<Result<_, _>>()?;
        Ok(AnalyzedJvmCfg {
            entry_location: self.entry_location,
            initial_frame: self.initial_frame,
            locations,
            phi_values,
            this_value: self.this_value,
            parameter_values: self.parameter_values,
        })
    }

    pub fn solve_locations(
        &mut self,
    ) -> Result<BTreeMap<Location, AnalyzedLocation>, MokaIRBuildError> {
        let entry_location = self.entry_location;
        let initial_frame = self.initial_frame.clone();
        solver::solve(self, entry_location, initial_frame)
    }

    pub fn new_value_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        self.value_id_allocator.new_value_id()
    }

    pub fn definition_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Bytecode { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.definition_ids.get(&location) {
            return Ok(id);
        }
        let id = self.new_value_id()?;
        self.definition_ids.insert(location, id);
        Ok(id)
    }

    pub fn caught_exception_at(
        &mut self,
        location: Location,
    ) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Handler { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.caught_exception_ids.get(&location) {
            return Ok(id);
        }
        let id = self.new_value_id()?;
        self.caught_exception_ids.insert(location, id);
        Ok(id)
    }

    pub fn next_pc_of(&self, pc: ProgramCounter) -> Result<ProgramCounter, MokaIRBuildError> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }

    pub fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(self.next_pc_of(pc)?, context)
    }

    pub fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(target, context)
    }

    pub fn handler_location(
        &mut self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.handler(handler, context)
    }

    pub fn unwind_location(&mut self) -> Result<Location, MokaIRBuildError> {
        self.normalizer.register(Location::Unwind)
    }

    pub fn enter_subroutine(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<
        (
            Location,
            crate::ir::generator::jvm::normalization::ReturnAddress,
        ),
        MokaIRBuildError,
    > {
        self.normalizer.enter(location, target, continuation)
    }

    pub fn return_from(
        &mut self,
        location: Location,
        address: crate::ir::generator::jvm::normalization::ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        self.normalizer.return_from(location, address)
    }
}

#[derive(Debug, Default)]
pub(super) struct ValueIdAllocator {
    pub(super) next_value_idx: u32,
}

impl ValueIdAllocator {
    fn new_value_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        let id = SsaValueId::new(self.next_value_idx);
        self.next_value_idx = self
            .next_value_idx
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(id)
    }
}
