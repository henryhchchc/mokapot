use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

/// Tarjan's bookkeeping, threaded through the recursive DFS.
pub(super) struct Tarjan<N> {
    adjacency: HashMap<N, Vec<N>>,
    index: HashMap<N, usize>,
    lowlink: HashMap<N, usize>,
    on_stack: HashSet<N>,
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
            index: HashMap::new(),
            lowlink: HashMap::new(),
            on_stack: HashSet::new(),
            stack: Vec::new(),
            components: Vec::new(),
        }
    }

    pub fn scc(mut self) -> Vec<HashSet<N>> {
        let nodes = self.adjacency.keys().copied().collect::<HashSet<_>>();
        for start in nodes {
            if !self.index.contains_key(&start) {
                self.strongconnect(start);
            }
        }
        self.components
    }

    /// Tarjan's `strongconnect`: visits `v`, recursing into its undiscovered
    /// successors and closing `v`'s component when `v` turns out to be a root.
    fn strongconnect(&mut self, v: N) {
        // Number `v` and push it onto the stack.
        let index = self.index.len();
        self.index.insert(v, index);
        self.lowlink.insert(v, index);
        self.stack.push(v);
        self.on_stack.insert(v);

        // Consider the successors of `v`. Iterating by index keeps the borrow of
        // `self.adjacency` from living across the recursive call.
        for position in 0..self.adjacency[&v].len() {
            let successor = self.adjacency[&v][position];
            if !self.index.contains_key(&successor) {
                // `successor` has not been visited yet: recurse on it.
                self.strongconnect(successor);
                let lowlink = self.lowlink[&v].min(self.lowlink[&successor]);
                self.lowlink.insert(v, lowlink);
            } else if self.on_stack.contains(&successor) {
                // `successor` is on the stack and hence in the current component.
                let lowlink = self.lowlink[&v].min(self.index[&successor]);
                self.lowlink.insert(v, lowlink);
            }
        }

        // If `v` is a root node, pop the stack and generate a component.
        if self.lowlink[&v] == self.index[&v] {
            let mut component = HashSet::new();
            loop {
                let member = self.stack.pop().expect("the stack is not empty");
                self.on_stack.remove(&member);
                component.insert(member);
                if member == v {
                    break;
                }
            }
            self.components.push(component);
        }
    }
}
