use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    hash::{BuildHasher, Hash},
};

use super::JoinSemiLattice;

/// The solver's fact map, doubling as the pending worklist.
///
/// A location's fact is owned by exactly one map at a time: [`pop_one`] hands a
/// pending `(location, fact)` to the result map, and successors are joined back
/// into the worklist. Facts move rather than clone.
///
/// Implemented for [`BTreeMap`] (`L: Ord`), [`HashMap`] (`L: Hash + Eq`), and
/// [`QueuedFactsMap`] (`L: Hash + Eq + Clone`).
///
/// [`pop_one`]: FactsMap::pop_one
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait FactsMap<L, F>: Default {
    /// Joins `fact` into the fact stored at `location`, inserting it directly
    /// when absent. The join is in place, so no [`Clone`] is required.
    ///
    /// # Returns
    ///
    /// The stored location and fact if the stored fact changed, else `None`.
    fn insert_or_join(&mut self, location: L, fact: F) -> Option<(&L, &F)>
    where
        F: JoinSemiLattice;

    /// Removes and returns an arbitrary `(location, fact)` entry, or `None`.
    fn pop_one(&mut self) -> Option<(L, F)>;
}

impl<L, F> FactsMap<L, F> for BTreeMap<L, F>
where
    L: Ord,
{
    fn insert_or_join(&mut self, location: L, fact: F) -> Option<(&L, &F)>
    where
        F: JoinSemiLattice,
    {
        use std::collections::btree_map::Entry;
        let entry = match self.entry(location) {
            Entry::Vacant(entry) => entry.insert_entry(fact),
            Entry::Occupied(mut entry) => {
                if !entry.get_mut().join_assign(fact) {
                    return None;
                }
                entry
            }
        };
        // SAFETY: `entry` borrows `self`, so its key and value outlive the
        // returned references; the local binding only obscures that lifetime.
        Some(unsafe {
            (
                std::mem::transmute::<&L, &L>(entry.key()),
                std::mem::transmute::<&F, &F>(entry.get()),
            )
        })
    }

    fn pop_one(&mut self) -> Option<(L, F)> {
        self.pop_first()
    }
}

impl<L, F, S> FactsMap<L, F> for HashMap<L, F, S>
where
    L: Hash + Eq,
    S: BuildHasher + Default,
{
    fn insert_or_join(&mut self, location: L, fact: F) -> Option<(&L, &F)>
    where
        F: JoinSemiLattice,
    {
        use std::collections::hash_map::Entry;
        let entry = match self.entry(location) {
            Entry::Vacant(entry) => entry.insert_entry(fact),
            Entry::Occupied(mut entry) => {
                if !entry.get_mut().join_assign(fact) {
                    return None;
                }
                entry
            }
        };
        // SAFETY: `entry` borrows `self`, so its key and value outlive the
        // returned references; the local binding only obscures that lifetime.
        Some(unsafe {
            (
                std::mem::transmute::<&L, &L>(entry.key()),
                std::mem::transmute::<&F, &F>(entry.get()),
            )
        })
    }

    fn pop_one(&mut self) -> Option<(L, F)> {
        let location = self.keys().next()?;
        // SAFETY: `remove_entry` searches with `location` before it moves the
        // matching entry and never reads the key afterwards, so disassociating
        // its lifetime from the map is sound for this call.
        let location = unsafe { std::mem::transmute::<&L, &L>(location) };
        self.remove_entry(location)
    }
}

/// A [`FactsMap`] that dequeues in first-enqueued order.
///
/// An unordered map picks an arbitrary key, making the work spent reaching the
/// fixed point depend on iteration order; the queue costs a clone per location.
#[derive(Debug)]
#[instability::unstable(feature = "fixed-point-analyses")]
pub struct QueuedFactsMap<L, F> {
    entries: HashMap<L, F>,
    order: VecDeque<L>,
}

impl<L, F> Default for QueuedFactsMap<L, F> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

impl<L, F> FactsMap<L, F> for QueuedFactsMap<L, F>
where
    L: Hash + Eq + Clone,
{
    fn insert_or_join(&mut self, location: L, fact: F) -> Option<(&L, &F)>
    where
        F: JoinSemiLattice,
    {
        use std::collections::hash_map::Entry;
        let entry = match self.entries.entry(location) {
            Entry::Vacant(entry) => {
                self.order.push_back(entry.key().clone());
                entry.insert_entry(fact)
            }
            Entry::Occupied(mut entry) => {
                if !entry.get_mut().join_assign(fact) {
                    return None;
                }
                entry
            }
        };
        // SAFETY: `entry` borrows `self.entries`, so its key and value outlive
        // the returned references.
        Some(unsafe {
            (
                std::mem::transmute::<&L, &L>(entry.key()),
                std::mem::transmute::<&F, &F>(entry.get()),
            )
        })
    }

    fn pop_one(&mut self) -> Option<(L, F)> {
        while let Some(location) = self.order.pop_front() {
            if let Some(fact) = self.entries.remove(&location) {
                return Some((location, fact));
            }
        }
        None
    }
}

impl<L, F> IntoIterator for QueuedFactsMap<L, F> {
    type IntoIter = std::collections::hash_map::IntoIter<L, F>;
    type Item = (L, F);

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test::prelude::TestSet;
    use std::iter;

    #[test]
    fn maps_insert_join_coalesce_and_pop_exact_entries() {
        assert_facts_map::<BTreeMap<u8, TestSet>>();
        assert_facts_map::<HashMap<u8, TestSet>>();
        assert_facts_map::<QueuedFactsMap<u8, TestSet>>();
    }

    fn assert_facts_map<M>()
    where
        M: FactsMap<u8, TestSet>,
    {
        let mut map = M::default();
        let fact = map.insert_or_join(2, TestSet::from([1])).unwrap().1;
        assert_eq!(fact, &TestSet::from([1]));

        assert!(map.insert_or_join(2, TestSet::from([1])).is_none());

        let fact = map.insert_or_join(2, TestSet::from([2])).unwrap().1;
        assert_eq!(fact, &TestSet::from([1, 2]));

        let fact = map.insert_or_join(1, TestSet::default()).unwrap().1;
        assert_eq!(fact, &TestSet::default());

        let popped: Vec<_> = iter::from_fn(|| map.pop_one()).collect();
        assert_eq!(popped.len(), 2);
        let popped: BTreeMap<_, _> = popped.into_iter().collect();
        let expected = BTreeMap::from([(1, TestSet::default()), (2, TestSet::from([1, 2]))]);
        assert_eq!(popped, expected);
    }

    #[test]
    fn ordered_maps_pop_in_key_or_insertion_order() {
        let mut ordered = BTreeMap::default();
        ordered.insert_or_join(2, TestSet::default());
        ordered.insert_or_join(1, TestSet::default());
        assert_eq!(ordered.pop_one().unwrap().0, 1);

        let mut queued = QueuedFactsMap::default();
        queued.insert_or_join(2, TestSet::default());
        queued.insert_or_join(1, TestSet::default());
        assert_eq!(queued.pop_one().unwrap().0, 2);
        assert_eq!(queued.pop_one().unwrap().0, 1);
    }
}
