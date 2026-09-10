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
