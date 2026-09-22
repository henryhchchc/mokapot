use std::{
    collections::{BTreeMap, HashMap},
    hash::{BuildHasher, Hash},
};

use super::JoinSemiLattice;

/// The solver's fact map, doubling as the pending worklist.
///
/// A location's fact is owned by exactly one map at a time: [`pop_one`] hands a
/// pending `(location, fact)` to the result map, and successors are joined back
/// into the worklist. Locations move rather than clone.
///
/// Implemented for [`BTreeMap`] (`L: Ord`) and [`HashMap`] (`L: Hash + Eq`).
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
