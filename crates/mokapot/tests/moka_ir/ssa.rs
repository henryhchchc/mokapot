#[cfg(integration_test)]
use std::collections::HashMap;

use mokapot::ir::BlockKind;
#[cfg(integration_test)]
use mokapot::ir::ValueId;

use super::*;

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn ssa_definitions_and_block_arguments_are_well_formed() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    let mut definitions = HashSet::new();
    let mut uses = HashSet::new();

    if let Some(value) = ir.this {
        definitions.insert(value);
    }
    definitions.extend(ir.parameters.clone());
    assert_eq!(
        ir.entry.arguments.len(),
        ir.block(ir.entry.block).unwrap().parameters.len()
    );
    uses.extend(ir.entry.arguments.clone());
    for (_, block) in reachable_blocks(&ir) {
        if let BlockKind::LandingPad { exception: value } = block.kind {
            definitions.insert(value);
        }
        for parameter in &block.parameters {
            assert!(definitions.insert(parameter.value));
        }
        for instruction in &block.operations {
            if let Some(value) = instruction.def() {
                assert!(definitions.insert(value));
            }
            uses.extend(instruction.uses());
        }
        if let Some(value) = block.terminator.def() {
            assert!(definitions.insert(value));
        }
        uses.extend(block.terminator.uses());
    }

    assert!(uses.iter().all(|it| definitions.contains(it)));
    assert!(definitions.iter().all(|it| ir.definition_of(*it).is_some()));
}

#[cfg(integration_test)]
#[derive(Debug, Clone, Copy)]
enum UseSite {
    /// An operation operand or a terminator-local value (returned/thrown, guard).
    Local { block: BlockId },
    /// An argument on an outgoing arm, tagged whether it is the normal arm.
    EdgeArgument { source: BlockId, normal: bool },
}

#[cfg(integration_test)]
impl UseSite {
    const fn block(self) -> BlockId {
        match self {
            Self::Local { block } => block,
            Self::EdgeArgument { source, .. } => source,
        }
    }
}

/// The definitions the fallible-result rule consults.
#[cfg(integration_test)]
struct Definitions {
    defined: HashSet<ValueId>,
    /// Values defined by a block terminator, keyed by that block.
    fallible: HashMap<ValueId, BlockId>,
}

#[cfg(integration_test)]
fn collect_definitions(ir: &MokaIRMethod) -> Definitions {
    let mut definitions = Definitions {
        defined: HashSet::new(),
        fallible: HashMap::new(),
    };
    if let Some(value) = ir.this_value() {
        definitions.defined.insert(value);
    }
    definitions
        .defined
        .extend(ir.parameter_values().iter().copied());
    for location in live_locations(ir) {
        match ir.instruction(location) {
            Some(InstructionRef::BlockParameter(parameter)) => {
                definitions.defined.insert(parameter.value);
            }
            Some(InstructionRef::Operation(operation)) => {
                definitions.defined.extend(operation.def());
            }
            Some(InstructionRef::Terminator(terminator)) => {
                if let Some(value) = terminator.def()
                    && let InstructionLocation::Terminator { block } = location
                {
                    definitions.defined.insert(value);
                    definitions.fallible.insert(value, block);
                }
            }
            None => {}
        }
        // The caught exception has no instruction location, so recover it from
        // the handler block the terminator location identifies.
        if let InstructionLocation::Terminator { block } = location
            && let Some(BlockKind::LandingPad { exception }) = ir.block(block).map(|it| it.kind)
        {
            definitions.defined.insert(exception);
        }
    }
    definitions
}

#[cfg(integration_test)]
fn block_uses(block_id: BlockId, block: &BasicBlock) -> Vec<(ValueId, UseSite)> {
    let mut uses = Vec::new();
    for operation in &block.operations {
        for value in operation.uses() {
            uses.push((value, UseSite::Local { block: block_id }));
        }
    }
    let terminator = &block.terminator;
    let normal_arm = match terminator {
        Terminator::Try { normal, .. } => Some(normal),
        _ => None,
    };
    let argument_values = terminator
        .successors()
        .flat_map(|successor| successor.arguments().iter().copied())
        .collect::<HashSet<_>>();
    for successor in terminator.successors() {
        let normal = normal_arm.is_some_and(|normal| std::ptr::eq(normal, successor));
        for &value in successor.arguments() {
            uses.push((
                value,
                UseSite::EdgeArgument {
                    source: block_id,
                    normal,
                },
            ));
        }
    }
    // Local uses (returned/thrown, guards, the tried operation) lack a public
    // accessor; subtracting arm arguments keeps the same block, which the rule needs.
    for value in terminator.uses() {
        if argument_values.contains(&value) {
            continue;
        }
        uses.push((value, UseSite::Local { block: block_id }));
    }
    uses
}

/// Blocks reachable from the entry while skipping the normal arm of `skipped`.
#[cfg(integration_test)]
fn reachable_skipping_normal(ir: &MokaIRMethod, skipped: BlockId) -> HashSet<BlockId> {
    let entry = ir.entry.block;
    let mut reachable = HashSet::from([entry]);
    let mut pending = VecDeque::from([entry]);
    while let Some(block_id) = pending.pop_front() {
        let block = ir
            .block(block_id)
            .expect("the reachable set only contains defined blocks");
        for successor in block.terminator.successors() {
            if block_id == skipped
                && let Terminator::Try { normal, .. } = &block.terminator
                && std::ptr::eq(normal, successor)
            {
                continue;
            }
            if let Some(target) = successor.block_target()
                && reachable.insert(target)
            {
                pending.push_back(target);
            }
        }
    }
    reachable
}

#[test]
#[cfg(integration_test)]
fn fallible_results_are_used_only_after_their_normal_arm() {
    for (label, ir) in corpus_ir() {
        let definitions = collect_definitions(&ir);
        for (block_id, block) in reachable_blocks(&ir) {
            for (value, usage) in block_uses(block_id, block) {
                assert!(
                    definitions.defined.contains(&value),
                    "{label}: {value} is used at {usage:?} but is not defined"
                );
                let Some(&definition_block) = definitions.fallible.get(&value) else {
                    continue;
                };
                if let UseSite::EdgeArgument { source, normal } = usage
                    && source == definition_block
                {
                    assert!(
                        normal,
                        "{label}: fallible result {value} is used as an argument on a non-normal edge"
                    );
                    continue;
                }
                assert!(
                    !reachable_skipping_normal(&ir, definition_block).contains(&usage.block()),
                    "{label}: fallible result {value} is visible at {usage:?} without taking its normal edge"
                );
            }
        }
    }
}
