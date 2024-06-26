use std::{
    collections::{btree_set, BTreeMap, BTreeSet},
    fmt::{self, Display, Formatter},
    iter::{self, Once},
    ops::BitOr,
};

use crate::jvm::code::ProgramCounter;
use itertools::{Either, Itertools};

use super::expression::{Condition, Expression};

/// Represents a single instruction in the Moka IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MokaInstruction {
    /// A no-op instruction.
    Nop,
    /// Creates a definition by evaluating an [`Expression`].
    Definition {
        /// The value defined by the expression.
        value: LocalValue,
        /// The expression that defines the value.
        expr: Expression,
    },
    /// Jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) holds.
    /// Unconditionally jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) is [`None`].
    Jump {
        /// The condition that must hold for the jump to occur.
        /// It denotes an Unconditional jump if it is [`None`].
        condition: Option<Condition>,
        /// The target of the jump.
        target: ProgramCounter,
    },
    /// Jump to the [`target`](MokaInstruction::Switch::default) corresponding to [`match_value`](MokaInstruction::Switch::match_value).
    /// If [`match_value`](MokaInstruction::Switch::match_value) does not match any [`target`](MokaInstruction::Switch::branches), jump to [`default`](MokaInstruction::Switch::default).
    Switch {
        /// The value to match against the branches.
        match_value: Argument,
        /// The branches of the switch.
        branches: BTreeMap<i32, ProgramCounter>,
        /// The target of the switch if no branches match.
        default: ProgramCounter,
    },
    /// Returns from the current method with a value if it is [`Some`].
    /// Otherwise, returns from the current method with `void`.
    Return(Option<Argument>),
    /// Returns from a subroutine.
    SubroutineRet(Argument),
}

impl MokaInstruction {
    /// Returns the value defined by the instruction if it is a definition.
    #[must_use]
    pub fn def(&self) -> Option<LocalValue> {
        match self {
            Self::Definition { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Returns the set of [`Identifier`]s used by the instruction.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Nop => BTreeSet::new(),
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
            _ => BTreeSet::default(),
        }
    }
}

impl Display for MokaInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nop => write!(f, "nop"),
            Self::Definition {
                value: def_id,
                expr,
            } => write!(f, "{def_id} = {expr}"),
            Self::Jump {
                condition: Some(condition),
                target,
            } => {
                write!(f, "if {condition} goto {target}")
            }
            Self::Jump {
                condition: None,
                target,
            } => {
                write!(f, "goto {target}")
            }
            Self::Switch {
                match_value,
                default,
                branches,
            } => {
                write!(
                    f,
                    "switch {} {{ {}, else => {} }}",
                    match_value,
                    default,
                    branches
                        .iter()
                        .map(|(key, target)| format!("{key} => {target}"))
                        .join(", ")
                )
            }
            Self::Return(Some(value)) => write!(f, "return {value}"),
            Self::Return(None) => write!(f, "return"),
            Self::SubroutineRet(target) => write!(f, "subroutine_ret {target}"),
        }
    }
}

/// Represents a reference to a value in the Moka IR.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Argument {
    /// A reference to a value defined in the current scope.
    Id(Identifier),
    /// A reference to a value combined from multiple branches.
    /// See the Phi function in [Static single-assignment form](https://en.wikipedia.org/wiki/Static_single-assignment_form) for more information.
    Phi(BTreeSet<Identifier>),
}

impl Display for Argument {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => id.fmt(f),
            Self::Phi(ids) => write!(
                f,
                "Phi({})",
                ids.iter().map(|id| format!("{id}")).join(", ")
            ),
        }
    }
}

impl From<Identifier> for Argument {
    fn from(value: Identifier) -> Self {
        Self::Id(value)
    }
}

impl BitOr for Argument {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        use Argument::{Id, Phi};
        match (self, rhs) {
            (Id(lhs), Id(rhs)) if lhs == rhs => Id(lhs),
            (Id(lhs), Id(rhs)) => Phi(BTreeSet::from([lhs, rhs])),
            (Id(id), Phi(mut ids)) | (Phi(mut ids), Id(id)) => {
                ids.insert(id);
                Phi(ids)
            }
            (Phi(mut lhs), Phi(mut rhs)) => {
                lhs.append(&mut rhs);
                Phi(lhs)
            }
        }
    }
}

impl IntoIterator for Argument {
    type Item = Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = Either<Once<Self::Item>, btree_set::IntoIter<Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        use Argument::{Id, Phi};
        match self {
            Id(id) => Either::Left(iter::once(id)),
            Phi(ids) => Either::Right(ids.into_iter()),
        }
    }
}

impl<'a> IntoIterator for &'a Argument {
    type Item = &'a Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = Either<Once<Self::Item>, btree_set::Iter<'a, Identifier>>;

    fn into_iter(self) -> Self::IntoIter {
        use Argument::{Id, Phi};
        match self {
            Id(id) => Either::Left(iter::once(id)),
            Phi(ids) => Either::Right(ids.iter()),
        }
    }
}

impl Argument {
    /// Creates an iterator over the possible [`Identifier`].
    pub fn iter(&self) -> impl Iterator<Item = &Identifier> {
        self.into_iter()
    }
}

/// A unique identifier of a value defined in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(transparent)]
#[display(fmt = "%{_0}")]
pub struct LocalValue(u16);

impl LocalValue {
    /// Creates a new [`LocalValue`] with the given ID.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Create an [`Argument`] by referencing this [`LocalValue`].
    #[must_use]
    pub fn as_argument(&self) -> Argument {
        Argument::Id((*self).into())
    }
}

/// Represents an identifier of a value in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Identifier {
    /// The `this` value in an instance method.
    This,
    /// An argument of the current method.
    Arg(u16),
    /// A locally defined value.
    Local(LocalValue),
    /// The exception caught by a `catch` block.
    CaughtException,
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Identifier::{Arg, CaughtException, Local, This};
        match self {
            This => write!(f, "%this"),
            Arg(idx) => write!(f, "%arg{idx}"),
            Local(idx) => idx.fmt(f),
            CaughtException => write!(f, "%caught_exception"),
        }
    }
}

impl From<LocalValue> for Identifier {
    fn from(value: LocalValue) -> Self {
        Self::Local(value)
    }
}

#[cfg(test)]
pub(super) mod test {
    use super::*;
    use proptest::prelude::*;

    pub(crate) fn arb_argument() -> impl Strategy<Value = Argument> {
        prop_oneof![
            any::<Identifier>().prop_map(Argument::Id),
            prop::collection::btree_set(any::<Identifier>(), 1..10).prop_map(Argument::Phi)
        ]
    }

    #[test]
    fn value_ref_merge() {
        use super::Argument::*;
        use super::Identifier::*;
        use std::collections::BTreeSet;

        assert_eq!(Id(This) | Id(This), Id(This));
        assert_eq!(Id(This) | Id(Arg(0)), Phi(BTreeSet::from([This, Arg(0)])));
        assert_eq!(Id(Arg(0)) | Id(This), Phi(BTreeSet::from([This, Arg(0)])));
        assert_eq!(
            Id(Arg(0)) | Id(Arg(1)),
            Phi(BTreeSet::from([Arg(0), Arg(1)]))
        );
        assert_eq!(
            Id(Arg(0)) | Phi(BTreeSet::from([Arg(1), Arg(2)])),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(1), Arg(2)])) | Id(Arg(0)),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(1), Arg(2)])) | Phi(BTreeSet::from([Arg(0), Arg(1), Arg(3)])),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2), Arg(3)]))
        );
    }

    #[test]
    fn value_ref_iter() {
        use super::Argument::*;
        use super::Identifier::*;
        use std::collections::BTreeSet;

        assert_eq!(
            Id(This).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([This])
        );
        assert_eq!(
            Id(Arg(0)).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([Arg(0)])
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(0), Arg(1)]))
                .into_iter()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([Arg(0), Arg(1)])
        );
    }

    #[test]
    fn value_ref_iter_over_refs() {
        use super::Argument::*;
        use super::Identifier::*;
        use std::collections::BTreeSet;

        assert_eq!(
            (&Id(This)).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([&This])
        );
        assert_eq!(
            (&Id(Arg(0))).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([&Arg(0)])
        );
        assert_eq!(
            (&Phi(BTreeSet::from([Arg(0), Arg(1)])))
                .into_iter()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([&Arg(0), &Arg(1)])
        );
    }
}
