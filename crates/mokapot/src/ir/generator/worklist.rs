//! FIFO scheduling with at most one queued entry per identity.

use std::{
    collections::{HashSet, VecDeque},
    hash::Hash,
};

pub(super) struct Worklist<T> {
    pending: VecDeque<T>,
    queued: HashSet<T>,
}

impl<T> Default for Worklist<T> {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
            queued: HashSet::new(),
        }
    }
}

impl<T: Copy + Eq + Hash> Worklist<T> {
    pub(super) fn schedule(&mut self, value: T) {
        if self.queued.insert(value) {
            self.pending.push_back(value);
        }
    }

    pub(super) fn pop(&mut self) -> Option<T> {
        let value = self.pending.pop_front()?;
        self.queued.remove(&value);
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Worklist;

    #[test]
    fn coalesces_pending_entries_and_allows_rescheduling_after_pop() {
        let mut worklist = Worklist::default();
        worklist.schedule(1);
        worklist.schedule(2);
        worklist.schedule(1);
        assert_eq!(worklist.pop(), Some(1));
        worklist.schedule(1);
        assert_eq!(worklist.pop(), Some(2));
        assert_eq!(worklist.pop(), Some(1));
        assert_eq!(worklist.pop(), None);
    }
}
