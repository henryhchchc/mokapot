use super::super::block_formation::{BlockEntry, JvmBlock};
use super::{
    BTreeMap, BlockId, Instruction, JvmReplayPlan, JvmStackFrame, Location, Method,
    MokaIRBuildError, OperandState, ReplayedArm, ReplayedBlock, ReturnAddress, SsaEntryFrames,
    SsaFrameValue, SsaValueId, next_ssa_value, unavailable_value_slots,
};
use crate::ir::generator::lifting::{
    fallibility::FallibilityContext,
    lift_instruction,
    semantics::{JvmSemantics, outgoing_from},
};
use crate::jvm::code::{MethodBody, ProgramCounter};

pub(super) fn entry_frames(
    blocks: &[JvmBlock],
    entry: BlockEntry,
    initial_frame: &JvmStackFrame<SsaFrameValue>,
    analyzed_initial_frame: &JvmStackFrame,
    this_temp: Option<SsaValueId>,
    parameter_temps: &[SsaValueId],
    next_temp: &mut u32,
) -> Result<SsaEntryFrames, MokaIRBuildError> {
    let bytecode_entry = entry.bytecode_entry();
    let has_preheader = matches!(entry, BlockEntry::Preheader { .. });
    let mut frames = BTreeMap::new();
    let mut phi_blocks = BTreeMap::new();
    for block in blocks {
        if block.id == bytecode_entry && !has_preheader {
            frames.insert(block.id, initial_frame.clone());
            continue;
        }
        let leader = *block
            .locations
            .first()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let analyzed = &block.analyzed_entry;
        if matches!(leader, Location::Unwind) {
            let frame = analyzed.without_values().try_map_values(
                |_| -> Result<SsaFrameValue, MokaIRBuildError> {
                    Err(MokaIRBuildError::MalformedControlFlow)
                },
            )?;
            frames.insert(block.id, frame);
            continue;
        }
        let mut incoming = block.incoming_frames.iter().collect::<Vec<_>>();
        if block.id == bytecode_entry && has_preheader {
            incoming.push(analyzed_initial_frame);
        }
        let (unavailable_locals, unavailable_stack) = unavailable_value_slots(analyzed, &incoming)?;
        let mut frame = analyzed.try_map_values(|value| {
            Ok(match value {
                OperandState::This => {
                    SsaFrameValue::Value(this_temp.ok_or(MokaIRBuildError::MalformedControlFlow)?)
                }
                OperandState::Arg(index) => SsaFrameValue::Value(
                    parameter_temps
                        .get(usize::from(*index))
                        .copied()
                        .ok_or(MokaIRBuildError::MalformedControlFlow)?,
                ),
                OperandState::Local(value) | OperandState::CaughtException(value) => {
                    SsaFrameValue::Value(*value)
                }
                OperandState::ReturnAddress(address) => SsaFrameValue::ReturnAddress(*address),
                OperandState::Merged => {
                    let value = next_ssa_value(next_temp)?;
                    phi_blocks.insert(value, block.id);
                    SsaFrameValue::Value(value)
                }
                OperandState::Invalid => {
                    return Err(MokaIRBuildError::MalformedControlFlow);
                }
            })
        })?;
        frame.invalidate_values_at(unavailable_locals, unavailable_stack);
        frames.insert(block.id, frame);
    }
    Ok((frames, phi_blocks))
}

pub(super) fn construct_blocks(
    method: &Method,
    replay_plan: &JvmReplayPlan,
    jvm_blocks: &[JvmBlock],
    mut entry_frames: BTreeMap<BlockId, JvmStackFrame<SsaFrameValue>>,
    location_to_block: &BTreeMap<Location, BlockId>,
) -> Result<Vec<ReplayedBlock>, MokaIRBuildError> {
    let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
    let mut replay = SsaReplay::new(body, replay_plan);
    let fallibility = FallibilityContext::for_method(method);
    let mut blocks = Vec::with_capacity(jvm_blocks.len());
    for block in jvm_blocks {
        let entry_frame = entry_frames
            .remove(&block.id)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let mut frame = entry_frame.clone();
        let mut instructions = Vec::with_capacity(block.locations.len());
        let mut arms = Vec::new();
        for (index, &location) in block.locations.iter().enumerate() {
            let pre_frame = frame.clone();
            let (instruction, fallible) = match location {
                Location::Bytecode { pc, .. } => {
                    let jvm_instruction = body
                        .instruction_at(pc)
                        .ok_or(MokaIRBuildError::MalformedControlFlow)?
                        .clone();
                    let instruction =
                        lift_instruction(&mut replay, &jvm_instruction, location, &mut frame)?;
                    (
                        instruction,
                        fallibility.is_synchronously_fallible(&jvm_instruction),
                    )
                }
                Location::Handler { .. } => (Instruction::HandlerEntry, false),
                Location::Unwind => (Instruction::Unwind, false),
            };
            let is_last = index + 1 == block.locations.len();
            if is_last {
                arms = outgoing_from(
                    &mut replay,
                    location,
                    &pre_frame,
                    frame.clone(),
                    &instruction,
                    fallible,
                    &|value| SsaFrameValue::Value(value),
                )?
                .into_iter()
                .map(|(target, transfer, frame)| {
                    location_to_block
                        .get(&target)
                        .copied()
                        .map(|target| ReplayedArm {
                            target,
                            transfer,
                            frame,
                        })
                        .ok_or(MokaIRBuildError::MalformedControlFlow)
                })
                .collect::<Result<_, _>>()?;
            } else if instruction.is_explicit_transfer() {
                return Err(MokaIRBuildError::MalformedControlFlow);
            }
            instructions.push((location, instruction));
        }
        blocks.push(ReplayedBlock {
            id: block.id,
            entry_frame,
            instructions,
            arms,
        });
    }
    Ok(blocks)
}

struct SsaReplay<'plan, 'body> {
    body: &'body MethodBody,
    plan: &'plan JvmReplayPlan,
}

impl<'plan, 'body> SsaReplay<'plan, 'body> {
    const fn new(body: &'body MethodBody, plan: &'plan JvmReplayPlan) -> Self {
        Self { body, plan }
    }
}

impl JvmSemantics for SsaReplay<'_, '_> {
    fn body(&self) -> &MethodBody {
        self.body
    }

    fn definition_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        self.plan.definition_at(location)
    }

    fn caught_exception_at(&mut self, location: Location) -> Result<SsaValueId, MokaIRBuildError> {
        self.plan.caught_exception_at(location)
    }

    fn next_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        self.plan.next_location(self.body, location)
    }

    fn target_location(
        &mut self,
        location: Location,
        target: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        self.plan.target_location(location, target)
    }

    fn handler_location(
        &mut self,
        location: Location,
        handler: ProgramCounter,
    ) -> Result<Location, MokaIRBuildError> {
        self.plan.handler_location(location, handler)
    }

    fn unwind_location(&mut self) -> Result<Location, MokaIRBuildError> {
        self.plan.unwind_location()
    }

    fn enter_subroutine(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError> {
        self.plan.enter_subroutine(location, target, continuation)
    }

    fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        self.plan.return_from(location, address)
    }
}
