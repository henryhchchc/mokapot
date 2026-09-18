//! Test-only validation of completed Moka IR invariants.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::{
    BlockId, BlockKind, EdgeId, InstructionLocation, MokaIRMethod, Terminator, ValueDefinition,
    ValueId,
};

type VerificationResult = Result<(), String>;

#[derive(Debug, Clone, Copy)]
enum UseSite {
    Operation { block: BlockId, index: usize },
    Terminator { block: BlockId },
    EdgeArgument { edge: EdgeId, source: BlockId },
}

impl UseSite {
    const fn evaluation_block(self) -> BlockId {
        match self {
            Self::Operation { block, .. } | Self::Terminator { block } => block,
            Self::EdgeArgument { source, .. } => source,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DefinitionSite {
    External,
    BlockEntry(BlockId),
    Operation { block: BlockId, index: usize },
    Terminator { block: BlockId, normal_edge: EdgeId },
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

    verify_definition_index(method, &definitions.definitions)?;
    verify_block_arguments(method, &definitions.sites, &dominators)?;
    verify_instruction_uses(method, &definitions.sites, &dominators)?;
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
    if method.entry().arguments().len()
        != method
            .block(method.entry_block())
            .expect("the entry block was checked above")
            .parameters
            .len()
    {
        return Err("method-entry argument count differs from entry parameter count".to_owned());
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
            let Some(target) = successor.block_target() else {
                continue;
            };
            let Some(target_predecessors) = predecessors.get_mut(&target) else {
                return Err(format!(
                    "edge {} targets undefined block {}",
                    successor.id(),
                    target
                ));
            };
            let expected = method
                .block(target)
                .expect("the successor target was checked above")
                .parameters
                .len();
            if successor.arguments().len() != expected {
                return Err(format!(
                    "edge {} supplies {} arguments to {} parameters",
                    successor.id(),
                    successor.arguments().len(),
                    expected
                ));
            }
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
            let Some(target) = successor.block_target() else {
                continue;
            };
            if !excluded_edges.contains(&successor.id()) && reachable.insert(target) {
                pending.push_back(target);
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
        if let BlockKind::LandingPad { exception: value } = block.kind {
            insert_definition(
                &mut definitions,
                &mut sites,
                value,
                ValueDefinition::CaughtException(block_id),
                DefinitionSite::BlockEntry(block_id),
            )?;
        }
        for (index, parameter) in block.parameters.iter().enumerate() {
            let location = InstructionLocation::BlockParameter {
                block: block_id,
                index,
            };
            insert_definition(
                &mut definitions,
                &mut sites,
                parameter.value,
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
        if let Terminator::Try {
            operation, normal, ..
        } = &block.terminator
            && let Some(value) = operation.def()
        {
            insert_definition(
                &mut definitions,
                &mut sites,
                value,
                ValueDefinition::Instruction(InstructionLocation::Terminator { block: block_id }),
                DefinitionSite::Terminator {
                    block: block_id,
                    normal_edge: normal.id(),
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

fn verify_block_arguments(
    method: &MokaIRMethod,
    definitions: &BTreeMap<ValueId, DefinitionSite>,
    dominators: &BTreeMap<BlockId, BTreeSet<BlockId>>,
) -> VerificationResult {
    for &value in method.entry().arguments() {
        if !matches!(definitions.get(&value), Some(DefinitionSite::External)) {
            return Err(format!(
                "method-entry argument {value} is not an externally defined value"
            ));
        }
    }
    for (source, block) in method.blocks() {
        for successor in block.terminator.successors() {
            for &value in successor.arguments() {
                verify_use(
                    method,
                    value,
                    UseSite::EdgeArgument {
                        edge: successor.id(),
                        source,
                    },
                    definitions,
                    dominators,
                )?;
            }
        }
    }
    Ok(())
}

fn verify_instruction_uses(
    method: &MokaIRMethod,
    definitions: &BTreeMap<ValueId, DefinitionSite>,
    dominators: &BTreeMap<BlockId, BTreeSet<BlockId>>,
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
                )?;
            }
        }
        for value in block.terminator.local_uses() {
            verify_use(
                method,
                value,
                UseSite::Terminator { block: block_id },
                definitions,
                dominators,
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
            UseSite::Terminator { .. } | UseSite::EdgeArgument { .. } => true,
        },
        DefinitionSite::Terminator { block, .. } if block == use_block => {
            matches!(usage, UseSite::EdgeArgument { .. })
        }
        DefinitionSite::BlockEntry(block)
        | DefinitionSite::Operation { block, .. }
        | DefinitionSite::Terminator { block, .. } => dominators[&use_block].contains(&block),
    };
    if !dominates {
        return Err(format!(
            "definition of {value} at {definition:?} does not dominate use at {usage:?}"
        ));
    }
    if let DefinitionSite::Terminator { block, normal_edge } = definition {
        verify_edge_sensitive_use(method, value, usage, block, normal_edge)?;
    }
    Ok(())
}

fn verify_edge_sensitive_use(
    method: &MokaIRMethod,
    value: ValueId,
    usage: UseSite,
    definition_block: BlockId,
    normal_edge: EdgeId,
) -> VerificationResult {
    if let UseSite::EdgeArgument { edge, source } = usage
        && source == definition_block
    {
        if edge == normal_edge {
            return Ok(());
        }
        return Err(format!(
            "fallible result {value} is used as an argument on non-normal edge {edge}"
        ));
    }

    let reachable_without_normal = reachable_blocks(method, &BTreeSet::from([normal_edge]));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ir::{MokaIRMethod, Successor, control_flow::ControlTransfer},
        jvm::{code::Instruction, method::AccessFlags},
    };

    fn entry_loop() -> MokaIRMethod {
        let method = crate::tests::method(
            [
                (0, Instruction::ILoad0),
                (1, Instruction::IfEq(8.into())),
                (2, Instruction::ILoad0),
                (3, Instruction::IConst1),
                (4, Instruction::ISub),
                (5, Instruction::IStore0),
                (6, Instruction::Goto(0.into())),
                (8, Instruction::Return),
            ],
            "(I)V",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );
        MokaIRMethod::from_method(&method).unwrap()
    }

    fn method_with_fallible_result() -> MokaIRMethod {
        let method = crate::tests::method(
            [
                (0, Instruction::ALoad0),
                (
                    1,
                    Instruction::CheckCast("java/lang/String".parse().unwrap()),
                ),
                (2, Instruction::AReturn),
                (10, Instruction::AStore1),
                (11, Instruction::ALoad0),
                (12, Instruction::AReturn),
            ],
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            vec![crate::jvm::code::ExceptionTableEntry {
                covered_pc: 1.into()..2.into(),
                handler_pc: 10.into(),
                catch_type: None,
            }],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );
        MokaIRMethod::from_method(&method).unwrap()
    }

    fn verify_fallible_result_on_edge(
        method: &MokaIRMethod,
        edge_predicate: impl Fn(&super::super::Successor) -> bool,
    ) -> VerificationResult {
        let blocks = collect_blocks(method)?;
        let predecessors = verify_edges(method, &blocks)?;
        let dominators = compute_dominators(method, &blocks, &predecessors)?;
        let definitions = collect_definitions(method)?;
        let (&value, &DefinitionSite::Terminator { block, .. }) = definitions
            .sites
            .iter()
            .find(|(_, site)| matches!(site, DefinitionSite::Terminator { .. }))
            .unwrap()
        else {
            unreachable!("the fixture must contain a terminator definition")
        };
        let defining_block = method.block(block).unwrap();
        let edge = defining_block
            .terminator
            .successors()
            .find(|edge| edge_predicate(edge))
            .unwrap();
        verify_use(
            method,
            value,
            UseSite::EdgeArgument {
                edge: edge.id(),
                source: block,
            },
            &definitions.sites,
            &dominators,
        )
    }

    #[test]
    fn rejects_method_entry_argument_arity_mismatch() {
        let mut method = entry_loop();
        method.entry_mut().arguments.clear();

        assert!(
            verify(&method)
                .unwrap_err()
                .contains("method-entry argument count")
        );
    }

    #[test]
    fn rejects_successor_argument_arity_mismatch() {
        let mut method = entry_loop();
        let entry = method.entry_block();
        let successor = method
            .blocks_mut()
            .values_mut()
            .find_map(|block| {
                block
                    .terminator
                    .successor_mut(|successor| successor.block_target() == Some(entry))
            })
            .unwrap();
        let Successor::Block { arguments, .. } = successor else {
            panic!("the selected successor must target a block");
        };
        arguments.clear();

        assert!(
            verify(&method)
                .unwrap_err()
                .contains("supplies 0 arguments")
        );
    }

    #[test]
    fn rejects_duplicate_block_parameter_definition() {
        let mut method = entry_loop();
        let external = method.parameter_values()[0];
        let entry = method.entry_block();
        method.blocks_mut().get_mut(&entry).unwrap().parameters[0].value = external;

        assert!(
            verify(&method)
                .unwrap_err()
                .contains("multiple definitions")
        );
    }

    #[test]
    fn accepts_a_try_result_on_its_normal_edge() {
        let method = method_with_fallible_result();
        verify_fallible_result_on_edge(&method, |edge| {
            matches!(edge.transfer(), Some(ControlTransfer::Unconditional))
        })
        .unwrap();
    }

    #[test]
    fn rejects_a_try_result_on_an_exceptional_edge() {
        let method = method_with_fallible_result();
        let error = verify_fallible_result_on_edge(&method, |edge| {
            matches!(edge.transfer(), Some(ControlTransfer::Exception(_)))
        })
        .unwrap_err();
        assert!(error.contains("non-normal edge"), "{error}");
    }
}
