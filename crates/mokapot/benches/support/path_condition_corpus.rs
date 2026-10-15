use std::{
    collections::{BTreeMap, HashSet},
    io,
};

use mokapot::ir::{ControlTransfer, MokaIRMethod};

pub fn predicate_limit(value: Option<&str>) -> io::Result<usize> {
    value.unwrap_or("16").parse().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid JDK_BENCH_MAX_PREDICATES: {error}"),
        )
    })
}

// Keep the metric identical to the JDK smoke tests: count occurrences across arms,
// rather than deduplicating predicates across the method.
pub fn reachable_predicate_count(method: &MokaIRMethod) -> usize {
    let mut pending = vec![method.entry.block];
    let mut visited = HashSet::new();
    let mut count = 0;
    while let Some(block) = pending.pop() {
        if !visited.insert(block) {
            continue;
        }
        let terminator = &method
            .block(block)
            .expect("successor blocks belong to the method")
            .terminator;
        for successor in terminator.successors() {
            pending.extend(successor.block_target());
            if let Some(ControlTransfer::Conditional(guard)) = successor.transfer() {
                count += guard.predicate_count();
            }
        }
    }
    count
}

#[derive(Default)]
pub struct PathCorpus {
    pub bins: BTreeMap<usize, Vec<MokaIRMethod>>,
    pub excluded: usize,
}

impl PathCorpus {
    pub fn insert(&mut self, method: MokaIRMethod, limit: usize) {
        let count = reachable_predicate_count(&method);
        if count <= limit {
            self.bins.entry(count).or_default().push(method);
        } else {
            self.excluded += 1;
        }
    }
}
