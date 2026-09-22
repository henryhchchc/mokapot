use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

#[derive(Clone)]
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
        let index = self.next_index;
        self.next_index += 1;

        let state = State {
            index,
            lowlink: index,
            on_stack: true,
        };
        self.state.insert(node, state);
        self.stack.push(node);

        for successor in self.adjacency[&node].clone() {
            if !self.state.contains_key(&successor) {
                self.strongconnect(successor);
                self.update_lowlink(node, self.state[&successor].lowlink);
            } else if self.state[&successor].on_stack {
                self.update_lowlink(node, self.state[&successor].index);
            }
        }

        let state = self.state[&node].clone();
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
