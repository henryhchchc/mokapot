use std::{
    collections::{BTreeMap, HashSet, hash_set},
    iter::{self, Once},
};

use itertools::{Either, Itertools};
use std::hash::Hash;

use super::expression::{Condition, Expression};
use crate::jvm::code::ProgramCounter;
use crate::{
    analysis::fixed_point::JoinSemiLattice,
    intrinsics::{HashUnordered, hashset_partial_order},
};

/// Represents a single instruction in the Moka IR.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum MokaInstruction {
    /// A no-op instruction.
    #[display("nop")]
    Nop,
    /// Creates a definition by evaluating an [`Expression`].
    #[display("{value} = {expr}")]
    Definition {
        /// The value defined by the expression.
        value: LocalValue,
        /// The expression that defines the value.
        expr: Expression,
    },
    /// Jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) holds.
    /// Unconditionally jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) is [`None`].
    #[display("{}goto {target}", condition.as_ref().map(|cond| format!("if {cond} ")).unwrap_or_default())]
    Jump {
        /// The condition that must hold for the jump to occur.
        /// It denotes an Unconditional jump if it is [`None`].
        condition: Option<Condition>,
        /// The target of the jump.
        target: ProgramCounter,
    },
    /// Jump to the [`target`](MokaInstruction::Switch::default) corresponding to [`match_value`](MokaInstruction::Switch::match_value).
    /// If [`match_value`](MokaInstruction::Switch::match_value) does not match any [`target`](MokaInstruction::Switch::branches), jump to [`default`](MokaInstruction::Switch::default).
    #[display(
        "switch {} {{ {}, else => {} }}",
        match_value,
        branches.iter().map(|(key, target)| format!("{key} => {target}")).join(", "),
        default
    )]
    Switch {
        /// The value to match against the branches.
        match_value: Operand,
        /// The branches of the switch.
        branches: BTreeMap<i32, ProgramCounter>,
        /// The target of the switch if no branches match.
        default: ProgramCounter,
    },
    /// Returns from the current method with a value if it is [`Some`].
    /// Otherwise, returns from the current method with `void`.
    #[display("return{}", _0.as_ref().map(|it| format!(" {it}")).unwrap_or_default())]
    Return(Option<Operand>),
    /// Returns from a subroutine.
    #[display("subroutine_ret {_0}")]
    SubroutineRet(Operand),
}

impl MokaInstruction {
    /// Returns the value defined by the instruction if it is a definition.
    #[must_use]
    pub const fn def(&self) -> Option<LocalValue> {
        match self {
            Self::Definition { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Returns the set of [`Identifier`]s used by the instruction.
    #[must_use]
    pub fn uses(&self) -> HashSet<Identifier> {
        match self {
            Self::Nop => HashSet::new(),
            Self::Definition { expr, .. } => expr.uses(),
            Self::Jump {
                condition: Some(condition),
                ..
            } => condition.uses(),
            Self::Return(Some(uses))
            | Self::SubroutineRet(uses)
            | Self::Switch {
                match_value: uses, ..
            } => uses.iter().copied().collect(),
            _ => HashSet::default(),
        }
    }
}

/// Represents a reference to a value in the Moka IR.
#[derive(Debug, PartialEq, Eq, Clone, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Operand {
    /// A reference to a value defined in the current scope.
    #[display("{_0}")]
    Just(Identifier),
    /// A reference to a value combined from multiple branches.
    /// See the Phi function in [Static single-assignment form](https://en.wikipedia.org/wiki/Static_single-assignment_form) for more information.
    #[display("Phi({})", _0.iter().map(ToString::to_string).join(", "))]
    #[cfg_attr(test, proptest(strategy = "prop_test_phi_inner()"))]
    Phi(HashSet<Identifier>),
}

#[cfg(test)]
fn prop_test_phi_inner() -> impl proptest::strategy::Strategy<Value = Operand> {
    use proptest::prelude::*;
    proptest::collection::hash_set(any::<Identifier>(), 1..10).prop_map(Operand::Phi)
}

impl PartialOrd for Operand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use Operand::{Just, Phi};
        use std::cmp::Ordering::{Equal, Greater, Less};
        match (self, other) {
            (Just(lhs), Just(rhs)) => lhs.eq(rhs).then_some(Equal),
            (Just(lhs), Phi(rhs_set)) => rhs_set.contains(lhs).then_some(Less),
            (Phi(lhs_set), Just(rhs)) => lhs_set.contains(rhs).then_some(Greater),
            (Phi(lhs), Phi(rhs)) => hashset_partial_order(lhs, rhs),
        }
    }
}

impl Hash for Operand {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Operand::Just(id) => id.hash(state),
            Operand::Phi(ids) => ids.hash_unordered(state),
        }
    }
}

impl From<Identifier> for Operand {
    fn from(value: Identifier) -> Self {
        Self::Just(value)
    }
}

impl JoinSemiLattice for Operand {
    fn join(self, other: Self) -> Self {
        use Operand::{Just, Phi};
        match (self, other) {
            (Just(lhs), Just(rhs)) if lhs == rhs => Just(lhs),
            (Just(lhs), Just(rhs)) => Phi(HashSet::from([lhs, rhs])),
            (Just(id), Phi(mut ids)) | (Phi(mut ids), Just(id)) => {
                ids.insert(id);
                Phi(ids)
            }
            (Phi(mut lhs), Phi(rhs)) => {
                lhs.extend(rhs);
                Phi(lhs)
            }
        }
    }
}

impl IntoIterator for Operand {
    type Item = Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = Either<Once<Self::Item>, hash_set::IntoIter<Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        use Operand::{Just, Phi};
        match self {
            Just(id) => Either::Left(iter::once(id)),
            Phi(ids) => Either::Right(ids.into_iter()),
        }
    }
}

impl<'a> IntoIterator for &'a Operand {
    type Item = &'a Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = Either<Once<Self::Item>, hash_set::Iter<'a, Identifier>>;

    fn into_iter(self) -> Self::IntoIter {
        use Operand::{Just, Phi};
        match self {
            Just(id) => Either::Left(iter::once(id)),
            Phi(ids) => Either::Right(ids.iter()),
        }
    }
}

impl Operand {
    /// Creates an iterator over the possible [`Identifier`].
    pub fn iter(&self) -> impl Iterator<Item = &Identifier> {
        self.into_iter()
    }
}

/// A unique identifier of a value defined in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(transparent)]
#[display("%{_0}")]
pub struct LocalValue(u16);

impl LocalValue {
    /// Creates a new [`LocalValue`] with the given ID.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Create an [`Operand`] by referencing this [`LocalValue`].
    #[must_use]
    pub const fn as_operand(&self) -> Operand {
        Operand::Just(Identifier::Local(*self))
    }
}

impl From<LocalValue> for u16 {
    fn from(value: LocalValue) -> Self {
        value.0
    }
}

/// Represents an identifier of a value in the current scope.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Identifier {
    /// The `this` value in an instance method.
    #[display("%this")]
    This,
    /// An argument of the current method.
    #[display("%arg{_0}")]
    Arg(u16),
    /// A locally defined value.
    Local(LocalValue),
    /// The exception caught by a `catch` block.
    #[display("%caught_exception@{_0}")]
    CaughtException(ProgramCounter),
}

impl From<LocalValue> for Identifier {
    fn from(value: LocalValue) -> Self {
        Self::Local(value)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn local_value_inner_conversion(id in 0..u16::MAX) {
            let value = LocalValue::new(id);
            let id: u16 = value.into();
            assert_eq!(id, value.0);
        }
    }

    #[test]
    fn value_ref_merge() {
        use std::collections::HashSet;

        use super::{Identifier::*, Operand::*};

        assert_eq!(Just(This).join(Just(This)), Just(This));
        assert_eq!(
            Just(This).join(Just(Arg(0))),
            Phi(HashSet::from([This, Arg(0)]))
        );
        assert_eq!(
            Just(Arg(0)).join(Just(This)),
            Phi(HashSet::from([This, Arg(0)]))
        );
        assert_eq!(
            Just(Arg(0)).join(Just(Arg(1))),
            Phi(HashSet::from([Arg(0), Arg(1)]))
        );
        assert_eq!(
            Just(Arg(0)).join(Phi(HashSet::from([Arg(1), Arg(2)]))),
            Phi(HashSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(HashSet::from([Arg(1), Arg(2)])).join(Just(Arg(0))),
            Phi(HashSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(HashSet::from([Arg(1), Arg(2)])).join(Phi(HashSet::from([Arg(0), Arg(1), Arg(3)]))),
            Phi(HashSet::from([Arg(0), Arg(1), Arg(2), Arg(3)]))
        );
    }

    #[test]
    fn value_ref_iter() {
        use std::collections::HashSet;

        use super::{Identifier::*, Operand::*};

        assert_eq!(
            Just(This).into_iter().collect::<HashSet<_>>(),
            HashSet::from([This])
        );
        assert_eq!(
            Just(Arg(0)).into_iter().collect::<HashSet<_>>(),
            HashSet::from([Arg(0)])
        );
        assert_eq!(
            Phi(HashSet::from([Arg(0), Arg(1)]))
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([Arg(0), Arg(1)])
        );
    }

    #[test]
    fn value_ref_iter_over_refs() {
        use std::collections::HashSet;

        use super::{Identifier::*, Operand::*};

        assert_eq!(
            (&Just(This)).into_iter().collect::<HashSet<_>>(),
            HashSet::from([&This])
        );
        assert_eq!(
            (&Just(Arg(0))).into_iter().collect::<HashSet<_>>(),
            HashSet::from([&Arg(0)])
        );
        assert_eq!(
            (&Phi(HashSet::from([Arg(0), Arg(1)])))
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([&Arg(0), &Arg(1)])
        );
    }

    proptest! {
       #[test]
       fn operand_join_ordering(
           lhs in any::<Operand>(),
           rhs in any::<Operand>(),
       ) {
           let joined = lhs.clone().join(rhs.clone());
           prop_assert!(joined >= lhs);
           prop_assert!(joined >= rhs);
       }
    }
}
