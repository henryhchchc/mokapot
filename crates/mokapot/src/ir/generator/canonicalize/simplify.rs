use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use crate::ir::{ValueId, generator::canonicalize::tarjan::Tarjan};

/// A block parameter and the arguments supplied by all incoming edges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParameterCandidate {
    pub(super) inputs: Vec<ValueId>,
}

type CandidateMap = HashMap<ValueId, ParameterCandidate>;
type Substitutions = HashMap<ValueId, ValueId>;

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
pub(super) fn simplify_parameters(mut candidates: CandidateMap) -> SimplifiedParameters {
    let mut remaps = HashMap::new();

    let retained = loop {
        if simplify_acyclic(&mut candidates, &mut remaps)
            || simplify_cyclic(&mut candidates, &mut remaps)
        {
            // Rewrite candidates after simplification
            candidates
                .values_mut()
                .flat_map(|it| it.inputs.iter_mut())
                .for_each(|it| *it = canonical(*it, &remaps));
        } else {
            // Terminate when no more candidates can be simplified
            break candidates.into_keys().collect();
        }
    };

    let remaps = remaps
        .keys()
        .copied()
        .map(|it| (it, canonical(it, &remaps)))
        .collect();

    SimplifiedParameters { remaps, retained }
}

fn simplify_acyclic(candidates: &mut CandidateMap, remaps: &mut Substitutions) -> bool {
    let substitution = candidates.iter().find_map(|(&val, candidate)| {
        let external = candidate
            .inputs
            .iter()
            .map(|it| canonical(*it, &*remaps))
            .filter(|&it| it != val);
        external.unique().exactly_one().ok().map(|id| (val, id))
    });

    substitution
        .map(|(result, replacement)| {
            candidates.remove(&result);
            remaps.insert(result, replacement);
        })
        .is_some()
}

fn simplify_cyclic(candidates: &mut CandidateMap, remaps: &mut Substitutions) -> bool {
    let components = strongly_connected_components(candidates);
    let mut collapsed = false;
    for scc in components {
        let mut external = scc
            .iter()
            .flat_map(|it| candidates[it].inputs.iter())
            .map(|value| canonical(*value, &*remaps))
            .filter(|value| !scc.contains(value))
            .unique();
        match (external.next(), external.next()) {
            (Some(replacement), None) => {
                for result in scc {
                    candidates.remove(&result);
                    remaps.insert(result, replacement);
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

fn canonical(mut value: ValueId, remaps: &Substitutions) -> ValueId {
    while let Some(&substitute) = remaps.get(&value) {
        debug_assert_ne!(value, substitute, "{value} is remapped to itself");
        value = substitute;
    }
    value
}

fn strongly_connected_components(candidates: &CandidateMap) -> Vec<HashSet<ValueId>> {
    let adjacency = candidates
        .iter()
        .map(|(&value, candidate)| {
            let dependencies = candidate
                .inputs
                .iter()
                .copied()
                .filter(|it| candidates.contains_key(it))
                .unique()
                .collect();
            (value, dependencies)
        })
        .collect();
    Tarjan::new(adjacency).scc()
}
