use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{BlockId, ValueId};

/// Phi candidates keyed by their provisional result value.
pub(crate) type PhiCandidates = BTreeMap<ValueId, Vec<(BlockId, ValueId)>>;

/// The result of simplifying a set of provisional phi nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SimplifiedPhis {
    /// Canonical replacements for eliminated phi results.
    pub(crate) substitutions: BTreeMap<ValueId, ValueId>,
    /// Phi candidates that represent genuine choices after rewriting.
    pub(crate) candidates: PhiCandidates,
}

/// An inconsistency found while simplifying provisional phi nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PhiSimplificationError {
    /// A reachable phi cycle has no value entering it from outside the cycle.
    #[error("reachable phi cycle containing {representative} has no external value")]
    ClosedCycle {
        /// The lowest-numbered result in the remaining canonical cycle.
        representative: ValueId,
    },
}

/// Eliminates trivial acyclic and cyclic phi candidates.
///
/// Inputs retain their caller-provided predecessor order. Eliminated results are
/// returned as fully canonical substitutions, and every retained input is
/// rewritten through those substitutions.
pub(crate) fn simplify_phis(
    mut candidates: PhiCandidates,
) -> Result<SimplifiedPhis, PhiSimplificationError> {
    let mut substitutions = BTreeMap::new();

    loop {
        rewrite_candidates(&mut candidates, &substitutions);

        let trivial = candidates.iter().find_map(|(&result, inputs)| {
            let external = inputs
                .iter()
                .map(|(_, value)| canonical(*value, &substitutions))
                .filter(|&value| value != result)
                .collect::<BTreeSet<_>>();
            (external.len() == 1).then(|| {
                (
                    result,
                    *external.first().expect("the set contains one value"),
                )
            })
        });
        if let Some((result, replacement)) = trivial {
            candidates.remove(&result);
            substitutions.insert(result, replacement);
            continue;
        }

        let components = strongly_connected_components(&candidates);
        let mut collapsed = false;
        for component in components {
            let external = component
                .iter()
                .flat_map(|result| {
                    candidates
                        .get(result)
                        .expect("SCC nodes are candidate results")
                })
                .map(|(_, value)| canonical(*value, &substitutions))
                .filter(|value| !component.contains(value))
                .collect::<BTreeSet<_>>();

            match external.len() {
                0 => {
                    return Err(PhiSimplificationError::ClosedCycle {
                        representative: *component
                            .first()
                            .expect("an SCC contains at least one result"),
                    });
                }
                1 => {
                    let replacement = *external.first().expect("the set contains one value");
                    for result in component {
                        candidates.remove(&result);
                        substitutions.insert(result, replacement);
                    }
                    collapsed = true;
                }
                _ => {}
            }
        }

        if !collapsed {
            break;
        }
    }

    rewrite_candidates(&mut candidates, &substitutions);
    let keys = substitutions.keys().copied().collect::<Vec<_>>();
    for value in keys {
        let replacement = canonical(value, &substitutions);
        substitutions.insert(value, replacement);
    }

    Ok(SimplifiedPhis {
        substitutions,
        candidates,
    })
}

fn canonical(mut value: ValueId, substitutions: &BTreeMap<ValueId, ValueId>) -> ValueId {
    while let Some(&replacement) = substitutions.get(&value) {
        debug_assert_ne!(value, replacement, "a substitution must make progress");
        value = replacement;
    }
    value
}

fn rewrite_candidates(candidates: &mut PhiCandidates, substitutions: &BTreeMap<ValueId, ValueId>) {
    for inputs in candidates.values_mut() {
        for (_, value) in inputs {
            *value = canonical(*value, substitutions);
        }
    }
}

fn strongly_connected_components(candidates: &PhiCandidates) -> Vec<BTreeSet<ValueId>> {
    let nodes = candidates.keys().copied().collect::<BTreeSet<_>>();
    let adjacency = candidates
        .iter()
        .map(|(&result, inputs)| {
            let dependencies = inputs
                .iter()
                .map(|(_, value)| *value)
                .filter(|value| nodes.contains(value))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            (result, dependencies)
        })
        .collect::<BTreeMap<_, _>>();

    let mut reverse = nodes
        .iter()
        .map(|&node| (node, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (&source, targets) in &adjacency {
        for target in targets {
            reverse
                .get_mut(target)
                .expect("a phi dependency is a candidate result")
                .push(source);
        }
    }

    let mut visited = BTreeSet::new();
    let mut finished = Vec::with_capacity(nodes.len());
    for &start in &nodes {
        if !visited.insert(start) {
            continue;
        }
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, next_index)) = stack.last_mut() {
            let neighbors = adjacency
                .get(node)
                .expect("every candidate result has an adjacency list");
            if let Some(&neighbor) = neighbors.get(*next_index) {
                *next_index += 1;
                if visited.insert(neighbor) {
                    stack.push((neighbor, 0));
                }
            } else {
                let (node, _) = stack.pop().expect("the DFS stack is not empty");
                finished.push(node);
            }
        }
    }

    visited.clear();
    let mut components = Vec::new();
    while let Some(start) = finished.pop() {
        if !visited.insert(start) {
            continue;
        }
        let mut component = BTreeSet::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            component.insert(node);
            for &neighbor in reverse
                .get(&node)
                .expect("every candidate result has a reverse adjacency list")
                .iter()
                .rev()
            {
                if visited.insert(neighbor) {
                    stack.push(neighbor);
                }
            }
        }
        components.push(component);
    }
    components.sort_by_key(|component| {
        *component
            .first()
            .expect("a strongly connected component is non-empty")
    });
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(index: u32) -> ValueId {
        ValueId::new(index)
    }

    fn block(index: u32) -> BlockId {
        BlockId::new(index)
    }

    #[test]
    fn canonicalizes_trivial_phi_chains() {
        let simplified = simplify_phis(BTreeMap::from([
            (value(1), vec![(block(0), value(100))]),
            (value(2), vec![(block(1), value(1))]),
        ]))
        .unwrap();

        assert!(simplified.candidates.is_empty());
        assert_eq!(
            simplified.substitutions,
            BTreeMap::from([(value(1), value(100)), (value(2), value(100))])
        );
    }

    #[test]
    fn ignores_self_inputs_when_simplifying() {
        let simplified = simplify_phis(BTreeMap::from([(
            value(1),
            vec![(block(0), value(100)), (block(1), value(1))],
        )]))
        .unwrap();

        assert!(simplified.candidates.is_empty());
        assert_eq!(simplified.substitutions[&value(1)], value(100));
    }

    #[test]
    fn collapses_mutually_recursive_trivial_phis() {
        let simplified = simplify_phis(BTreeMap::from([
            (value(1), vec![(block(0), value(100)), (block(1), value(2))]),
            (value(2), vec![(block(0), value(100)), (block(1), value(1))]),
        ]))
        .unwrap();

        assert!(simplified.candidates.is_empty());
        assert_eq!(simplified.substitutions[&value(1)], value(100));
        assert_eq!(simplified.substitutions[&value(2)], value(100));
    }

    #[test]
    fn rejects_a_closed_reachable_cycle() {
        let error = simplify_phis(BTreeMap::from([
            (value(1), vec![(block(0), value(2))]),
            (value(2), vec![(block(1), value(1))]),
        ]))
        .unwrap_err();

        assert_eq!(
            error,
            PhiSimplificationError::ClosedCycle {
                representative: value(2)
            }
        );
    }

    #[test]
    fn retains_a_cycle_with_distinct_external_values() {
        let candidates = BTreeMap::from([
            (value(1), vec![(block(0), value(100)), (block(1), value(2))]),
            (value(2), vec![(block(0), value(101)), (block(1), value(1))]),
        ]);
        let simplified = simplify_phis(candidates.clone()).unwrap();

        assert!(simplified.substitutions.is_empty());
        assert_eq!(simplified.candidates, candidates);
    }

    #[test]
    fn rewrites_inputs_of_retained_candidates_to_canonical_values() {
        let simplified = simplify_phis(BTreeMap::from([
            (value(1), vec![(block(0), value(100))]),
            (
                value(2),
                vec![
                    (block(0), value(1)),
                    (block(1), value(101)),
                    (block(2), value(2)),
                ],
            ),
        ]))
        .unwrap();

        assert_eq!(simplified.substitutions[&value(1)], value(100));
        assert_eq!(
            simplified.candidates[&value(2)],
            [
                (block(0), value(100)),
                (block(1), value(101)),
                (block(2), value(2))
            ]
        );
    }
}
