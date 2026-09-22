/// A join semi-lattice over dataflow facts.
///
/// Facts are partially ordered, and every pair has a least upper bound, the
/// join (⊔), computed in place by [`join_assign`](Self::join_assign). The join
/// must be idempotent, commutative, and associative.
///
/// [`PartialOrd`] is the lattice ordering (⊑): `a <= b` means `a` is no more
/// informative than `b`, and `a ⊔ b` is the least upper bound. It may differ
/// from the type's natural ordering; a powerset lattice, for example, has
/// `{a} <= {a, b}`.
///
/// [`solve`](super::solve) terminates only if the lattice has finite height.
#[instability::unstable(feature = "fixed-point-analyses")]
pub trait JoinSemiLattice: PartialOrd {
    /// Joins `other` into `self` in place, reusing owned storage where
    /// practical.
    ///
    /// Returns `true` exactly when `self` changed, in which case it moved
    /// strictly upwards in the lattice ordering.
    fn join_assign(&mut self, other: Self) -> bool;
}

/// Lifts `T` with a bottom element `None` (⊥) that stands for "no information
/// yet", below every concrete fact: `None ⊔ x = x`, `Some(x) ⊔ Some(y) =
/// Some(x ⊔ y)`, and `None <= Some(_)`.
impl<T: JoinSemiLattice> JoinSemiLattice for Option<T> {
    fn join_assign(&mut self, other: Self) -> bool {
        match (self, other) {
            (_, None) => false,
            (slot @ None, Some(other)) => {
                *slot = Some(other);
                true
            }
            (Some(this), Some(other)) => this.join_assign(other),
        }
    }
}
