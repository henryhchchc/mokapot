//! Path constraint analysis.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    iter::once,
};

use crate::{
    ir::{
        self, control_flow::ControlTransfer, expression::Expression, MokaIRMethod, MokaInstruction,
        Operand,
    },
    jvm::{code::ProgramCounter, ConstantValue},
};

use super::fixed_point;

/// An analyzer for path conditions.
#[derive(Debug)]
pub struct Analyzer<'a> {
    method: &'a MokaIRMethod,
}

impl<'a> Analyzer<'a> {
    /// Creates a new path condition analyzer.
    #[must_use]
    pub fn new(method: &'a MokaIRMethod) -> Self {
        Self { method }
    }
}

/// A boolean expression.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum BooleanExpr<C> {
    /// A conjunction of clauses.
    Conjunction {
        /// The clauses.
        clauses: BTreeSet<C>,
    },
    /// A disjunction of clauses.
    Disjunction {
        /// The clauses.
        clauses: BTreeSet<C>,
    },
}

impl<C: Ord> BooleanExpr<C> {
    /// Creates a new conjunction.
    pub fn conjunction(clauses: impl Into<BTreeSet<C>>) -> Self {
        let clauses = clauses.into();
        Self::Conjunction { clauses }
    }

    /// Creates a new disjunction.
    pub fn disjunction(clauses: impl Into<BTreeSet<C>>) -> Self {
        let clauses = clauses.into();
        Self::Disjunction { clauses }
    }
}

impl<C> BooleanExpr<C>
where
    C: Ord + std::ops::Not<Output = C> + Clone,
{
    /// Normalizes the expression into conjunctive normal form.
    #[must_use]
    pub fn into_cnf(self) -> Self {
        if let Self::Disjunction { clauses: dis } = self {
            Self::Conjunction {
                clauses: dis.into_iter().map(|it| !it).collect(),
            }
        } else {
            self
        }
    }

    /// Normalizes the expression into disjunctive normal form.
    #[must_use]
    pub fn into_dnf(self) -> Self {
        if let Self::Conjunction { clauses: con } = self {
            Self::Disjunction {
                clauses: con.into_iter().map(|it| !it).collect(),
            }
        } else {
            self
        }
    }
}

impl<C> Display for BooleanExpr<C>
where
    C: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[allow(clippy::enum_glob_use)]
        use BooleanExpr::*;
        let (clauses, join_op) = match self {
            Conjunction { clauses } => (clauses, "&&"),
            Disjunction { clauses } => (clauses, "||"),
        };
        for (idx, clause) in clauses.iter().enumerate() {
            if idx > 0 {
                write!(f, " {join_op} ")?;
            }
            write!(f, "{clause}")?;
        }
        Ok(())
    }
}

impl<C, T> std::ops::Not for BooleanExpr<C>
where
    C: std::ops::Not<Output = T> + Clone,
    T: Ord,
{
    type Output = BooleanExpr<T>;

    fn not(self) -> Self::Output {
        #[allow(clippy::enum_glob_use)]
        use BooleanExpr::*;
        match self {
            Conjunction { clauses } => Disjunction {
                clauses: clauses.into_iter().map(std::ops::Not::not).collect(),
            },
            Disjunction { clauses } => Conjunction {
                clauses: clauses.into_iter().map(std::ops::Not::not).collect(),
            },
        }
    }
}

impl<V> std::ops::BitAnd for BooleanExpr<V>
where
    V: Ord + std::ops::Not<Output = V>,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Conjunction { clauses: lhs }, Self::Conjunction { clauses: rhs }) => {
                Self::Conjunction {
                    clauses: lhs.into_iter().chain(rhs).collect(),
                }
            }
            (Self::Conjunction { clauses: con }, Self::Disjunction { clauses: dis })
            | (Self::Disjunction { clauses: dis }, Self::Conjunction { clauses: con }) => {
                Self::Conjunction {
                    clauses: con
                        .into_iter()
                        .chain(dis.into_iter().map(|it| !it))
                        .collect(),
                }
            }
            (Self::Disjunction { clauses: lhs }, Self::Disjunction { clauses: rhs }) => {
                Self::Disjunction {
                    clauses: lhs.into_iter().chain(rhs).map(|it| !it).collect(),
                }
            }
        }
    }
}

impl<V> std::ops::BitOr for BooleanExpr<V>
where
    V: Ord + std::ops::Not<Output = V>,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Conjunction { clauses: lhs }, Self::Conjunction { clauses: rhs }) => {
                Self::Conjunction {
                    clauses: lhs.into_iter().chain(rhs).map(|it| !it).collect(),
                }
            }
            (Self::Conjunction { clauses: con }, Self::Disjunction { clauses: dis })
            | (Self::Disjunction { clauses: dis }, Self::Conjunction { clauses: con }) => {
                Self::Disjunction {
                    clauses: dis
                        .into_iter()
                        .chain(con.into_iter().map(|it| !it))
                        .collect(),
                }
            }
            (Self::Disjunction { clauses: lhs }, Self::Disjunction { clauses: rhs }) => {
                Self::Disjunction {
                    clauses: lhs.into_iter().chain(rhs).collect(),
                }
            }
        }
    }
}

impl<V> Default for BooleanExpr<V> {
    fn default() -> Self {
        Self::Disjunction {
            clauses: BTreeSet::default(),
        }
    }
}

/// A condition.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Condition<V> {
    /// The left-hand side is equal to the right-hand side.
    Equal(V, V),
    /// The left-hand side is not equal to the right-hand side.
    NotEqual(V, V),
    /// The left-hand side is less than the right-hand side.
    LessThan(V, V),
    /// The left-hand side is less than or equal to the right-hand side.
    LessThanOrEqual(V, V),
    /// The value is null.
    IsNull(V),
    /// The value is not null.
    IsNotNull(V),
}

impl<V> std::ops::Not for Condition<V> {
    type Output = Self;

    fn not(self) -> Self::Output {
        #[allow(clippy::enum_glob_use)]
        use Condition::*;
        match self {
            Equal(lhs, rhs) => NotEqual(lhs, rhs),
            NotEqual(lhs, rhs) => Equal(lhs, rhs),
            LessThan(lhs, rhs) => LessThanOrEqual(rhs, lhs),
            LessThanOrEqual(lhs, rhs) => LessThan(rhs, lhs),
            IsNull(value) => IsNotNull(value),
            IsNotNull(value) => IsNull(value),
        }
    }
}

impl<V> Display for Condition<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal(lhs, rhs) => write!(f, "{lhs} == {rhs}"),
            Self::NotEqual(lhs, rhs) => write!(f, "{lhs} != {rhs}"),
            Self::LessThan(lhs, rhs) => write!(f, "{lhs} < {rhs}"),
            Self::LessThanOrEqual(lhs, rhs) => write!(f, "{lhs} <= {rhs}"),
            Self::IsNull(value) => write!(f, "{value} == null"),
            Self::IsNotNull(value) => write!(f, "{value} != null"),
        }
    }
}

impl<C: Ord> From<Condition<C>> for BooleanExpr<Condition<C>> {
    fn from(value: Condition<C>) -> Self {
        BooleanExpr::Disjunction {
            clauses: BTreeSet::from([value]),
        }
    }
}

impl From<ir::expression::Condition> for BooleanExpr<Condition<Value>> {
    fn from(value: ir::expression::Condition) -> Self {
        #[allow(clippy::enum_glob_use)]
        use ir::expression::Condition::*;

        let zero = ConstantValue::Integer(0).into();
        let cond = match value {
            IsZero(value) => Condition::Equal(value.into(), zero),
            IsNonZero(value) => Condition::NotEqual(value.into(), zero),
            IsPositive(value) => Condition::LessThan(zero, value.into()),
            IsNegative(value) => Condition::LessThan(value.into(), zero),
            IsNonPositive(value) => Condition::LessThanOrEqual(value.into(), zero),
            IsNonNegative(value) => Condition::LessThanOrEqual(zero, value.into()),
            Equal(lhs, rhs) => Condition::Equal(lhs.into(), rhs.into()),
            NotEqual(lhs, rhs) => Condition::NotEqual(lhs.into(), rhs.into()),
            LessThan(lhs, rhs) => Condition::LessThan(lhs.into(), rhs.into()),
            LessThanOrEqual(lhs, rhs) => Condition::LessThanOrEqual(lhs.into(), rhs.into()),
            GreaterThan(lhs, rhs) => Condition::LessThan(rhs.into(), lhs.into()),
            GreaterThanOrEqual(lhs, rhs) => Condition::LessThanOrEqual(rhs.into(), lhs.into()),
            IsNull(value) => Condition::IsNull(value.into()),
            IsNotNull(value) => Condition::IsNotNull(value.into()),
        };
        BooleanExpr::disjunction([cond])
    }
}

/// A value.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, derive_more::Display)]
pub enum Value {
    /// An operand.
    Operand(Operand),
    /// A constant value.
    Constant(ConstantValue),
}

impl From<ir::Operand> for Value {
    fn from(value: ir::Operand) -> Self {
        Self::Operand(value)
    }
}

impl From<ConstantValue> for Value {
    fn from(value: ConstantValue) -> Self {
        Self::Constant(value)
    }
}

impl fixed_point::Analyzer for Analyzer<'_> {
    type Location = ProgramCounter;

    type Fact = BooleanExpr<Condition<Value>>;

    type Err = AnalysisError;

    type AffectedLocations = BTreeMap<Self::Location, Self::Fact>;

    fn entry_fact(&self) -> Result<(Self::Location, Self::Fact), Self::Err> {
        Ok((ProgramCounter::ZERO, Self::Fact::default()))
    }

    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::AffectedLocations, Self::Err> {
        let insn = self
            .method
            .instructions
            .get(location)
            .ok_or(AnalysisError::InstructionNotFound)?;
        let affected_locations = match insn {
            MokaInstruction::Jump { condition, target } => {
                if let Some(condition) = condition {
                    let condition: BooleanExpr<_> = condition.clone().into();
                    let target = {
                        let pc = *target;
                        let condition = fact.clone() & condition.clone();
                        (pc, condition)
                    };
                    // let next_pc = self.get_next_pc(*location)?;
                    let next_pc = self.get_next_pc(*location)?;
                    let fallthrough = (next_pc, fact.clone() & !condition);
                    vec![target, fallthrough]
                } else {
                    vec![(*target, fact.clone())]
                }
            }
            MokaInstruction::Switch {
                match_value,
                branches,
                default,
            } => {
                let match_targets = branches.iter().map(|(value, target)| {
                    let condition = Condition::Equal(
                        match_value.clone().into(),
                        ConstantValue::Integer(*value).into(),
                    );
                    let condition: BooleanExpr<_> = condition.into();
                    let condition = fact.clone() & condition.clone();
                    (*target, condition)
                });
                let default_condition = default_branch_condition(branches, match_value);
                let default = {
                    let pc = *default;
                    let condition = fact.clone() & default_condition.clone();
                    (pc, condition)
                };
                match_targets.chain(once(default)).collect()
            }
            MokaInstruction::SubroutineRet(_) => {
                let return_addresses = self
                    .method
                    .control_flow_graph
                    .edges_from(*location)
                    .ok_or(AnalysisError::MalformControlFlow)?
                    .filter(|(_, _, it)| matches!(it, ControlTransfer::SubroutineReturn));
                return_addresses
                    .map(|(_, addr, _)| (addr, fact.clone()))
                    .collect()
            }
            MokaInstruction::Definition {
                expr: Expression::Subroutine { target, .. },
                ..
            } => {
                let target = (*target, fact.clone());
                vec![target]
            }
            MokaInstruction::Definition { expr, .. } => {
                let handlers = self
                    .method
                    .exception_table
                    .iter()
                    .filter(|it| it.covers(*location))
                    .map(|it| (it.handler_pc, fact.clone()));
                let next_pc = if let Expression::Throw(_) = expr {
                    None
                } else {
                    let next_pc = self.get_next_pc(*location)?;
                    Some((next_pc, fact.clone()))
                };
                handlers.chain(next_pc).collect()
            }
            MokaInstruction::Return(_) => Vec::default(),
            MokaInstruction::Nop => {
                // let next_pc = self.get_next_pc(*location)?;
                let next_pc = self.get_next_pc(*location)?;
                vec![(next_pc, fact.clone())]
            }
        };
        Ok(affected_locations
            .into_iter()
            .map(|(pc, cond)| (pc, cond.into_dnf()))
            .collect())
    }

    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err> {
        Ok(current_fact.clone() | incoming_fact)
    }
}

fn default_branch_condition(
    branches: &BTreeMap<i32, ProgramCounter>,
    match_value: &Operand,
) -> BooleanExpr<Condition<Value>> {
    branches
        .keys()
        .map(|value| {
            let condition = Condition::NotEqual(
                match_value.clone().into(),
                ConstantValue::Integer(*value).into(),
            );
            let condition: BooleanExpr<_> = condition.into();
            condition
        })
        .reduce(|lhs, rhs| lhs & rhs)
        .unwrap_or(BooleanExpr::default())
}

impl Analyzer<'_> {
    fn get_next_pc(&self, pc: ProgramCounter) -> Result<ProgramCounter, AnalysisError> {
        self.method
            .instructions
            .next_pc_of(&pc)
            .ok_or(AnalysisError::MalformControlFlow)
    }
}

/// Error when analyzing path conditions.
#[derive(Debug, thiserror::Error, derive_more::Display)]
pub enum AnalysisError {
    /// Cannot find instruction.
    #[display(fmt = "Cannot find instruction.")]
    InstructionNotFound,
    /// Malformed control flow in the method.
    #[display(fmt = "Malformed control flow in the method.")]
    MalformControlFlow,
}

#[cfg(test)]
mod test {}
