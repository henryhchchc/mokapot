//! Abstractly executes JVM frames to determine reachable states.

pub(in crate::ir::generator) mod operand_state;

use super::{
    BTreeMap, DataflowProblem, JvmStackFrame, LegacyNormalizer, LiftedControlTransfer,
    LiftedInstruction, Location, Method, MethodBody, MokaIRBuildError, NormalizedJvm, OperandState,
    ProgramCounter, ReturnAddress, SsaValueId, method,
};
use crate::ir::generator::lifting::{
    fallibility::FallibilityContext,
    lift_instruction,
    semantics::{JvmSemantics, outgoing_from},
};

/// Mutable state used only while solving JVM frame facts.
pub(in crate::ir::generator) struct JvmFrameAnalyzer<'method> {
    method: &'method Method,
    body: &'method MethodBody,
    fallibility: FallibilityContext,
    normalizer: LegacyNormalizer,
    definition_ids: BTreeMap<Location, SsaValueId>,
    caught_exception_ids: BTreeMap<Location, SsaValueId>,
    next_definition_id: u32,
    instructions: BTreeMap<Location, LiftedInstruction>,
    successors: BTreeMap<Location, Vec<(Location, LiftedControlTransfer<OperandState>)>>,
    successor_frames: BTreeMap<Location, Vec<JvmStackFrame>>,
    entry: Option<(Location, JvmStackFrame)>,
}

/// Reachable JVM locations, their incoming frames, and normalized control flow.
pub(in crate::ir::generator) struct JvmFrameFacts<'method> {
    pub method: &'method Method,
    pub body: &'method MethodBody,
    pub entry: (Location, JvmStackFrame),
    pub frames: BTreeMap<Location, JvmStackFrame>,
    pub instructions: BTreeMap<Location, LiftedInstruction>,
    pub successors: BTreeMap<Location, Vec<(Location, LiftedControlTransfer<OperandState>)>>,
    pub successor_frames: BTreeMap<Location, Vec<JvmStackFrame>>,
    pub definition_ids: BTreeMap<Location, SsaValueId>,
    pub caught_exception_ids: BTreeMap<Location, SsaValueId>,
    pub normalized: NormalizedJvm,
}

impl DataflowProblem for JvmFrameAnalyzer<'_> {
    type Location = Location;
    type Fact = JvmStackFrame;
    type Err = MokaIRBuildError;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        self.entry.clone().into_iter().collect::<Vec<_>>()
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
                let target = self.normalizer.bytecode(handler_pc, context)?;
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
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?
                    .clone();
                let fallible = self.fallibility.is_synchronously_fallible(&jvm_instruction);
                let instruction =
                    lift_instruction(self, &jvm_instruction, location, &mut normal_frame)?;
                let outgoing = outgoing_from(
                    self,
                    location,
                    &pre_frame,
                    normal_frame,
                    &instruction,
                    fallible,
                    &OperandState::CaughtException,
                )?;
                (instruction, outgoing)
            }
        };

        self.instructions.insert(location, instruction);
        self.successors.insert(
            location,
            outgoing
                .iter()
                .map(|(target, transfer, _)| (*target, transfer.clone()))
                .collect(),
        );
        self.successor_frames.insert(
            location,
            outgoing.iter().map(|(_, _, frame)| frame.clone()).collect(),
        );
        Ok(outgoing
            .into_iter()
            .map(|(target, _, frame)| (target, frame))
            .collect::<Vec<_>>())
    }
}

impl<'method> JvmFrameAnalyzer<'method> {
    pub(super) fn for_method(method: &'method Method) -> Result<Self, MokaIRBuildError> {
        let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
        let first_pc = body
            .instructions
            .entry_point()
            .map(|(pc, _)| *pc)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let initial_frame = JvmStackFrame::new(
            method.access_flags.contains(method::AccessFlags::STATIC),
            &method.descriptor,
            body.max_locals,
            body.max_stack,
        )?;

        Ok(Self {
            method,
            body,
            fallibility: FallibilityContext::for_method(method),
            normalizer: LegacyNormalizer::new(first_pc),
            definition_ids: BTreeMap::new(),
            caught_exception_ids: BTreeMap::new(),
            next_definition_id: 0,
            instructions: BTreeMap::new(),
            successors: BTreeMap::new(),
            successor_frames: BTreeMap::new(),
            entry: Some((Location::entry(first_pc), initial_frame)),
        })
    }

    pub(super) fn run(mut self) -> Result<JvmFrameFacts<'method>, MokaIRBuildError> {
        use crate::analysis::fixed_point::solve;

        let frames = solve(&mut self)?;
        Ok(JvmFrameFacts {
            method: self.method,
            body: self.body,
            entry: self.entry.ok_or(MokaIRBuildError::MalformedControlFlow)?,
            frames,
            instructions: self.instructions,
            successors: self.successors,
            successor_frames: self.successor_frames,
            definition_ids: self.definition_ids,
            caught_exception_ids: self.caught_exception_ids,
            normalized: self.normalizer.finish(),
        })
    }

    fn next_definition_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        let id = SsaValueId::new(self.next_definition_id);
        self.next_definition_id = self
            .next_definition_id
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(id)
    }
}

impl JvmSemantics for JvmFrameAnalyzer<'_> {
    fn body(&self) -> &MethodBody {
        self.body
    }

    fn definition_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Bytecode { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.definition_ids.get(&location) {
            return Ok(id);
        }
        let id = self.next_definition_id()?;
        self.definition_ids.insert(location, id);
        Ok(id)
    }

    fn caught_exception_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(location, Location::Handler { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.caught_exception_ids.get(&location) {
            return Ok(id);
        }
        let id = self.next_definition_id()?;
        self.caught_exception_ids.insert(location, id);
        Ok(id)
    }

    fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(self.next_pc_of(pc)?, context)
    }

    fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.bytecode(target, context)
    }

    fn handler_location(
        &mut self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalizer.handler(handler, context)
    }

    fn unwind_location(&mut self) -> Result<Location, MokaIRBuildError> {
        self.normalizer.register(Location::Unwind)
    }

    fn enter_subroutine(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError> {
        self.normalizer.enter(location, target, continuation)
    }

    fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        self.normalizer.return_from(location, address)
    }
}
