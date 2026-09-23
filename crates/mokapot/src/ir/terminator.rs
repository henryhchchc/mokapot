use std::{
    cmp,
    collections::{HashSet, hash_set},
    convert::Infallible,
    fmt::{self, Display},
    hash::{Hash, Hasher},
    slice,
};

use itertools::Itertools;

use super::{
    BlockId, Operation, ValueId,
    expression::{BooleanVariable, Predicate},
};
use crate::{
    intrinsics::{HashUnordered, hashset_partial_order},
    jvm::references::ClassRef,
};

/// One outgoing arm of a terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Successor {
    /// Continues execution in a basic block.
    Block {
        /// The destination block.
        target: BlockId,
        /// Values supplied to the target block's parameters.
        arguments: Vec<ValueId>,
        /// The state transfer associated with this arm.
        transfer: ControlTransfer,
    },
    /// Propagates an exception out of the method.
    Unwind,
}

impl Successor {
    /// Returns the target block, or `None` when this arm exits by unwinding.
    #[must_use]
    pub const fn block_target(&self) -> Option<BlockId> {
        match self {
            Self::Block { target, .. } => Some(*target),
            Self::Unwind => None,
        }
    }
    /// Returns the values supplied to the target block's parameters.
    #[must_use]
    pub fn arguments(&self) -> &[ValueId] {
        match self {
            Self::Block { arguments, .. } => arguments,
            Self::Unwind => &[],
        }
    }
    /// Returns the state transfer associated with a block arm.
    ///
    /// Unwind arms have no block transfer.
    #[must_use]
    pub const fn transfer(&self) -> Option<&ControlTransfer> {
        match self {
            Self::Block { transfer, .. } => Some(transfer),
            Self::Unwind => None,
        }
    }
}

/// A structurally valid control-flow operation ending a basic block.
///
/// The outgoing arm type is a parameter defaulting to [`Successor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator<Arm = Successor> {
    /// Transfers control to one successor.
    Goto {
        /// The sole continuation.
        target: Arm,
    },
    /// Selects one of two guarded successors.
    Branch {
        /// The arm selected when the condition holds.
        taken: Arm,
        /// The arm selected when the condition does not hold.
        otherwise: Arm,
    },
    /// Selects an arm by matching a value, or takes the default arm.
    ///
    /// Each case arm carries a conditional guard matching the selected value
    /// against its case key, and the default arm carries the negation of every
    /// case.
    Switch {
        /// Ordered case arms.
        cases: Vec<Arm>,
        /// The arm selected when no key matches.
        default: Arm,
    },
    /// Selects the normal or an exceptional outcome of a fallible operation.
    Try {
        /// The operation attempted by this terminator.
        operation: Operation,
        /// The continuation taken after successful completion.
        normal: Arm,
        /// Ordered exceptional outcomes.
        exceptional: Vec<Arm>,
    },
    /// Attempts to complete the method normally, but may fail while exiting.
    Return {
        /// The returned value, or `None` for a void return.
        value: Option<ValueId>,
        /// Ordered failures possible while completing the return.
        exceptional: Vec<Arm>,
    },
    /// Throws an exception to an ordered handler or out of the method.
    Throw {
        /// The thrown value.
        value: ValueId,
        /// Ordered handlers followed by an optional unwind exit.
        exceptional: Vec<Arm>,
    },
}

impl<Arm> Terminator<Arm> {
    /// Maps every outgoing arm while preserving the terminator's structure.
    pub(super) fn map_arms<MappedArm>(
        self,
        mut map: impl FnMut(Arm) -> MappedArm,
    ) -> Terminator<MappedArm> {
        let Ok(term) = self.try_map_arms(|arm| Ok::<_, Infallible>(map(arm)));
        term
    }

    /// Maps every outgoing arm fallibly while preserving the terminator's
    /// structure.
    pub(super) fn try_map_arms<MappedArm, E>(
        self,
        mut map: impl FnMut(Arm) -> Result<MappedArm, E>,
    ) -> Result<Terminator<MappedArm>, E> {
        Ok(match self {
            Self::Goto { target } => Terminator::Goto {
                target: map(target)?,
            },
            Self::Branch { taken, otherwise } => Terminator::Branch {
                taken: map(taken)?,
                otherwise: map(otherwise)?,
            },
            Self::Switch { cases, default } => Terminator::Switch {
                cases: cases.into_iter().map(&mut map).collect::<Result<_, _>>()?,
                default: map(default)?,
            },
            Self::Try {
                operation,
                normal,
                exceptional,
            } => Terminator::Try {
                operation,
                normal: map(normal)?,
                exceptional: exceptional
                    .into_iter()
                    .map(&mut map)
                    .collect::<Result<_, _>>()?,
            },
            Self::Return { value, exceptional } => Terminator::Return {
                value,
                exceptional: exceptional
                    .into_iter()
                    .map(&mut map)
                    .collect::<Result<_, _>>()?,
            },
            Self::Throw { value, exceptional } => Terminator::Throw {
                value,
                exceptional: exceptional
                    .into_iter()
                    .map(&mut map)
                    .collect::<Result<_, _>>()?,
            },
        })
    }

    /// Iterates over outgoing arms in semantic order.
    pub(super) fn arms(&self) -> impl Iterator<Item = &Arm> {
        let (head, tail): (&[_], &[_]) = match self {
            Self::Goto { target } => (slice::from_ref(target), &[]),
            Self::Branch { taken, otherwise } => {
                (slice::from_ref(taken), slice::from_ref(otherwise))
            }
            Self::Switch { cases, default } => (cases, slice::from_ref(default)),
            Self::Try {
                normal,
                exceptional,
                ..
            } => (slice::from_ref(normal), exceptional),
            Self::Return { exceptional, .. } | Self::Throw { exceptional, .. } => {
                (&[], exceptional)
            }
        };
        head.iter().chain(tail)
    }

    /// Iterates mutably over outgoing arms in semantic order.
    pub(super) fn arms_mut(&mut self) -> impl Iterator<Item = &mut Arm> {
        let (head, tail): (&mut [_], &mut [_]) = match self {
            Self::Goto { target } => (slice::from_mut(target), &mut []),
            Self::Branch { taken, otherwise } => {
                (slice::from_mut(taken), slice::from_mut(otherwise))
            }
            Self::Switch { cases, default } => (cases, slice::from_mut(default)),
            Self::Try {
                normal,
                exceptional,
                ..
            } => (slice::from_mut(normal), exceptional),
            Self::Return { exceptional, .. } | Self::Throw { exceptional, .. } => {
                (&mut [], exceptional)
            }
        };
        head.iter_mut().chain(tail)
    }
}

impl Terminator<Successor> {
    /// Iterates over outgoing arms in semantic order.
    pub fn successors(&self) -> impl Iterator<Item = &Successor> {
        self.arms()
    }

    /// Returns the operation attempted by this terminator, if any.
    #[must_use]
    pub const fn operation(&self) -> Option<&Operation> {
        match self {
            Self::Try { operation, .. } => Some(operation),
            _ => None,
        }
    }

    /// Returns the values used by this terminator and its successor guards.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        let successor_values = self
            .successors()
            .flat_map(|successor| successor.arguments().iter().copied());
        self.local_uses()
            .into_iter()
            .chain(successor_values)
            .collect()
    }

    /// Returns the value defined by a successful attempted operation.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        match self {
            Self::Try { operation, .. } => operation.def(),
            _ => None,
        }
    }
}

impl Terminator<Successor> {
    /// Returns the values this terminator uses, excluding successor arguments.
    pub(super) fn local_uses(&self) -> HashSet<ValueId> {
        let value = match self {
            Self::Throw { value, .. }
            | Self::Return {
                value: Some(value), ..
            } => Some(*value),
            _ => None,
        };
        let guard_uses = self
            .successors()
            .filter_map(|it| match it.transfer() {
                Some(ControlTransfer::Conditional(guard)) => Some(guard),
                _ => None,
            })
            .flat_map(BranchGuard::predicates)
            .flat_map(Predicate::uses);
        let operation_uses = match self {
            Self::Try { operation, .. } => operation.uses(),
            _ => HashSet::new(),
        };
        value
            .into_iter()
            .chain(operation_uses)
            .chain(guard_uses)
            .collect()
    }
}

impl<Arm> fmt::Display for Terminator<Arm> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Goto { .. } => f.write_str("goto"),
            Self::Branch { .. } => f.write_str("branch"),
            Self::Switch { .. } => f.write_str("switch"),
            Self::Try { operation, .. } => write!(f, "try {operation}"),
            Self::Return {
                value: Some(value), ..
            } => write!(f, "return {value}"),
            Self::Return { value: None, .. } => f.write_str("return"),
            Self::Throw { value, .. } => write!(f, "throw {value}"),
        }
    }
}

/// The semantics of one control-flow successor arm.
///
/// An exceptional arm always begins a new block, so an operation that can raise
/// never coalesces with the block its unguarded arm continues into.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ControlTransfer {
    /// An unguarded transfer that always reaches its target.
    Unconditional,
    /// A conditional transfer guarded by a conjunction of literals.
    Conditional(BranchGuard<Predicate>),
    /// An exceptional outcome selected by this catch type.
    ///
    /// `None` denotes a catch-all exception-table entry. Arm order retains
    /// JVM exception-handler precedence.
    Exception(Option<ClassRef>),
}

/// A conjunction of literals carried by a conditional CFG edge.
///
/// An empty guard represents `⊤`.
#[derive(Debug, Clone)]
pub struct BranchGuard<P>(pub(super) HashSet<BooleanVariable<P>>);

impl<P> PartialEq for BranchGuard<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<P> Eq for BranchGuard<P> where P: Hash + Eq {}

impl<P> Hash for BranchGuard<P>
where
    P: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.0).hash_unordered(state);
    }
}

impl<P> PartialOrd for BranchGuard<P>
where
    P: Hash + Eq,
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        hashset_partial_order(&self.0, &other.0)
    }
}

impl<P> BranchGuard<P> {
    /// Creates the tautological guard `⊤`.
    #[must_use]
    pub fn one() -> Self {
        Self(HashSet::new())
    }

    /// Creates a guard containing a single literal.
    #[must_use]
    pub fn of(predicate: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self(HashSet::from([predicate]))
    }

    /// Returns whether this guard is `⊤`.
    #[must_use]
    pub fn is_tautology(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterates over this guard's literals.
    ///
    /// The iteration order is unspecified.
    pub fn literals(&self) -> impl Iterator<Item = BooleanVariable<&P>> {
        self.0.iter().map(|literal| match literal {
            BooleanVariable::Positive(predicate) => BooleanVariable::Positive(predicate),
            BooleanVariable::Negative(predicate) => BooleanVariable::Negative(predicate),
        })
    }

    /// Returns the number of unique predicates referenced by this guard.
    #[must_use]
    pub fn predicate_count(&self) -> usize
    where
        P: Hash + Eq,
    {
        self.0
            .iter()
            .map(|it| match it {
                BooleanVariable::Negative(predicate) | BooleanVariable::Positive(predicate) => {
                    predicate
                }
            })
            .unique()
            .count()
    }
}

impl<P> BranchGuard<P> {
    pub(crate) fn predicates(&self) -> impl Iterator<Item = &P> {
        self.literals().map(|literal| match literal {
            BooleanVariable::Positive(predicate) | BooleanVariable::Negative(predicate) => {
                predicate
            }
        })
    }

    /// Borrows the predicates while preserving the conjunction structure.
    pub(super) fn as_ref(&self) -> BranchGuard<&P>
    where
        P: Hash + Eq,
    {
        self.0
            .iter()
            .map(|literal| match literal {
                BooleanVariable::Positive(predicate) => BooleanVariable::Positive(predicate),
                BooleanVariable::Negative(predicate) => BooleanVariable::Negative(predicate),
            })
            .collect()
    }
}

impl<P: Display> Display for BranchGuard<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_tautology() {
            write!(f, "⊤")
        } else {
            let literals = self
                .0
                .iter()
                .map(ToString::to_string)
                .sorted()
                .collect::<Vec<_>>();
            write!(f, "{}", literals.iter().format(" && "))
        }
    }
}

impl<P> FromIterator<BooleanVariable<P>> for BranchGuard<P>
where
    P: Hash + Eq,
{
    fn from_iter<T: IntoIterator<Item = BooleanVariable<P>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<P> IntoIterator for BranchGuard<P> {
    type Item = BooleanVariable<P>;
    type IntoIter = hash_set::IntoIter<BooleanVariable<P>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
