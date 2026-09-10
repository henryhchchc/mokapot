//! Constructs and simplifies scalar SSA from planned basic blocks.

mod construction;
mod merge;
mod model;
mod simplify;
pub(in crate::ir::generator) mod value;

use super::block_formation::BlockLayout;
use super::{
    BTreeMap, BlockId, FrameOperand, JvmFrameAnalysis, JvmStackFrame, LiftedControlTransfer,
    LiftedInstruction, Location, MokaIRBuildError, OperandState, ReturnAddress, SsaFrameValue,
    SsaValueId, jvm_frame, method,
};

pub(in crate::ir::generator) use merge::{collect_phi_candidates, unavailable_value_slots};
pub(in crate::ir::generator) use model::{
    PairedFrameValue, SsaArm, SsaBlock, SsaEntryFrames, next_ssa_value,
};
pub(in crate::ir::generator) use simplify::{SimplifiedPhis, simplify_phis};

/// The internal SSA representation consumed by `MokaIR` emission.
pub(super) struct SsaMethod<'method> {
    pub(super) analysis: JvmFrameAnalysis<'method>,
    pub(super) entry: BlockId,
    pub(super) bytecode_entry: BlockId,
    pub(super) needs_entry_preheader: bool,
    pub(super) blocks: Vec<SsaBlock>,
    pub(super) phi_blocks: BTreeMap<SsaValueId, BlockId>,
    pub(super) simplified_phis: SimplifiedPhis,
    pub(super) this_value: Option<SsaValueId>,
    pub(super) parameter_values: Vec<SsaValueId>,
}

/// Constructs exact SSA frames, blocks, and phis from a planned block layout.
pub(super) fn construct(layout: BlockLayout<'_>) -> Result<SsaMethod<'_>, MokaIRBuildError> {
    let BlockLayout {
        mut analysis,
        entry,
        bytecode_entry,
        needs_entry_preheader,
        plans,
        location_to_block,
    } = layout;
    let mut next_value = analysis
        .value_ids
        .values()
        .chain(analysis.caught_exception_ids.values())
        .map(|value| value.index())
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let this_value = if analysis
        .method
        .access_flags
        .contains(method::AccessFlags::STATIC)
    {
        None
    } else {
        Some(next_ssa_value(&mut next_value)?)
    };
    let parameter_values = analysis
        .method
        .descriptor
        .parameters_types
        .iter()
        .map(|_| next_ssa_value(&mut next_value))
        .collect::<Result<Vec<_>, _>>()?;
    let frame_this = this_value.map(SsaFrameValue::Value);
    let frame_parameters = parameter_values
        .iter()
        .copied()
        .map(SsaFrameValue::Value)
        .collect::<Vec<_>>();
    let initial_frame = JvmStackFrame::with_inputs(
        &analysis.method.descriptor,
        analysis.body.max_locals,
        analysis.body.max_stack,
        frame_this,
        &frame_parameters,
    )?;

    let (entry_frames, phi_blocks) = analysis.ssa_entry_frames(
        &plans,
        &analysis.facts,
        bytecode_entry,
        needs_entry_preheader,
        &initial_frame,
        this_value,
        &parameter_values,
        &mut next_value,
    )?;
    let blocks = analysis.construct_ssa_blocks(&plans, entry_frames, &location_to_block)?;
    let candidates = collect_phi_candidates(
        &blocks,
        &phi_blocks,
        needs_entry_preheader.then_some((bytecode_entry, &initial_frame)),
    )?;
    let simplified_phis =
        simplify_phis(candidates).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;

    Ok(SsaMethod {
        analysis,
        entry,
        bytecode_entry,
        needs_entry_preheader,
        blocks,
        phi_blocks,
        simplified_phis,
        this_value,
        parameter_values,
    })
}
