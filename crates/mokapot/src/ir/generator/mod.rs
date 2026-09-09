mod analysis;
mod assembly;
mod fallibility;
mod jvm_frame;
mod legacy;
mod lifting;
mod materialize;
mod merge;
mod remap;
mod ssa;

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, btree_set},
    fmt,
};

use jvm_frame::Entry;
pub use jvm_frame::ExecutionError;

use self::jvm_frame::JvmStackFrame;
use self::legacy::{Location, Normalizer as LegacyNormalizer, ReturnAddress};
use self::merge::{collect_phi_candidates, unavailable_value_slots};
use self::remap::{remap_expression, remap_transfer};
use super::{
    BasicBlock, BlockId, EdgeId, InstructionId, InstructionKind, MokaIRMethod, MokaInstruction,
    Phi, PhiInput, SourceMap, Successor, Terminator, TerminatorKind, ValueDefinition, ValueId,
    control_flow::{ControlTransfer, LiftedControlTransfer},
    expression::{LiftedCondition, LiftedExpression},
};

/// A value identity used only while interpreting the JVM stack machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
enum Identifier {
    #[display("%this")]
    This,
    #[display("%arg{_0}")]
    Arg(u16),
    Local(ValueId),
    #[display("%caught_exception{_0}")]
    CaughtException(ValueId),
    #[display("%return_address")]
    ReturnAddress(ReturnAddress),
}

trait FrameOperand:
    Clone + Eq + std::hash::Hash + fmt::Display + From<ValueId> + From<ReturnAddress>
{
    fn return_address(&self) -> Option<ReturnAddress>;

    fn contains_return_address(&self) -> bool;
}

/// A private reaching-definition set. Completed `MokaIR` never exposes this type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
struct Operand(BTreeSet<Identifier>);

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use itertools::Itertools as _;

        if self.0.len() > 1 {
            write!(f, "Phi({})", self.0.iter().format(", "))
        } else {
            self.0
                .first()
                .expect("lifting operands are non-empty")
                .fmt(f)
        }
    }
}

impl Operand {
    fn just(identifier: Identifier) -> Self {
        Self(BTreeSet::from([identifier]))
    }
}

impl From<ValueId> for Operand {
    fn from(value: ValueId) -> Self {
        Self::just(Identifier::Local(value))
    }
}

impl From<Identifier> for Operand {
    fn from(value: Identifier) -> Self {
        Self::just(value)
    }
}

impl From<ReturnAddress> for Operand {
    fn from(value: ReturnAddress) -> Self {
        Self::just(Identifier::ReturnAddress(value))
    }
}

impl FrameOperand for Operand {
    fn return_address(&self) -> Option<ReturnAddress> {
        (self.0.len() == 1)
            .then(|| self.0.first())
            .flatten()
            .and_then(|identifier| match identifier {
                Identifier::ReturnAddress(address) => Some(*address),
                _ => None,
            })
    }

    fn contains_return_address(&self) -> bool {
        self.0
            .iter()
            .any(|identifier| matches!(identifier, Identifier::ReturnAddress(_)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
enum ScalarValue {
    #[display("{_0}")]
    Value(ValueId),
    #[display("%return_address")]
    ReturnAddress(ReturnAddress),
}

impl From<ValueId> for ScalarValue {
    fn from(value: ValueId) -> Self {
        Self::Value(value)
    }
}

impl From<ReturnAddress> for ScalarValue {
    fn from(value: ReturnAddress) -> Self {
        Self::ReturnAddress(value)
    }
}

impl FrameOperand for ScalarValue {
    fn return_address(&self) -> Option<ReturnAddress> {
        match self {
            Self::ReturnAddress(address) => Some(*address),
            Self::Value(_) => None,
        }
    }

    fn contains_return_address(&self) -> bool {
        matches!(self, Self::ReturnAddress(_))
    }
}

impl crate::analysis::fixed_point::JoinSemiLattice for Operand {
    fn join(mut self, other: Self) -> Self {
        self.0.extend(other.0);
        self
    }
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

impl IntoIterator for Operand {
    type Item = Identifier;
    type IntoIter = btree_set::IntoIter<Identifier>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Operand {
    type Item = &'a Identifier;
    type IntoIter = btree_set::Iter<'a, Identifier>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug)]
struct GeneratedMethod {
    entry: BlockId,
    blocks: Vec<BasicBlock>,
    source_map: SourceMap,
    this_value: Option<ValueId>,
    parameter_values: Vec<ValueId>,
    caught_exceptions: BTreeMap<BlockId, ValueId>,
    value_definitions: Vec<ValueDefinition>,
}

#[derive(Debug, Clone)]
struct PlannedBlock {
    id: BlockId,
    pcs: Vec<Location>,
}

#[derive(Debug, Clone)]
struct ScalarArm {
    target: BlockId,
    transfer: LiftedControlTransfer<ScalarValue>,
    frame: JvmStackFrame<ScalarValue>,
}

#[derive(Debug, Clone)]
struct ScalarBlock {
    plan: PlannedBlock,
    entry_frame: JvmStackFrame<ScalarValue>,
    instructions: Vec<(Location, LiftedInstruction<ScalarValue>)>,
    arms: Vec<ScalarArm>,
}

type OutgoingState<OP> = (Location, LiftedControlTransfer<OP>, JvmStackFrame<OP>);
type ScalarEntryFrames = (
    BTreeMap<BlockId, JvmStackFrame<ScalarValue>>,
    BTreeMap<ValueId, BlockId>,
);
type PairedFrameValue = (Option<ValueId>, Option<ValueId>);

fn next_temp_value(next: &mut u32) -> Result<ValueId, MokaIRBrewingError> {
    let value = ValueId::new(*next);
    *next = next
        .checked_add(1)
        .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
    Ok(value)
}
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::control_flow::path_condition::{BooleanVariable, BranchGuard, LiftedValue},
    jvm::{
        ConstantValue, Method,
        code::{MethodBody, ProgramCounter},
        method,
    },
};

#[derive(Debug, Clone)]
enum LiftedInstruction<OP: fmt::Display = Operand> {
    HandlerEntry,
    Unwind,
    Nop,
    Definition {
        value: ValueId,
        expr: LiftedExpression<OP>,
    },
    Effect(LiftedExpression<OP>),
    Jump {
        condition: Option<LiftedCondition<OP>>,
        target: ProgramCounter,
    },
    Switch {
        match_value: OP,
        branches: BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Return(Option<OP>),
    Throw(OP),
    Subroutine {
        target: Location,
    },
    SubroutineReturn(OP),
}

impl<OP: fmt::Display> LiftedInstruction<OP> {
    const fn is_explicit_transfer(&self) -> bool {
        matches!(
            self,
            Self::HandlerEntry
                | Self::Unwind
                | Self::Jump { .. }
                | Self::Switch { .. }
                | Self::Return(_)
                | Self::Throw(_)
                | Self::Subroutine { .. }
                | Self::SubroutineReturn(_)
        )
    }
}

/// An error that occurs when generating Moka IR.
#[derive(Debug, thiserror::Error)]
pub enum MokaIRBrewingError {
    /// An error that occurs when executing bytecode on a JVM frame.
    #[error("Error when executing bytecode on a JVM frame: {0}")]
    ExecutionError(#[from] ExecutionError),
    /// An error that occurs when merging two stack frames.
    #[error("Error when merging two stack frames: {0}")]
    MergeError(ExecutionError),
    /// An error that occurs when a method does not have a body.
    #[error("The method does not have a body")]
    NoMethodBody,
    /// An error that occurs when the method contains malformed control flow.
    #[error("The method contains malformed control flow")]
    MalformedControlFlow,
    /// Legacy subroutine expansion exceeded its deterministic safety budget.
    #[error("legacy subroutine expansion exceeded the {limit}-location budget")]
    LegacySubroutineExpansionLimit {
        /// The maximum number of expanded locations.
        limit: usize,
    },
}

struct MokaIRGenerator<'method> {
    lifted: BTreeMap<Location, LiftedInstruction>,
    outgoing: BTreeMap<Location, Vec<(Location, LiftedControlTransfer<Operand>)>>,
    outgoing_frames: BTreeMap<Location, Vec<JvmStackFrame>>,
    value_ids: BTreeMap<Location, ValueId>,
    caught_exception_ids: BTreeMap<Location, ValueId>,
    method: &'method Method,
    body: &'method MethodBody,
    legacy: LegacyNormalizer,
    discovering: bool,
    next_lifted_value: u32,
    initial_seed: Option<(Location, JvmStackFrame)>,
}

/// An extension trait for [`Method`] that generates Moka IR.
pub trait MokaIRMethodExt {
    /// Generates Moka IR for the method.
    ///
    /// # Errors
    /// See [`MokaIRBrewingError`] for more information.
    fn brew(&self) -> Result<MokaIRMethod, MokaIRBrewingError>;
}

impl MokaIRMethodExt for Method {
    fn brew(&self) -> Result<MokaIRMethod, MokaIRBrewingError> {
        let generated = MokaIRGenerator::for_method(self)?.generate()?;
        Ok(MokaIRMethod::new(
            self.access_flags,
            self.name.clone(),
            self.descriptor.clone(),
            self.owner.clone(),
            generated.entry,
            generated.blocks,
            generated.source_map,
            generated.this_value,
            generated.parameter_values,
            generated.caught_exceptions,
            generated.value_definitions,
        ))
    }
}

#[cfg(test)]
mod tests;
