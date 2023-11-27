use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
    iter::Once,
    ops::BitOr,
};

use super::{Condition, Expression};
use crate::elements::instruction::ProgramCounter;
use itertools::{Either, Itertools};

/// Represents a single instruction in the Moka IR.
#[derive(Debug, Clone, PartialEq)]
pub enum MokaInstruction {
    /// A no-op instruction.
    Nop,
    /// Assigns [`expr`](MokaInstruction::Assignment::expr) to [`def_id`](MokaInstruction::Assignment::def_id).
    Assignment {
        def_id: Identifier,
        expr: Expression,
    },
    /// Jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) holds.
    /// Unconditionally jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) is [`None`].
    Jump {
        condition: Option<Condition>,
        target: ProgramCounter,
    },
    /// Jump to the [`target`](MokaInstruction::Switch::default) corresponding to [`match_value`](MokaInstruction::Switch::match_value).
    /// If [`match_value`](MokaInstruction::Switch::match_value) does not match any [`target`](MokaInstruction::Switch::branches), jump to [`default`](MokaInstruction::Switch::default).
    Switch {
        match_value: ValueRef,
        default: ProgramCounter,
        branches: Vec<(i32, ProgramCounter)>,
    },
    /// Returns from the current method with a value if it is [`Some`].
    /// Otherwise, returns from the current method with `void`.
    Return(Option<ValueRef>),
    /// Returns from a subroutine.
    SubroutineRet(ValueRef),
}

impl Display for MokaInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nop => write!(f, "nop"),
            Self::Assignment {
                def_id: lhs,
                expr: rhs,
            } => write!(f, "{} = {}", lhs, rhs),
            Self::Jump {
                condition: Some(condition),
                target,
            } => {
                write!(f, "if {} goto {}", condition, target)
            }
            Self::Jump {
                condition: None,
                target,
            } => {
                write!(f, "goto {}", target)
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
                        .map(|(key, target)| format!("{} => {}", key, target))
                        .join(", ")
                )
            }
            Self::Return(value) => {
                if let Some(value) = value {
                    write!(f, "return {}", value)
                } else {
                    write!(f, "return")
                }
            }
            Self::SubroutineRet(target) => write!(f, "ret {}", target),
        }
    }
}

/// Represents a reference to a value in the Moka IR.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueRef {
    /// A reference to a value defined in the current scope.
    Def(Identifier),
    /// A reference to a value combined from multiple branches.
    /// See the Phi function in [Static single-assignment form](https://en.wikipedia.org/wiki/Static_single-assignment_form) for more information.
    Phi(BTreeSet<Identifier>),
}

impl Display for ValueRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Def(id) => id.fmt(f),
            Self::Phi(ids) => write!(
                f,
                "Phi({})",
                ids.iter().map(|id| format!("{}", id)).join(", ")
            ),
        }
    }
}

impl From<Identifier> for ValueRef {
    fn from(value: Identifier) -> Self {
        Self::Def(value)
    }
}

impl BitOr for ValueRef {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        use ValueRef::*;
        match (self, rhs) {
            (Def(lhs), Def(rhs)) if lhs == rhs => Def(lhs),
            (Def(lhs), Def(rhs)) => Phi(BTreeSet::from([lhs, rhs])),
            (Def(id), Phi(mut ids)) | (Phi(mut ids), Def(id)) => {
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

impl IntoIterator for ValueRef {
    type Item = Identifier;
    type IntoIter = Either<Once<Self::Item>, std::collections::btree_set::IntoIter<Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        use ValueRef::*;
        match self {
            Def(id) => Either::Left(std::iter::once(id)),
            Phi(ids) => Either::Right(ids.into_iter()),
        }
    }
}

impl<'a> IntoIterator for &'a ValueRef {
    type Item = &'a Identifier;
    type IntoIter = Either<Once<Self::Item>, std::collections::btree_set::Iter<'a, Identifier>>;

    fn into_iter(self) -> Self::IntoIter {
        use ValueRef::*;
        match self {
            Def(id) => Either::Left(std::iter::once(id)),
            Phi(ids) => Either::Right(ids.into_iter()),
        }
    }
}

/// Represents an identifier of a value in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Identifier {
    /// The `this` value in an instance method.
    This,
    /// An argument of the current method.
    Arg(u16),
    /// A locally defined value.
    Val(u16),
    /// The exception caught by a `catch` block.
    CaughtException,
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Identifier::*;
        match self {
            This => write!(f, "%this"),
            Arg(idx) => write!(f, "%arg{}", idx),
            Val(idx) => write!(f, "%{}", idx),
            CaughtException => write!(f, "%caught_exception"),
        }
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn value_ref_merge() {
        use super::Identifier::*;
        use super::ValueRef::*;
        use std::collections::BTreeSet;

        assert_eq!(Def(This) | Def(This), Def(This));
        assert_eq!(Def(This) | Def(Arg(0)), Phi(BTreeSet::from([This, Arg(0)])));
        assert_eq!(Def(Arg(0)) | Def(This), Phi(BTreeSet::from([This, Arg(0)])));
        assert_eq!(
            Def(Arg(0)) | Def(Arg(1)),
            Phi(BTreeSet::from([Arg(0), Arg(1)]))
        );
        assert_eq!(
            Def(Arg(0)) | Phi(BTreeSet::from([Arg(1), Arg(2)])),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(1), Arg(2)])) | Def(Arg(0)),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(1), Arg(2)])) | Phi(BTreeSet::from([Arg(0), Arg(1), Arg(3)])),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2), Arg(3)]))
        );
    }

    #[test]
    fn value_ref_iter() {
        use super::Identifier::*;
        use super::ValueRef::*;
        use std::collections::BTreeSet;

        assert_eq!(
            Def(This).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([This])
        );
        assert_eq!(
            Def(Arg(0)).into_iter().collect::<BTreeSet<_>>(),
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
        use super::Identifier::*;
        use super::ValueRef::*;
        use std::collections::BTreeSet;

        assert_eq!(
            (&Def(This)).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([&This])
        );
        assert_eq!(
            (&Def(Arg(0))).into_iter().collect::<BTreeSet<_>>(),
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
