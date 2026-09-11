//! Abstractly executes JVM frames to determine reachable states.

pub(in crate::ir::generator) mod operand_state;

use super::{
    BTreeMap, ControlTransfer, DataflowProblem, Instruction, JvmStackFrame, Location, Method,
    MethodBody, MokaIRBuildError, NormalizedJvm, Normalizer, OperandState, ProgramCounter,
    ReturnAddress, SsaValueId, method,
};
use crate::{
    analysis::fixed_point::DataflowOutput,
    ir::generator::lifting::{
        fallibility::FallibilityContext,
        lift_instruction,
        semantics::{JvmSemantics, outgoing_from},
    },
};

/// Mutable state used only while solving JVM frame facts.
pub(in crate::ir::generator) struct JvmFrameAnalyzer<'method> {
    body: &'method MethodBody,
    fallibility: FallibilityContext,
    normalizer: Normalizer,
    definition_ids: BTreeMap<Location, SsaValueId>,
    caught_exception_ids: BTreeMap<Location, SsaValueId>,
    next_definition_id: u32,
    entry: Option<(Location, JvmStackFrame)>,
}

/// Transfer output retained for one reachable JVM location.
pub(in crate::ir::generator) struct JvmFlowOutput {
    is_explicit_transfer: bool,
    outgoing: Vec<JvmOutgoing>,
}

impl DataflowOutput<Location, JvmStackFrame> for JvmFlowOutput {
    fn successors<'a>(&'a self) -> impl Iterator<Item = (&'a Location, &'a JvmStackFrame)>
    where
        Location: 'a,
        JvmStackFrame: 'a,
    {
        self.outgoing
            .iter()
            .map(|outgoing| (&outgoing.target, &outgoing.frame))
    }

    fn into_successors(self) -> impl Iterator<Item = (Location, JvmStackFrame)> {
        self.outgoing
            .into_iter()
            .map(|outgoing| (outgoing.target, outgoing.frame))
    }
}

/// One outgoing edge and the abstract frame reaching its target.
pub(in crate::ir::generator) struct JvmOutgoing {
    pub target: Location,
    pub transfer: JvmTransferCategory,
    pub frame: JvmStackFrame,
}

/// The control-flow category of an abstract outgoing JVM edge.
///
/// Frame analysis needs only the category; exact guards are replayed while
/// constructing scalar SSA.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ir::generator) enum JvmTransferCategory {
    /// An ordinary unguarded transfer.
    Unconditional,
    /// A guarded branch transfer.
    Conditional,
    /// The normal outcome of a fallible instruction.
    Normal,
    /// An outcome caught by an exception-table entry.
    Exception,
    /// An exceptional outcome leaving the method.
    Unwind,
}

impl From<&ControlTransfer<OperandState>> for JvmTransferCategory {
    fn from(transfer: &ControlTransfer<OperandState>) -> Self {
        match transfer {
            ControlTransfer::Unconditional => Self::Unconditional,
            ControlTransfer::Conditional(_) => Self::Conditional,
            ControlTransfer::Normal => Self::Normal,
            ControlTransfer::Exception(_) => Self::Exception,
            ControlTransfer::Unwind => Self::Unwind,
        }
    }
}

/// Completed abstract-execution facts for one reachable JVM location.
pub(in crate::ir::generator) struct AnalyzedLocation {
    pub incoming: JvmStackFrame,
    pub is_explicit_transfer: bool,
    pub outgoing: Vec<JvmOutgoing>,
}

/// Immutable lookups required to replay bytecode with exact SSA operands.
pub(in crate::ir::generator) struct JvmReplayPlan {
    definition_ids: BTreeMap<Location, SsaValueId>,
    caught_exception_ids: BTreeMap<Location, SsaValueId>,
    normalized: NormalizedJvm,
}

/// Reachable JVM locations and abstract control-flow facts.
pub(in crate::ir::generator) struct AnalyzedJvmCfg {
    pub entry_location: Location,
    pub initial_frame: JvmStackFrame,
    pub locations: BTreeMap<Location, AnalyzedLocation>,
    pub replay: JvmReplayPlan,
}

impl DataflowProblem for JvmFrameAnalyzer<'_> {
    type Location = Location;
    type Fact = JvmStackFrame;
    type Err = MokaIRBuildError;
    type Output = JvmFlowOutput;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        self.entry.clone().into_iter().collect::<Vec<_>>()
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::Output, Self::Err> {
        let location = *location;
        let (instruction, outgoing) = match location {
            Location::Handler {
                handler_pc,
                context,
            } => {
                let target = self.normalizer.bytecode(handler_pc, context)?;
                (
                    Instruction::HandlerEntry,
                    vec![(target, ControlTransfer::Unconditional, fact.same_frame())],
                )
            }
            Location::Unwind => (Instruction::Unwind, Vec::new()),
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

        Ok(JvmFlowOutput {
            is_explicit_transfer: instruction.is_explicit_transfer(),
            outgoing: outgoing
                .into_iter()
                .map(|(target, transfer, frame)| JvmOutgoing {
                    target,
                    transfer: JvmTransferCategory::from(&transfer),
                    frame,
                })
                .collect(),
        })
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
            body,
            fallibility: FallibilityContext::for_method(method),
            normalizer: Normalizer::new(first_pc),
            definition_ids: BTreeMap::new(),
            caught_exception_ids: BTreeMap::new(),
            next_definition_id: 0,
            entry: Some((Location::entry(first_pc), initial_frame)),
        })
    }

    pub(super) fn run(mut self) -> Result<AnalyzedJvmCfg, MokaIRBuildError> {
        use crate::analysis::fixed_point::{FixedPointResult, solve_with_outputs};

        let result: FixedPointResult<
            BTreeMap<Location, JvmStackFrame>,
            BTreeMap<Location, JvmFlowOutput>,
        > = solve_with_outputs(&mut self)?;
        let (frames, mut outputs) = result.into_parts();
        let locations: BTreeMap<Location, AnalyzedLocation> = frames
            .into_iter()
            .map(|(location, incoming)| {
                let output = outputs
                    .remove(&location)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?;
                Ok((
                    location,
                    AnalyzedLocation {
                        incoming,
                        is_explicit_transfer: output.is_explicit_transfer,
                        outgoing: output.outgoing,
                    },
                ))
            })
            .collect::<Result<_, MokaIRBuildError>>()?;
        if !outputs.is_empty() {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        let (entry_location, initial_frame) =
            self.entry.ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(AnalyzedJvmCfg {
            entry_location,
            initial_frame,
            locations,
            replay: JvmReplayPlan {
                definition_ids: self.definition_ids,
                caught_exception_ids: self.caught_exception_ids,
                normalized: self.normalizer.finish(),
            },
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

impl JvmReplayPlan {
    pub(in crate::ir::generator) fn max_value_index(&self) -> Option<u32> {
        self.definition_ids
            .values()
            .chain(self.caught_exception_ids.values())
            .map(|value| value.index())
            .max()
    }

    pub(in crate::ir::generator) fn definition_at(
        &self,
        location: Location,
    ) -> Result<SsaValueId, MokaIRBuildError> {
        self.definition_ids
            .get(&location)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }

    pub(in crate::ir::generator) fn caught_exception_at(
        &self,
        location: Location,
    ) -> Result<SsaValueId, MokaIRBuildError> {
        self.caught_exception_ids
            .get(&location)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }

    pub(in crate::ir::generator) fn caught_exception(
        &self,
        location: Location,
    ) -> Option<SsaValueId> {
        self.caught_exception_ids.get(&location).copied()
    }

    pub(in crate::ir::generator) fn next_location(
        &self,
        body: &MethodBody,
        location: Location,
    ) -> Result<Location, MokaIRBuildError> {
        let pc = location
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let next = body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalized.bytecode(next, context)
    }

    pub(in crate::ir::generator) fn target_location(
        &self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalized.bytecode(target, context)
    }

    pub(in crate::ir::generator) fn handler_location(
        &self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        let context = location
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.normalized.handler(handler, context)
    }

    pub(in crate::ir::generator) fn unwind_location(&self) -> Result<Location, MokaIRBuildError> {
        self.normalized.unwind()
    }

    pub(in crate::ir::generator) fn enter_subroutine(
        &self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError> {
        self.normalized.enter(location, target, continuation)
    }

    pub(in crate::ir::generator) fn return_from(
        &self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        self.normalized.return_from(location, address)
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        analysis::fixed_point::solve, ir::generator::tests::method,
        jvm::code::Instruction as JvmInstruction,
    };

    #[test]
    fn reprocessing_loop_locations_reuses_definition_identities() {
        let method = method(
            [
                (0.into(), JvmInstruction::IConst0),
                (1.into(), JvmInstruction::IStore0),
                (2.into(), JvmInstruction::ILoad0),
                (3.into(), JvmInstruction::IConst1),
                (4.into(), JvmInstruction::IAdd),
                (5.into(), JvmInstruction::IStore0),
                (6.into(), JvmInstruction::Goto(2.into())),
            ],
            "()V",
            vec![],
        );
        let mut analyzer = JvmFrameAnalyzer::for_method(&method).expect("valid method");
        let _: BTreeMap<Location, JvmStackFrame> = solve(&mut analyzer).expect("valid loop");

        assert_eq!(analyzer.definition_ids.len(), 7);
        assert_eq!(analyzer.next_definition_id, 7);
    }
}
