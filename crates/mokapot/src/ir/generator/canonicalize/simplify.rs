use std::collections::{HashMap, HashSet};

use crate::ir::ValueId;

/// A block parameter and the arguments supplied by all incoming edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParameterCandidate {
    pub(super) inputs: Vec<ValueId>,
}

/// The result of simplifying provisional block parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SimplifiedParameters {
    /// Canonical replacements for eliminated parameter results.
    pub(super) substitutions: HashMap<ValueId, ValueId>,
    /// Results of parameters that represent genuine choices after rewriting.
    pub(super) retained: HashSet<ValueId>,
}

/// Eliminates trivial acyclic and cyclic block parameters.
///
/// Inputs retain their caller-provided edge order. Eliminated results are
/// returned as fully canonical substitutions, and every retained input is
/// rewritten through those substitutions.
pub(super) fn simplify_parameters(
    mut candidates: HashMap<ValueId, ParameterCandidate>,
) -> SimplifiedParameters {
    let mut substitutions = HashMap::new();

    loop {
        rewrite_candidates(&mut candidates, &substitutions);

        let trivial = candidates.iter().find_map(|(&result, candidate)| {
            let external = candidate
                .inputs
                .iter()
                .map(|value| canonical(*value, &substitutions))
                .filter(|&value| value != result)
                .collect::<HashSet<_>>();
            (external.len() == 1).then(|| {
                (
                    result,
                    *external.iter().next().expect("the set contains one value"),
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
                        .inputs
                        .iter()
                })
                .map(|value| canonical(*value, &substitutions))
                .filter(|value| !component.contains(value))
                .collect::<HashSet<_>>();

            match external.len() {
                0 => {
                    debug_assert!(false, "reachable block parameters form a closed cycle");
                }
                1 => {
                    let replacement = *external.iter().next().expect("the set contains one value");
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

    SimplifiedParameters {
        substitutions,
        retained: candidates.into_keys().collect(),
    }
}

fn canonical(mut value: ValueId, substitutions: &HashMap<ValueId, ValueId>) -> ValueId {
    while let Some(&replacement) = substitutions.get(&value) {
        debug_assert_ne!(value, replacement, "a substitution must make progress");
        value = replacement;
    }
    value
}

fn rewrite_candidates(
    candidates: &mut HashMap<ValueId, ParameterCandidate>,
    substitutions: &HashMap<ValueId, ValueId>,
) {
    for candidate in candidates.values_mut() {
        for value in &mut candidate.inputs {
            *value = canonical(*value, substitutions);
        }
    }
}

fn strongly_connected_components(
    candidates: &HashMap<ValueId, ParameterCandidate>,
) -> Vec<HashSet<ValueId>> {
    let nodes = candidates.keys().copied().collect::<HashSet<_>>();
    let adjacency = candidates
        .iter()
        .map(|(&result, candidate)| {
            let dependencies = candidate
                .inputs
                .iter()
                .copied()
                .filter(|value| nodes.contains(value))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            (result, dependencies)
        })
        .collect::<HashMap<_, _>>();

    let mut reverse = nodes
        .iter()
        .map(|&node| (node, Vec::new()))
        .collect::<HashMap<_, _>>();
    for (&source, targets) in &adjacency {
        for target in targets {
            reverse
                .get_mut(target)
                .expect("a parameter dependency is a candidate result")
                .push(source);
        }
    }

    let mut visited = HashSet::new();
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
        let mut component = HashSet::new();
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
    components
}
