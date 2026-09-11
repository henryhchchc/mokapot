//! `MokaIR` is an intermediate representation of JVM bytecode.
//! It is register based and is in SSA form, which make it easier to analyze.

mod basic_block;
pub mod control_flow;
pub mod data_flow;
pub mod expression;
mod generator;
mod identity;
mod method;
mod operation;
#[cfg(feature = "petgraph")]
pub mod petgraph;
mod phi;
mod source_map;
mod terminator;

pub use basic_block::BasicBlock;
pub use data_flow::{DefUseChain, UseSite};
pub use generator::MokaIRBuildError;
pub use identity::{BlockId, EdgeId, InstructionId, ValueId};
pub use method::MokaIRMethod;
pub use operation::{Operation, OperationKind};
pub use phi::{Phi, PhiInput, ValueDefinition};
pub use source_map::SourceMap;
pub use terminator::{Successor, Terminator, TerminatorKind};

/// Maps all IR values contained by a structure into a corresponding structure.
///
/// Implementations preserve non-value data and stop at the first mapping
/// error. When an implementation contains values in an unordered collection,
/// traversal follows that collection's iteration order; mappers must map equal
/// values consistently.
pub trait TryMapValues<OUT> {
    /// The type of IR values contained by this structure.
    type Value;

    /// The corresponding structure after its values have been mapped to `OUT`.
    type Mapped;

    /// Maps contained values, stopping at the first mapping error.
    ///
    /// # Errors
    ///
    /// Returns the first error from `remap`.
    fn try_map_values<E>(
        self,
        remap: impl FnMut(Self::Value) -> Result<OUT, E>,
    ) -> Result<Self::Mapped, E>;
}
