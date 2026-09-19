//! Conversion from analyzed frames to a frame-free scalar graph.

mod model;

pub(crate) use model::{PhiCandidate, ScalarBlock, ScalarGraph, ScalarSuccessor};

use std::collections::BTreeMap;

use super::analysis::{CompletedAnalysis, LiftedBlock, PhiDefinition, PhiSite, Predecessor};
use crate::ir::{BlockId, SourceMap, ValueId, generator::error::Error};

pub(super) fn materialize(
    completed: CompletedAnalysis,
    entry: BlockId,
) -> Result<(ScalarGraph, SourceMap), Error> {
    let CompletedAnalysis {
        blocks,
        phi_definitions,
        receiver_value,
        parameter_values,
    } = completed;
    let mut source_map = SourceMap::default();
    let scalar_blocks = blocks.iter().map(|(&id, state)| {
        let analyzed = state
            .execution
            .block()
            .cloned()
            .ok_or_else(|| Error::internal("a normalized reachable block was not executed"))?;
        Ok((id, materialize_block(id, analyzed, &mut source_map)))
    });
    let blocks = scalar_blocks.collect::<Result<BTreeMap<_, _>, Error>>()?;

    let phi_candidates = materialize_phis(&phi_definitions, &blocks)?;
    Ok((
        ScalarGraph {
            entry,
            blocks,
            phi_candidates,
            this_value: receiver_value,
            parameter_values,
        },
        source_map,
    ))
}

fn materialize_phis(
    phi_definitions: &BTreeMap<PhiSite, PhiDefinition>,
    blocks: &BTreeMap<BlockId, ScalarBlock>,
) -> Result<BTreeMap<ValueId, PhiCandidate>, Error> {
    phi_definitions
        .iter()
        .filter(|(site, _)| blocks.contains_key(&site.block))
        .map(|(site, definition)| {
            materialize_phi(site.block, definition).map(|candidate| (definition.result, candidate))
        })
        .collect()
}

fn materialize_phi(block: BlockId, definition: &PhiDefinition) -> Result<PhiCandidate, Error> {
    let inputs = definition
        .inputs
        .iter()
        .map(|(predecessor, &input)| {
            let Predecessor::Block(predecessor) = predecessor else {
                return Err(Error::internal(
                    "an entry contribution cannot be an active phi input",
                ));
            };
            Ok((*predecessor, input))
        })
        .collect::<Result<_, Error>>()?;
    Ok(PhiCandidate {
        placement: block,
        inputs,
    })
}

fn materialize_block(id: BlockId, lifted: LiftedBlock, source_map: &mut SourceMap) -> ScalarBlock {
    let successors = lifted
        .successors
        .edges
        .into_iter()
        .map(|successor| ScalarSuccessor {
            id: successor.id,
            target: successor.target,
            transfer: successor.transfer,
        })
        .collect();
    let operations = lifted
        .operations
        .into_iter()
        .enumerate()
        .map(|(index, (pc, operation))| {
            source_map.record_operation(pc, id, index);
            operation
        })
        .collect();
    let terminator = lifted.terminator;
    if let Some(pc) = lifted.terminator_source {
        source_map.record_terminator(pc, id);
    }
    ScalarBlock {
        caught_exception: lifted.caught_exception,
        operations,
        terminator,
        successors,
    }
}
