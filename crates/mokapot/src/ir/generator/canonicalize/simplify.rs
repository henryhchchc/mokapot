use std::collections::{HashMap, HashSet};

use itertools::Itertools;

use super::{ParameterInputs, SimplifiedParameters, Substitutions, tarjan::Tarjan};
use crate::ir::{ValueId, generator::worklist::Worklist};

type InputsByParameter = HashMap<ValueId, ParameterInputs>;

impl SimplifiedParameters {
    /// Eliminates trivial acyclic and cyclic block parameters.
    ///
    /// Preserves each input's caller-provided edge order. Eliminated results are
    /// returned as fully canonical substitutions.
    pub(super) fn for_block_inputs(inputs: InputsByParameter) -> SimplifiedParameters {
        let mut solver = ParameterSolver::new(inputs);
        loop {
            solver.eliminate_acyclic();
            if solver.inputs.is_empty() || !solver.eliminate_cyclic() {
                break;
            }
        }

        let eliminated = solver.substitutions.keys().copied().collect::<Vec<_>>();
        let substitutions = eliminated
            .into_iter()
            .map(|value| (value, canonical(value, &mut solver.substitutions)))
            .collect();
        SimplifiedParameters {
            substitutions,
            retained: solver.inputs.into_keys().collect(),
        }
    }
}

struct ParameterSolver {
    inputs: InputsByParameter,
    substitutions: Substitutions,
    dependents: HashMap<ValueId, HashSet<ValueId>>,
    worklist: Worklist<ValueId>,
}

impl ParameterSolver {
    fn new(inputs: InputsByParameter) -> Self {
        let mut dependents: HashMap<ValueId, HashSet<ValueId>> = HashMap::new();
        for (&parameter, input) in &inputs {
            for &argument in &input.arguments {
                dependents.entry(argument).or_default().insert(parameter);
            }
        }
        let mut worklist = Worklist::default();
        for &parameter in inputs.keys() {
            worklist.schedule(parameter);
        }
        Self {
            inputs,
            substitutions: HashMap::new(),
            dependents,
            worklist,
        }
    }

    fn eliminate_acyclic(&mut self) {
        while let Some(parameter) = self.worklist.pop() {
            let Some(input) = self.inputs.get(&parameter) else {
                continue;
            };
            let mut external = input
                .arguments
                .iter()
                .map(|&value| canonical(value, &mut self.substitutions))
                .filter(|&value| value != parameter)
                .unique();
            if let (Some(replacement), None) = (external.next(), external.next()) {
                self.substitute(parameter, replacement);
            }
        }
    }

    fn eliminate_cyclic(&mut self) -> bool {
        let components = strongly_connected_components(&self.inputs, &mut self.substitutions);
        let mut collapsed = false;
        for component in components {
            let mut external = component
                .iter()
                .flat_map(|value| &self.inputs[value].arguments)
                .map(|&value| canonical(value, &mut self.substitutions))
                .filter(|value| !component.contains(value))
                .unique();
            match (external.next(), external.next()) {
                (Some(replacement), None) => {
                    for parameter in component {
                        self.substitute(parameter, replacement);
                    }
                    collapsed = true;
                }
                (None, None) => panic!("reachable block parameters form a closed cycle"),
                _ => {}
            }
        }
        collapsed
    }

    fn substitute(&mut self, parameter: ValueId, replacement: ValueId) {
        self.inputs.remove(&parameter);
        self.substitutions.insert(parameter, replacement);

        let mut affected = self.dependents.remove(&parameter).unwrap_or_default();
        affected.retain(|dependent| self.inputs.contains_key(dependent));
        for &dependent in &affected {
            self.worklist.schedule(dependent);
        }
        // Keep indirect users attached to the current representative. Merging
        // the smaller set into the larger bounds total movement of each user.
        let users = self.dependents.entry(replacement).or_default();
        if affected.len() > users.len() {
            std::mem::swap(users, &mut affected);
        }
        users.extend(affected);
    }
}

fn canonical(value: ValueId, substitutions: &mut Substitutions) -> ValueId {
    let mut root = value;
    while let Some(&next) = substitutions.get(&root) {
        debug_assert_ne!(root, next, "{root} is remapped to itself");
        root = next;
    }
    let mut current = value;
    while let Some(&next) = substitutions.get(&current) {
        substitutions.insert(current, root);
        current = next;
    }
    root
}

fn strongly_connected_components(
    inputs: &InputsByParameter,
    substitutions: &mut Substitutions,
) -> Vec<HashSet<ValueId>> {
    let adjacency = inputs
        .iter()
        .map(|(&value, input)| {
            let dependencies = input
                .arguments
                .iter()
                .map(|&argument| canonical(argument, substitutions))
                .filter(|dependency| inputs.contains_key(dependency))
                .unique()
                .collect();
            (value, dependencies)
        })
        .collect();
    Tarjan::new(adjacency).scc()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ParameterInputs, SimplifiedParameters};
    use crate::ir::test::prelude::{ValueId, ids};

    fn input(arguments: impl IntoIterator<Item = ValueId>) -> ParameterInputs {
        ParameterInputs {
            arguments: arguments.into_iter().collect(),
        }
    }

    #[test]
    fn collapses_a_long_forwarding_chain() {
        const LENGTH: usize = if cfg!(miri) { 256 } else { 10_000 };
        let values: [ValueId; LENGTH + 1] = ids(0);
        let source = values[0];
        let inputs = values
            .windows(2)
            .map(|pair| (pair[1], input([pair[0]])))
            .collect();

        let simplified = SimplifiedParameters::for_block_inputs(inputs);
        assert!(simplified.retained.is_empty());
        assert_eq!(simplified.substitutions.len(), LENGTH);
        let mut values = simplified.substitutions.values();
        assert!(values.all(|&value| value == source));
    }

    #[test]
    fn revisits_indirect_dependents_after_a_substitution() {
        let [source, first, second, dependent] = ids(0);
        let inputs = HashMap::from([
            (first, input([source])),
            (second, input([first])),
            (dependent, input([second, source])),
        ]);

        let simplified = SimplifiedParameters::for_block_inputs(inputs);
        assert_eq!(simplified.substitutions[&first], source);
        assert_eq!(simplified.substitutions[&second], source);
        assert_eq!(simplified.substitutions[&dependent], source);
        assert!(simplified.retained.is_empty());
    }

    #[test]
    fn collapses_a_cycle_and_then_its_dependent() {
        let [source, first, second, dependent] = ids(0);
        let inputs = HashMap::from([
            (first, input([second, source])),
            (second, input([first, source])),
            (dependent, input([first, second])),
        ]);

        let simplified = SimplifiedParameters::for_block_inputs(inputs);
        assert!(simplified.retained.is_empty());
        let mut values = simplified.substitutions.values();
        assert!(values.all(|&value| value == source));
    }
}
