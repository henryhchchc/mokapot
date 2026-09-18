//! Scalar SSA intermediate representation of reachable JVM method behavior.
//!
//! Build a [`MokaIRMethod`] from a [`crate::jvm::Method`] with
//! [`MokaIRMethod::from_method`]. The result contains maximal [`BasicBlock`]s:
//! block-entry [`Phi`] nodes, semantic [`Operation`]s in execution order, and
//! exactly one [`Terminator`]. The terminators' ordered [`Successor`] arms are
//! the authoritative control-flow graph.
//!
//! Each operand is one method-local [`ValueId`] with one [`ValueDefinition`].
//! JVM local slots and the operand stack exist only while lifting. Effect-only
//! operations remain ordered but define no value. Potentially throwing
//! operations have distinct normal and exceptional successor arms, and each
//! reachable handler context has a distinct caught-exception value.
//!
//! [`SourceMap`] records sparse, bidirectional JVM provenance. There is no
//! bytecode-to-IR bijection: erased stack operations may have no IR node, while
//! phis and other synthetic nodes may have no JVM origin.
//!
//! # Example
//!
//! ```no_run
//! use std::{fs::File, io::BufReader};
//!
//! use mokapot::{ir::MokaIRMethod, jvm::Class};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut reader = BufReader::new(File::open("Example.class")?);
//! let class = Class::from_reader(&mut reader)?;
//! for method in class.methods.iter().filter(|method| method.body.is_some()) {
//!     let ir = MokaIRMethod::from_method(method)?;
//!     for block in ir.blocks() {
//!         for operation in block.operations() {
//!             println!("{}: {operation}", operation.id());
//!         }
//!         println!("{}: {}", block.terminator().id(), block.terminator());
//!     }
//! }
//! # Ok(())
//! # }
//! ```

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
pub use generator::{MalformedBytecode, MokaIRBuildError, MokaIRFrameError, UnsupportedBytecode};
pub use identity::{BlockId, EdgeId, InstructionId, ValueId};
pub use method::{InstructionRef, MokaIRMethod};
pub use operation::{Operation, OperationKind};
pub use phi::{Phi, PhiInput, ValueDefinition};
pub use source_map::SourceMap;
pub use terminator::{Successor, Terminator, TerminatorKind};
