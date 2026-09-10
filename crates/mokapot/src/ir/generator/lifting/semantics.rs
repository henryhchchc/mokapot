//! Narrow services required to interpret JVM instruction semantics.

use crate::{
    ir::{
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, Value},
        },
        expression::Condition,
        generator::{
            Entry, FrameOperand, Instruction, JvmStackFrame, Location, MokaIRBuildError,
            ReturnAddress, SsaValueId,
        },
    },
    jvm::{
        ConstantValue,
        code::{MethodBody, ProgramCounter},
    },
};

type OutgoingState<OP> = (Location, ControlTransfer<OP>, JvmStackFrame<OP>);

/// Capabilities required by shared JVM lifting semantics.
pub(in crate::ir::generator) trait JvmSemantics {
    fn body(&self) -> &MethodBody;

    fn definition_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError>;

    fn caught_exception_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError>;

    fn next_pc_of(&self, pc: ProgramCounter) -> Result<ProgramCounter, MokaIRBuildError> {
        self.body()
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }

    fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError>;

    fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError>;

    fn handler_location(
        &mut self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError>;

    fn unwind_location(&mut self) -> Result<Location, MokaIRBuildError>;

    fn enter_subroutine(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError>;

    fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError>;
}

fn exception_edges<OP: FrameOperand>(
    semantics: &mut impl JvmSemantics,
    location: Location,
    pre_frame: &JvmStackFrame<OP>,
    caught_value: &impl Fn(SsaValueId) -> OP,
) -> Result<Vec<OutgoingState<OP>>, MokaIRBuildError> {
    let pc = location
        .source_pc()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let entries = semantics
        .body()
        .exception_table
        .iter()
        .filter(|entry| entry.covers(pc))
        .cloned()
        .collect::<Vec<_>>();
    let mut outgoing = Vec::with_capacity(entries.len() + 1);
    let mut exhaustive = false;
    for entry in entries {
        let handler = semantics.handler_location(location, entry.handler_pc)?;
        let caught = caught_value(semantics.caught_exception_at(handler)?);
        outgoing.push((
            handler,
            ControlTransfer::Exception(entry.catch_type.clone()),
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
        outgoing.push((
            semantics.unwind_location()?,
            ControlTransfer::Unwind,
            pre_frame.same_locals_empty_stack_frame(),
        ));
    }
    Ok(outgoing)
}

#[expect(
    clippy::too_many_lines,
    reason = "all control-flow forms are classified together"
)]
pub(in crate::ir::generator) fn outgoing_from<OP: FrameOperand>(
    semantics: &mut impl JvmSemantics,
    location: Location,
    pre_frame: &JvmStackFrame<OP>,
    normal_frame: JvmStackFrame<OP>,
    instruction: &Instruction<OP>,
    fallible: bool,
    caught_value: &impl Fn(SsaValueId) -> OP,
) -> Result<Vec<OutgoingState<OP>>, MokaIRBuildError> {
    use ControlTransfer::{Conditional, Normal, Unconditional};

    Ok(match instruction {
        Instruction::HandlerEntry => {
            let Location::Handler { handler_pc, .. } = location else {
                return Err(MokaIRBuildError::MalformedControlFlow);
            };
            vec![(
                semantics.target_location(location, handler_pc)?,
                Unconditional,
                normal_frame,
            )]
        }
        Instruction::Return(_) if fallible => {
            exception_edges(semantics, location, pre_frame, caught_value)?
        }
        Instruction::Unwind | Instruction::Return(_) => Vec::new(),
        Instruction::Throw(_) => exception_edges(semantics, location, pre_frame, caught_value)?,
        Instruction::Subroutine { target, .. } => {
            vec![(*target, Unconditional, normal_frame)]
        }
        Instruction::Definition { .. } | Instruction::Effect(_) if fallible => {
            let mut outgoing = vec![(semantics.next_location(location)?, Normal, normal_frame)];
            outgoing.extend(exception_edges(
                semantics,
                location,
                pre_frame,
                caught_value,
            )?);
            outgoing
        }
        Instruction::Erased | Instruction::Definition { .. } | Instruction::Effect(_) => vec![(
            semantics.next_location(location)?,
            Unconditional,
            normal_frame,
        )],
        Instruction::Jump {
            condition: None,
            target,
        } => vec![(
            semantics.target_location(location, *target)?,
            Unconditional,
            normal_frame,
        )],
        Instruction::Jump {
            condition: Some(condition),
            target,
        } => {
            let condition: BooleanVariable<_> = condition.clone().into();
            vec![
                (
                    semantics.target_location(location, *target)?,
                    Conditional(BranchGuard::of(condition.clone())),
                    normal_frame.same_frame(),
                ),
                (
                    semantics.next_location(location)?,
                    Conditional(BranchGuard::of(!condition)),
                    normal_frame,
                ),
            ]
        }
        Instruction::Switch {
            default,
            branches,
            match_value,
        } => {
            let mut outgoing = Vec::with_capacity(branches.len() + 1);
            for (&case, &target) in branches {
                let value = Value::Constant(ConstantValue::Integer(case));
                let condition =
                    BooleanVariable::Positive(Condition::Equal(match_value.clone().into(), value));
                outgoing.push((
                    semantics.target_location(location, target)?,
                    Conditional(BranchGuard::of(condition)),
                    normal_frame.same_frame(),
                ));
            }
            let default_guard = branches
                .keys()
                .map(|case| {
                    BooleanVariable::Negative(Condition::Equal(
                        match_value.clone().into(),
                        Value::Constant(ConstantValue::Integer(*case)),
                    ))
                })
                .collect();
            outgoing.push((
                semantics.target_location(location, *default)?,
                Conditional(default_guard),
                normal_frame,
            ));
            outgoing
        }
        Instruction::SubroutineReturn(value) => {
            let address = value
                .return_address()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            vec![(
                semantics.return_from(location, address)?,
                Unconditional,
                normal_frame,
            )]
        }
    })
}
