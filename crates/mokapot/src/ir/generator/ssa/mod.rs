//! Constructs and simplifies scalar SSA from planned basic blocks.

mod construction;
mod merge;
mod model;
mod simplify;
pub(in crate::ir::generator) mod value;

use super::block_formation::{BlockEntry, JvmBlockGraph};
use super::{
    BTreeMap, BlockId, ControlTransfer, FrameOperand, Instruction, JvmReplayPlan, JvmStackFrame,
    Location, Method, MokaIRBuildError, OperandState, ReturnAddress, SsaFrameValue, SsaValueId,
    jvm_frame, method,
};

use merge::{collect_phi_candidates, unavailable_value_slots};
use model::{SsaArm, SsaBlock, SsaEntryFrames, SsaPhi, next_ssa_value};
use simplify::simplify_phis;

/// The internal SSA representation consumed by `MokaIR` emission.
pub(super) struct SsaGraph {
    pub entry: BlockEntry,
    pub caught_exceptions: BTreeMap<BlockId, SsaValueId>,
    pub blocks: Vec<SsaBlock>,
    pub phis: Vec<SsaPhi>,
    pub value_aliases: BTreeMap<SsaValueId, SsaValueId>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// Constructs exact SSA frames, blocks, and phis from a block-level JVM graph.
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
    let bytecode_entry = entry.bytecode_entry();
    let has_entry_preheader = matches!(entry, BlockEntry::Preheader { .. });
    let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
    let mut next_value = replay
        .max_value_index()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let this_value = if method.access_flags.contains(method::AccessFlags::STATIC) {
        None
    } else {
        Some(next_ssa_value(&mut next_value)?)
    };
    let parameter_values = method
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
        &method.descriptor,
        body.max_locals,
        body.max_stack,
        frame_this,
        &frame_parameters,
    )?;

    let caught_exceptions = jvm_blocks
        .iter()
        .filter_map(|block| {
            block
                .locations
                .first()
                .and_then(|&leader| replay.caught_exception(leader))
                .map(|value| (block.id, value))
        })
        .collect();
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
    let candidates = collect_phi_candidates(
        &blocks,
        &phi_blocks,
        has_entry_preheader.then_some((bytecode_entry, &initial_frame)),
    )?;
    let simplified_phis =
        simplify_phis(candidates).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
    let phis = simplified_phis
        .candidates
        .into_iter()
        .map(|(value, inputs)| {
            let block = phi_blocks
                .get(&value)
                .copied()
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            Ok(SsaPhi {
                block,
                value,
                inputs,
            })
        })
        .collect::<Result<Vec<_>, MokaIRBuildError>>()?;

    Ok(SsaGraph {
        caught_exceptions,
        entry,
        blocks,
        phis,
        value_aliases: simplified_phis.substitutions,
        this_value,
        parameter_values,
    })
}
