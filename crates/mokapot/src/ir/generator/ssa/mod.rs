//! Constructs and simplifies scalar SSA from planned basic blocks.

mod construction;
mod finalization;
mod merge;
mod model;
mod simplify;
pub(in crate::ir::generator) mod value;

use super::block_formation::{BlockEntry, JvmBlockGraph};
use super::{
    BTreeMap, BlockId, ControlTransfer, FrameOperand, Instruction, JvmReplayPlan, JvmStackFrame,
    Location, Method, MokaIRBuildError, OperandState, OperationKind, ReturnAddress, SsaFrameValue,
    SsaValueId, TerminatorKind, jvm_frame, method,
};
use merge::{collect_phi_candidates, unavailable_value_slots};
pub(in crate::ir::generator) use model::SsaBlock;
use model::{ReplayedArm, ReplayedBlock, SsaEntryFrames, SsaPhi, SsaSuccessor, next_ssa_value};
use simplify::simplify_phis;

/// Fully lowered scalar SSA consumed by final identity allocation and emission.
pub(super) struct SsaGraph {
    pub entry: BlockId,
    pub blocks: Vec<SsaBlock>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Replays JVM blocks, simplifies phis, and discards JVM construction state.
pub(super) fn construct(
    method: &Method,
    graph: JvmBlockGraph,
) -> Result<SsaGraph, MokaIRBuildError> {
    let JvmBlockGraph {
        entry,
        initial_frame: analyzed_initial_frame,
        blocks: jvm_blocks,
        location_to_block,
        replay,
    } = graph;
    let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
    let mut next_value = replay
        .max_value_index()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let this_value = (!method.access_flags.contains(method::AccessFlags::STATIC))
        .then(|| next_ssa_value(&mut next_value))
        .transpose()?;
    let parameter_values = method
        .descriptor
        .parameters_types
        .iter()
        .map(|_| next_ssa_value(&mut next_value))
        .collect::<Result<Vec<_>, _>>()?;
    let frame_parameters = parameter_values
        .iter()
        .copied()
        .map(SsaFrameValue::Value)
        .collect::<Vec<_>>();
    let initial_frame = JvmStackFrame::with_inputs(
        &method.descriptor,
        body.max_locals,
        body.max_stack,
        this_value.map(SsaFrameValue::Value),
        &frame_parameters,
    )?;
    let (entry_frames, phi_blocks) = construction::entry_frames(
        &jvm_blocks,
        entry,
        &initial_frame,
        &analyzed_initial_frame,
        this_value,
        &parameter_values,
        &mut next_value,
    )?;
    let blocks = construction::construct_blocks(
        method,
        &replay,
        &jvm_blocks,
        entry_frames,
        &location_to_block,
    )?;
    let preheader = match entry {
        BlockEntry::Direct(_) => None,
        BlockEntry::Preheader {
            synthetic,
            bytecode,
        } => Some((bytecode, synthetic, &initial_frame)),
    };
    let candidates = collect_phi_candidates(&blocks, &phi_blocks, preheader)?;
    let simplified =
        simplify_phis(candidates).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
    let blocks = finalization::finalize(entry, &replay, blocks, &phi_blocks, simplified)?;
    Ok(SsaGraph {
        entry: entry.method_entry(),
        blocks,
        this_value,
        parameter_values,
    })
}
