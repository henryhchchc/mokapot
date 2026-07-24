use std::{
    collections::{BTreeMap, BTreeSet, HashSet, btree_set},
    fmt,
};

use itertools::Itertools;
use std::hash::Hash;

use super::expression::{Condition, Expression};
use crate::analysis::fixed_point::JoinSemiLattice;
use crate::jvm::code::ProgramCounter;

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
/// It can contain more than one possible values for a value combined from multiple branches.
/// See the Phi function in [Static single-assignment form](https://en.wikipedia.org/wiki/Static_single-assignment_form) for more information.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Operand(
    #[cfg_attr(test, proptest(strategy = "prop_test_phi_inner()"))] BTreeSet<Identifier>,
);

/// An error returned when constructing an [`Operand`] from an empty iterator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("an operand must contain at least one identifier")]
pub struct EmptyOperandError;

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() > 1 {
            write!(f, "Phi({})", self.0.iter().format(", "))
        } else {
            self.0.first().expect("Operand is always non-empty").fmt(f)
        }
    }
}

#[cfg(test)]
fn prop_test_phi_inner() -> impl proptest::strategy::Strategy<Value = BTreeSet<Identifier>> {
    use proptest::prelude::*;
    proptest::collection::hash_set(any::<Identifier>(), 1..10).prop_map(BTreeSet::from_iter)
}

impl PartialOrd for Operand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::{Equal, Greater, Less};

        if self == other {
            Some(Equal)
        } else if self.0.is_subset(&other.0) {
            Some(Less)
        } else if other.0.is_subset(&self.0) {
            Some(Greater)
        } else {
            None
        }
    }
}

impl From<Identifier> for Operand {
    fn from(value: Identifier) -> Self {
        Self::just(value)
    }
}

impl JoinSemiLattice for Operand {
    fn join(mut self, other: Self) -> Self {
        self.0.extend(other.0);
        self
    }
}

impl IntoIterator for Operand {
    type Item = Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = btree_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Operand {
    type Item = &'a Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = btree_set::Iter<'a, Identifier>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Operand {
    /// Creates an operand that can only refer to `identifier`.
    #[must_use]
    pub fn just(identifier: Identifier) -> Self {
        Self(BTreeSet::from([identifier]))
    }

    /// Creates an operand from all identifiers yielded by `identifiers`.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyOperandError`] when `identifiers` yields no identifiers.
    pub fn try_from_iter(
        identifiers: impl IntoIterator<Item = Identifier>,
    ) -> Result<Self, EmptyOperandError> {
        let values = BTreeSet::from_iter(identifiers);
        if values.is_empty() {
            return Err(EmptyOperandError);
        }
        Ok(Self(values))
    }

    /// Returns an iterator over the possible [`Identifier`]s.
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
}

impl From<LocalValue> for Operand {
    fn from(val: LocalValue) -> Self {
        Self::just(Identifier::Local(val))
    }
}

impl From<LocalValue> for u16 {
    fn from(value: LocalValue) -> Self {
        value.0
    }
}

/// Represents an identifier of a value in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
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

    fn operand(identifiers: impl IntoIterator<Item = Identifier>) -> Operand {
        Operand::try_from_iter(identifiers).expect("test operands must not be empty")
    }

    proptest! {
        #[test]
        fn local_value_inner_conversion(id in 0..u16::MAX) {
            let value = LocalValue::new(id);
            let id: u16 = value.into();
            assert_eq!(id, value.0);
        }
    }

    #[test]
    fn operand_construction() {
        use std::collections::HashSet;

        use super::Identifier::*;

        assert_eq!(Operand::try_from_iter([]), Err(EmptyOperandError));
        assert_eq!(
            operand([This, This, Arg(0)])
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([This, Arg(0)])
        );
        assert_eq!(operand([This, Arg(0)]).to_string(), "Phi(%this, %arg0)");
    }

    #[test]
    fn operand_merge() {
        use super::Identifier::*;

        assert_eq!(
            Operand::just(This).join(Operand::just(This)),
            Operand::just(This)
        );
        assert_eq!(
            Operand::just(This).join(Operand::just(Arg(0))),
            operand([This, Arg(0)])
        );
        assert_eq!(
            Operand::just(Arg(0)).join(Operand::just(This)),
            operand([This, Arg(0)])
        );
        assert_eq!(
            Operand::just(Arg(0)).join(Operand::just(Arg(1))),
            operand([Arg(0), Arg(1)])
        );
        assert_eq!(
            Operand::just(Arg(0)).join(operand([Arg(1), Arg(2)])),
            operand([Arg(0), Arg(1), Arg(2)])
        );
        assert_eq!(
            operand([Arg(1), Arg(2)]).join(Operand::just(Arg(0))),
            operand([Arg(0), Arg(1), Arg(2)])
        );
        assert_eq!(
            operand([Arg(1), Arg(2)]).join(operand([Arg(0), Arg(1), Arg(3)])),
            operand([Arg(0), Arg(1), Arg(2), Arg(3)])
        );
    }

    #[test]
    fn operand_iter() {
        use std::collections::HashSet;

        use super::Identifier::*;

        assert_eq!(
            Operand::just(This).into_iter().collect::<HashSet<_>>(),
            HashSet::from([This])
        );
        assert_eq!(
            Operand::just(Arg(0)).into_iter().collect::<HashSet<_>>(),
            HashSet::from([Arg(0)])
        );
        assert_eq!(
            operand([Arg(0), Arg(1)])
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([Arg(0), Arg(1)])
        );
    }

    #[test]
    fn operand_iter_over_refs() {
        use std::collections::HashSet;

        use super::Identifier::*;

        assert_eq!(
            (&Operand::just(This)).into_iter().collect::<HashSet<_>>(),
            HashSet::from([&This])
        );
        assert_eq!(
            (&Operand::just(Arg(0))).into_iter().collect::<HashSet<_>>(),
            HashSet::from([&Arg(0)])
        );
        assert_eq!(
            (&operand([Arg(0), Arg(1)]))
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
