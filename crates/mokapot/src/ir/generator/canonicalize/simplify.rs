use std::collections::{HashMap, HashSet};

use itertools::Itertools;

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
    pub(super) remaps: HashMap<ValueId, ValueId>,
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
    let mut working_remaps = HashMap::new();

    let retained = loop {
        if simplify_acyclic(&mut candidates, &mut working_remaps)
            || simplify_cyclic(&mut candidates, &mut working_remaps)
        {
            // Rewrite candidates after simplification
            candidates
                .values_mut()
                .flat_map(|it| it.inputs.iter_mut())
                .for_each(|it| *it = canonical(*it, &working_remaps));
        } else {
            // Terminate when no more candidates can be simplified
            break candidates.into_keys().collect();
        }
    };

    let remaps = working_remaps
        .keys()
        .copied()
        .map(|it| (it, canonical(it, &working_remaps)))
        .collect();

    SimplifiedParameters { remaps, retained }
}

fn simplify_acyclic(
    candidates: &mut HashMap<ValueId, ParameterCandidate>,
    working_remaps: &mut HashMap<ValueId, ValueId>,
) -> bool {
    let substitution = candidates.iter().find_map(|(&val, candidate)| {
        let external = candidate
            .inputs
            .iter()
            .map(|it| canonical(*it, &*working_remaps))
            .filter(|&it| it != val);
        external.unique().exactly_one().ok().map(|id| (val, id))
    });

    substitution
        .map(|(result, replacement)| {
            candidates.remove(&result);
            working_remaps.insert(result, replacement);
        })
        .is_some()
}

fn simplify_cyclic(
    candidates: &mut HashMap<ValueId, ParameterCandidate>,
    working_remaps: &mut HashMap<ValueId, ValueId>,
) -> bool {
    let components = strongly_connected_components(candidates);
    let mut collapsed = false;
    for scc in components {
        let mut external = scc
            .iter()
            .flat_map(|it| candidates[it].inputs.iter())
            .map(|value| canonical(*value, &*working_remaps))
            .filter(|value| !scc.contains(value))
            .unique();
        match (external.next(), external.next()) {
            (Some(replacement), None) => {
                for result in scc {
                    candidates.remove(&result);
                    working_remaps.insert(result, replacement);
                }
                collapsed = true;
            }
            (None, None) => {
                // A reachable parameter always takes an input from outside the cycle.
                panic!("reachable block parameters form a closed cycle");
            }
            _ => {}
        }
    }
    collapsed
}

fn canonical(mut value: ValueId, remaps: &HashMap<ValueId, ValueId>) -> ValueId {
    while let Some(&substitute) = remaps.get(&value) {
        debug_assert_ne!(value, substitute, "{value} is remapped to itself");
        value = substitute;
    }
    value
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
