//! Test-only validation of completed Moka IR invariants.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{
    BlockId, EdgeId, InstructionLocation, MokaIRMethod, TerminatorKind, ValueDefinition, ValueId,
    control_flow::ControlTransfer,
};

type VerificationResult = Result<(), String>;

#[derive(Debug, Clone, Copy)]
enum UseSite {
    Operation {
        block: BlockId,
        index: usize,
    },
    Terminator {
        block: BlockId,
    },
    PhiInput {
        block: BlockId,
        predecessor: BlockId,
    },
}

impl UseSite {
    const fn evaluation_block(self) -> BlockId {
        match self {
            Self::Operation { block, .. } | Self::Terminator { block } => block,
            Self::PhiInput { predecessor, .. } => predecessor,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DefinitionSite {
    External,
    BlockEntry(BlockId),
    Operation { block: BlockId, index: usize },
}

#[derive(Debug, Clone)]
struct EdgeSensitiveDefinition {
    block: BlockId,
    normal_edges: BTreeSet<EdgeId>,
}

struct DefinitionIndex {
    definitions: BTreeMap<ValueId, ValueDefinition>,
    sites: BTreeMap<ValueId, DefinitionSite>,
}

/// Verifies the invariants promised by a completed generated method.
pub(crate) fn verify(method: &MokaIRMethod) -> VerificationResult {
    let blocks = collect_blocks(method)?;
    let predecessors = verify_edges(method, &blocks)?;
    verify_reachability(method, &blocks)?;
    let dominators = compute_dominators(method, &blocks, &predecessors)?;
    let definitions = collect_definitions(method)?;
    let edge_sensitive = collect_edge_sensitive_definitions(method);

    verify_definition_index(method, &definitions.definitions)?;
    verify_phis(
        method,
        &predecessors,
        &definitions.sites,
        &dominators,
        &edge_sensitive,
    )?;
    verify_instruction_uses(method, &definitions.sites, &dominators, &edge_sensitive)?;
    verify_source_map(method)?;
    Ok(())
}

fn collect_blocks(method: &MokaIRMethod) -> Result<BTreeSet<BlockId>, String> {
    let mut blocks = BTreeSet::new();
    for (block, _) in method.blocks() {
        if !blocks.insert(block) {
            return Err(format!("block {block} is defined more than once"));
        }
    }
    if !blocks.contains(&method.entry_block()) {
        return Err(format!(
            "entry block {} has no definition",
            method.entry_block()
        ));
    }
    Ok(blocks)
}

fn verify_edges(
    method: &MokaIRMethod,
    blocks: &BTreeSet<BlockId>,
) -> Result<BTreeMap<BlockId, BTreeSet<BlockId>>, String> {
    let mut edge_ids = BTreeSet::new();
    let mut predecessors = blocks
        .iter()
        .map(|&block| (block, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for (source, block) in method.blocks() {
        for successor in block.terminator.successors() {
            if !edge_ids.insert(successor.id()) {
                return Err(format!("edge {} is defined more than once", successor.id()));
            }
            let Some(target_predecessors) = predecessors.get_mut(&successor.target()) else {
                return Err(format!(
                    "edge {} targets undefined block {}",
                    successor.id(),
                    successor.target()
                ));
            };
            target_predecessors.insert(source);
        }
    }
    Ok(predecessors)
}

fn verify_reachability(method: &MokaIRMethod, blocks: &BTreeSet<BlockId>) -> VerificationResult {
    let reachable = reachable_blocks(method, &BTreeSet::new());
    if &reachable == blocks {
        Ok(())
    } else {
        let unreachable = blocks.difference(&reachable).collect::<Vec<_>>();
        Err(format!(
            "completed IR contains unreachable blocks: {unreachable:?}"
        ))
    }
}

fn reachable_blocks(method: &MokaIRMethod, excluded_edges: &BTreeSet<EdgeId>) -> BTreeSet<BlockId> {
    let mut reachable = BTreeSet::from([method.entry_block()]);
    let mut pending = VecDeque::from([method.entry_block()]);
    while let Some(block_id) = pending.pop_front() {
        let block = method
            .block(block_id)
            .expect("the verifier only enqueues defined blocks");
        for successor in block.terminator.successors() {
            if !excluded_edges.contains(&successor.id()) && reachable.insert(successor.target()) {
                pending.push_back(successor.target());
            }
        }
    }
    reachable
}

fn compute_dominators(
    method: &MokaIRMethod,
    blocks: &BTreeSet<BlockId>,
    predecessors: &BTreeMap<BlockId, BTreeSet<BlockId>>,
) -> Result<BTreeMap<BlockId, BTreeSet<BlockId>>, String> {
    let entry = method.entry_block();
    let mut dominators = blocks
        .iter()
        .map(|&block| {
            let initial = if block == entry {
                BTreeSet::from([entry])
            } else {
                blocks.clone()
            };
            (block, initial)
        })
        .collect::<BTreeMap<_, _>>();

    loop {
        let mut changed = false;
        for &block in blocks.iter().filter(|&&block| block != entry) {
            let incoming = &predecessors[&block];
            let mut incoming = incoming.iter();
            let Some(&first) = incoming.next() else {
                return Err(format!("non-entry block {block} has no predecessor"));
            };
            let mut intersection = dominators[&first].clone();
            for predecessor in incoming {
                intersection.retain(|candidate| dominators[predecessor].contains(candidate));
            }
            intersection.insert(block);
            if dominators[&block] != intersection {
                dominators.insert(block, intersection);
                changed = true;
            }
        }
        if !changed {
            return Ok(dominators);
        }
    }
}

fn collect_definitions(method: &MokaIRMethod) -> Result<DefinitionIndex, String> {
    let mut definitions = BTreeMap::new();
    let mut sites = BTreeMap::new();
    if let Some(value) = method.this_value() {
        insert_definition(
            &mut definitions,
            &mut sites,
            value,
            ValueDefinition::This,
            DefinitionSite::External,
        )?;
    }
    for (index, &value) in method.parameter_values().iter().enumerate() {
        let index = u16::try_from(index)
            .map_err(|_| "method parameter index cannot be represented".to_owned())?;
        insert_definition(
            &mut definitions,
            &mut sites,
            value,
            ValueDefinition::Parameter(index),
            DefinitionSite::External,
        )?;
    }
    for (block_id, block) in method.blocks() {
        if let Some(value) = block.caught_exception {
            insert_definition(
                &mut definitions,
                &mut sites,
                value,
                ValueDefinition::CaughtException(block_id),
                DefinitionSite::BlockEntry(block_id),
            )?;
        }
        for (index, phi) in block.phis.iter().enumerate() {
            let location = InstructionLocation::Phi {
                block: block_id,
                index,
            };
            insert_definition(
                &mut definitions,
                &mut sites,
                phi.value,
                ValueDefinition::Instruction(location),
                DefinitionSite::BlockEntry(block_id),
            )?;
        }
        for (index, operation) in block.operations.iter().enumerate() {
            let Some(value) = operation.def() else {
                continue;
            };
            let location = InstructionLocation::Operation {
                block: block_id,
                index,
            };
            insert_definition(
                &mut definitions,
                &mut sites,
                value,
                ValueDefinition::Instruction(location),
                DefinitionSite::Operation {
                    block: block_id,
                    index,
                },
            )?;
        }
    }
    Ok(DefinitionIndex { definitions, sites })
}

fn insert_definition(
    definitions: &mut BTreeMap<ValueId, ValueDefinition>,
    sites: &mut BTreeMap<ValueId, DefinitionSite>,
    value: ValueId,
    definition: ValueDefinition,
    site: DefinitionSite,
) -> VerificationResult {
    if let Some(previous) = definitions.insert(value, definition) {
        return Err(format!(
            "value {value} has multiple definitions: {previous:?} and {definition:?}"
        ));
    }
    sites.insert(value, site);
    Ok(())
}

fn verify_definition_index(
    method: &MokaIRMethod,
    definitions: &BTreeMap<ValueId, ValueDefinition>,
) -> VerificationResult {
    for (index, &indexed_definition) in method.value_definitions().iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| "value definition index cannot be represented".to_owned())?;
        let value = ValueId::new(index);
        let actual_definition = definitions.get(&value).copied();
        if indexed_definition != actual_definition {
            return Err(match (indexed_definition, actual_definition) {
                (Some(indexed), Some(actual)) => {
                    format!("definition index for {value} is {indexed:?}, expected {actual:?}")
                }
                (Some(indexed), None) => {
                    format!("definition index contains undefined value {value} as {indexed:?}")
                }
                (None, Some(actual)) => format!(
                    "definition index has a hole for live value {value}, expected {actual:?}"
                ),
                (None, None) => unreachable!("equal empty definitions were handled above"),
            });
        }
        if let Some(ValueDefinition::Instruction(location)) = indexed_definition
            && method.instruction(location).is_none()
        {
            return Err(format!(
                "definition of {value} refers to missing instruction {location:?}"
            ));
        }
    }
    for (&value, &definition) in definitions {
        if method.definition_of(value) != Some(definition) {
            return Err(format!(
                "definition index is missing live value {value} defined as {definition:?}"
            ));
        }
    }
    Ok(())
}

fn collect_edge_sensitive_definitions(
    method: &MokaIRMethod,
) -> BTreeMap<ValueId, EdgeSensitiveDefinition> {
    method
        .blocks()
        .filter_map(|(block_id, block)| {
            if !matches!(block.terminator.kind(), TerminatorKind::Fallible) {
                return None;
            }
            let value = block.operations.last()?.def()?;
            let normal_edges = block
                .terminator
                .successors()
                .iter()
                .filter(|successor| {
                    !matches!(
                        successor.transfer(),
                        ControlTransfer::Exception(_) | ControlTransfer::Unwind
                    )
                })
                .map(super::Successor::id)
                .collect();
            Some((
                value,
                EdgeSensitiveDefinition {
                    block: block_id,
                    normal_edges,
                },
            ))
        })
        .collect()
}

fn verify_phis(
    method: &MokaIRMethod,
    predecessors: &BTreeMap<BlockId, BTreeSet<BlockId>>,
    definitions: &BTreeMap<ValueId, DefinitionSite>,
    dominators: &BTreeMap<BlockId, BTreeSet<BlockId>>,
    edge_sensitive: &BTreeMap<ValueId, EdgeSensitiveDefinition>,
) -> VerificationResult {
    for (block_id, block) in method.blocks() {
        for phi in &block.phis {
            let mut inputs = BTreeSet::new();
            for input in &phi.inputs {
                if !inputs.insert(input.predecessor) {
                    return Err(format!(
                        "phi {} has more than one input from {}",
                        phi.value, input.predecessor
                    ));
                }
                verify_use(
                    method,
                    input.value,
                    UseSite::PhiInput {
                        block: block_id,
                        predecessor: input.predecessor,
                    },
                    definitions,
                    dominators,
                    edge_sensitive,
                )?;
            }
            if inputs != predecessors[&block_id] {
                return Err(format!(
                    "phi {} covers predecessors {inputs:?}, expected {:?}",
                    phi.value, predecessors[&block_id]
                ));
            }
        }
    }
    Ok(())
}

fn verify_instruction_uses(
    method: &MokaIRMethod,
    definitions: &BTreeMap<ValueId, DefinitionSite>,
    dominators: &BTreeMap<BlockId, BTreeSet<BlockId>>,
    edge_sensitive: &BTreeMap<ValueId, EdgeSensitiveDefinition>,
) -> VerificationResult {
    for (block_id, block) in method.blocks() {
        for (index, operation) in block.operations.iter().enumerate() {
            for value in operation.uses() {
                verify_use(
                    method,
                    value,
                    UseSite::Operation {
                        block: block_id,
                        index,
                    },
                    definitions,
                    dominators,
                    edge_sensitive,
                )?;
            }
        }
        for value in block.terminator.uses() {
            verify_use(
                method,
                value,
                UseSite::Terminator { block: block_id },
                definitions,
                dominators,
                edge_sensitive,
            )?;
        }
    }
    Ok(())
}

fn verify_use(
    method: &MokaIRMethod,
    value: ValueId,
    usage: UseSite,
    definitions: &BTreeMap<ValueId, DefinitionSite>,
    dominators: &BTreeMap<BlockId, BTreeSet<BlockId>>,
    edge_sensitive: &BTreeMap<ValueId, EdgeSensitiveDefinition>,
) -> VerificationResult {
    let Some(&definition) = definitions.get(&value) else {
        return Err(format!("{value} is used at {usage:?} but is not defined"));
    };
    let use_block = usage.evaluation_block();
    let dominates = match definition {
        DefinitionSite::External => true,
        DefinitionSite::Operation { block, index } if block == use_block => match usage {
            UseSite::Operation {
                index: use_index, ..
            } => index < use_index,
            UseSite::Terminator { .. } | UseSite::PhiInput { .. } => true,
        },
        DefinitionSite::BlockEntry(block) | DefinitionSite::Operation { block, .. } => {
            dominators[&use_block].contains(&block)
        }
    };
    if !dominates {
        return Err(format!(
            "definition of {value} at {definition:?} does not dominate use at {usage:?}"
        ));
    }
    if let Some(edge_definition) = edge_sensitive.get(&value) {
        verify_edge_sensitive_use(method, value, usage, edge_definition)?;
    }
    Ok(())
}

fn verify_edge_sensitive_use(
    method: &MokaIRMethod,
    value: ValueId,
    usage: UseSite,
    definition: &EdgeSensitiveDefinition,
) -> VerificationResult {
    if definition.normal_edges.is_empty() {
        return Err(format!(
            "fallible result {value} is used despite having no normal outcome"
        ));
    }
    if let UseSite::PhiInput { block, predecessor } = usage
        && predecessor == definition.block
    {
        let transfers = method
            .block(predecessor)
            .expect("the predecessor is a defined block")
            .terminator
            .successors()
            .iter()
            .filter(|successor| successor.target() == block)
            .collect::<Vec<_>>();
        let only_normal_edges = !transfers.is_empty()
            && transfers
                .iter()
                .all(|successor| definition.normal_edges.contains(&successor.id()));
        if only_normal_edges {
            return Ok(());
        }
        return Err(format!(
            "fallible result {value} is used by a phi on a non-normal edge from {predecessor} to {block}"
        ));
    }

    let reachable_without_normal = reachable_blocks(method, &definition.normal_edges);
    if reachable_without_normal.contains(&usage.evaluation_block()) {
        return Err(format!(
            "fallible result {value} is visible at {usage:?} without taking its normal edge"
        ));
    }
    Ok(())
}

fn verify_source_map(method: &MokaIRMethod) -> VerificationResult {
    for (location, pc) in method.source_map().mappings() {
        if method.instruction(location).is_none() {
            return Err(format!(
                "source location {pc} refers to missing instruction {location:?}"
            ));
        }
        if method.source_map().origin_of(location) != Some(pc)
            || !method
                .source_map()
                .instructions_at(pc)
                .any(|candidate| candidate == location)
        {
            return Err(format!(
                "source mapping between {pc} and {location:?} is not bidirectional"
            ));
        }
    }
    Ok(())
}
