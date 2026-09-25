use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

struct State {
    index: usize,
    lowlink: usize,
    on_stack: bool,
}

pub(super) struct Tarjan<N> {
    adjacency: HashMap<N, Vec<N>>,
    next_index: usize,
    state: HashMap<N, State>,
    stack: Vec<N>,
    components: Vec<HashSet<N>>,
}

impl<N> Tarjan<N>
where
    N: Hash + Eq + Copy,
{
    pub fn new(adjacency: HashMap<N, Vec<N>>) -> Self {
        Self {
            adjacency,
            next_index: 0,
            state: HashMap::new(),
            stack: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn scc(mut self) -> Vec<HashSet<N>> {
        let nodes = self.adjacency.keys().copied().collect::<Vec<_>>();

        for node in nodes {
            if !self.state.contains_key(&node) {
                self.strongconnect(node);
            }
        }

        self.components
    }

    fn strongconnect(&mut self, node: N) {
        self.enter(node);
        let mut pending = vec![(node, 0)];

        while let Some(&(current, next_index)) = pending.last() {
            if let Some(&successor) = self.adjacency[&current].get(next_index) {
                pending.last_mut().expect("the current node is pending").1 += 1;
                if !self.state.contains_key(&successor) {
                    self.enter(successor);
                    pending.push((successor, 0));
                } else if self.state[&successor].on_stack {
                    self.update_lowlink(current, self.state[&successor].index);
                }
            } else {
                self.finish(current);
                pending.pop();
                if let Some(&(parent, _)) = pending.last() {
                    self.update_lowlink(parent, self.state[&current].lowlink);
                }
            }
        }
    }

    fn enter(&mut self, node: N) {
        let index = self.next_index;
        self.next_index += 1;

        let state = State {
            index,
            lowlink: index,
            on_stack: true,
        };
        self.state.insert(node, state);
        self.stack.push(node);
    }

    fn finish(&mut self, node: N) {
        let state = &self.state[&node];
        if state.lowlink == state.index {
            let mut component = HashSet::new();
            loop {
                let member = self.stack.pop().expect("node must be on the stack");
                let state = self.state.get_mut(&member).expect("it's in the state");
                state.on_stack = false;
                component.insert(member);

                if member == node {
                    break;
                }
            }
            self.components.push(component);
        }
    }

    fn update_lowlink(&mut self, node: N, candidate: usize) {
        let state = self.state.get_mut(&node).unwrap();
        state.lowlink = state.lowlink.min(candidate);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::Tarjan;

    #[test]
    fn finds_a_deep_cycle_without_recursing() {
        const LENGTH: usize = if cfg!(miri) { 256 } else { 20_000 };
        let adjacency = (0..LENGTH)
            .map(|node| (node, vec![(node + 1) % LENGTH]))
            .collect::<HashMap<_, _>>();

        let components = Tarjan::new(adjacency).scc();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), LENGTH);
    }

    #[test]
    fn separates_a_cycle_from_its_parent_and_tail() {
        let adjacency = HashMap::from([(0, vec![1]), (1, vec![2]), (2, vec![1, 3]), (3, vec![])]);

        let components = Tarjan::new(adjacency).scc();
        assert_eq!(components.len(), 3);
        assert!(components.contains(&[0].into()));
        assert!(components.contains(&[1, 2].into()));
        assert!(components.contains(&[3].into()));
    }
}
