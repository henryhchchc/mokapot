use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use super::{ParameterInputs, SimplifiedParameters, Substitutions, tarjan::Tarjan};
use crate::ir::ValueId;

type InputsByParameter = HashMap<ValueId, ParameterInputs>;

impl SimplifiedParameters {
    /// Eliminates trivial acyclic and cyclic block parameters.
    ///
    /// Preserves each input's caller-provided edge order. Eliminated results are
    /// returned as fully canonical substitutions.
    pub(super) fn for_block_inputs(mut inputs: InputsByParameter) -> SimplifiedParameters {
        let mut substitutions = HashMap::new();

        let retained = loop {
            if eliminate_acyclic(&mut inputs, &mut substitutions)
                || eliminate_cyclic(&mut inputs, &mut substitutions)
            {
                inputs
                    .values_mut()
                    .flat_map(|it| it.arguments.iter_mut())
                    .for_each(|it| *it = canonical(*it, &substitutions));
            } else {
                break inputs.into_keys().collect();
            }
        };

        let substitutions = substitutions
            .keys()
            .copied()
            .map(|it| (it, canonical(it, &substitutions)))
            .collect();

        SimplifiedParameters {
            substitutions,
            retained,
        }
    }
}

fn eliminate_acyclic(inputs: &mut InputsByParameter, substitutions: &mut Substitutions) -> bool {
    let substitution = inputs.iter().find_map(|(&val, input)| {
        let external = input
            .arguments
            .iter()
            .map(|it| canonical(*it, substitutions))
            .filter(|&it| it != val);
        external.unique().exactly_one().ok().map(|id| (val, id))
    });

    substitution
        .map(|(result, replacement)| {
            inputs.remove(&result);
            substitutions.insert(result, replacement);
        })
        .is_some()
}

fn eliminate_cyclic(inputs: &mut InputsByParameter, substitutions: &mut Substitutions) -> bool {
    let components = strongly_connected_components(inputs);
    let mut collapsed = false;
    for scc in components {
        let mut external = scc
            .iter()
            .flat_map(|it| inputs[it].arguments.iter())
            .map(|value| canonical(*value, substitutions))
            .filter(|value| !scc.contains(value))
            .unique();
        match (external.next(), external.next()) {
            (Some(replacement), None) => {
                for result in scc {
                    inputs.remove(&result);
                    substitutions.insert(result, replacement);
                }
                collapsed = true;
            }
            (None, None) => {
                panic!("reachable block parameters form a closed cycle");
            }
            _ => {}
        }
    }
    collapsed
}

fn canonical(mut value: ValueId, substitutions: &Substitutions) -> ValueId {
    while let Some(&substitute) = substitutions.get(&value) {
        debug_assert_ne!(value, substitute, "{value} is remapped to itself");
        value = substitute;
    }
    value
}

fn strongly_connected_components(inputs: &InputsByParameter) -> Vec<HashSet<ValueId>> {
    let adjacency = inputs
        .iter()
        .map(|(&value, input)| {
            let dependencies = input
                .arguments
                .iter()
                .copied()
                .filter(|it| inputs.contains_key(it))
                .unique()
                .collect();
            (value, dependencies)
        })
        .collect();
    Tarjan::new(adjacency).scc()
}
