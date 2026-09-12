//! Resolves formed JVM blocks into scalar SSA blocks.
use super::{
    BTreeMap, BlockId, MokaIRBuildError, SsaBlock, SsaPhi, SsaSuccessor, SsaValueId,
    merge::MergePlan, simplify::SimplifiedPhis,
};
use crate::ir::TryMapValues;
use crate::ir::generator::block_formation::{JvmBlock, JvmBlockArm};

type PhiCandidate = (SsaValueId, Vec<(BlockId, SsaValueId)>);

pub(super) fn finalize(
    blocks: Vec<JvmBlock>,
    merge_plan: &MergePlan,
    simplified: SimplifiedPhis,
) -> Result<Vec<SsaBlock>, MokaIRBuildError> {
    // `simplify_phis` returns substitutions whose targets are already canonical.
    let canonical = |value| {
        simplified
            .substitutions
            .get(&value)
            .copied()
            .unwrap_or(value)
    };
    let mut phis_by_block = BTreeMap::<BlockId, Vec<PhiCandidate>>::new();
    for (value, inputs) in simplified.candidates {
        let block = merge_plan
            .block_for(value)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        phis_by_block
            .entry(block)
            .or_default()
            .push((value, inputs));
    }
    let mut finalized = Vec::with_capacity(blocks.len());
    for block in blocks {
        let id = block.id;
        finalized.push(finalize_block(
            block,
            phis_by_block.remove(&id).unwrap_or_default(),
            merge_plan,
            &canonical,
        )?);
    }
    if phis_by_block.is_empty() {
        Ok(finalized)
    } else {
        Err(MokaIRBuildError::MalformedControlFlow)
    }
}
fn finalize_block(
    block: JvmBlock,
    phis: Vec<PhiCandidate>,
    merge_plan: &MergePlan,
    canonical: &impl Fn(SsaValueId) -> SsaValueId,
) -> Result<SsaBlock, MokaIRBuildError> {
    let JvmBlock {
        id,
        entry_frame: _,
        operations,
        terminator,
        terminator_source,
        arms,
        caught_exception,
    } = block;
    let resolve = |operand| merge_plan.resolve(operand).map(canonical);
    let caught_exception = caught_exception.map(canonical);
    let phis = phis
        .into_iter()
        .map(|(value, inputs)| SsaPhi {
            value: canonical(value),
            inputs: inputs
                .into_iter()
                .map(|(predecessor, value)| (predecessor, canonical(value)))
                .collect(),
        })
        .collect();
    let operations = operations
        .into_iter()
        .map(|(source, operation)| {
            operation
                .try_map_values(&resolve)
                .map(|operation| (source, operation))
        })
        .collect::<Result<_, _>>()?;
    let terminator = terminator.try_map_values(&resolve)?;
    let successors = arms
        .into_iter()
        .map(
            |JvmBlockArm {
                 target,
                 transfer,
                 frame: _,
             }| {
                Ok(SsaSuccessor {
                    target,
                    transfer: transfer.try_map_values(&resolve)?,
                })
            },
        )
        .collect::<Result<_, MokaIRBuildError>>()?;
    Ok(SsaBlock {
        id,
        caught_exception,
        phis,
        operations,
        terminator,
        terminator_source,
        successors,
    })
}
