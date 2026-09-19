//! Scalar SSA intermediate representation of reachable JVM method behavior.
//!
//! Build a [`MokaIRMethod`] from a [`crate::jvm::Method`] with
//! [`MokaIRMethod::from_method`]. The result contains maximal [`BasicBlock`]s:
//! block-entry [`BlockParameter`]s, semantic [`Operation`]s in execution order, and
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
//! block parameters and other synthetic nodes may have no JVM origin.
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
//!     for (block_id, block) in ir.blocks() {
//!         for (index, operation) in block.operations.iter().enumerate() {
//!             println!("{block_id}, operation {index}: {operation}");
//!         }
//!         println!("{block_id}, terminator: {}", block.terminator);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

mod basic_block;
pub mod control_flow;
pub mod expression;
mod generator;
mod identity;
mod method;
mod operation;
#[cfg(feature = "petgraph")]
pub mod petgraph;
mod source_map;
mod terminator;
mod value_definition;
#[cfg(test)]
mod verify;

pub use basic_block::{BasicBlock, BlockKind, BlockParameter};
pub use generator::{MalformedBytecode, MokaIRBuildError, MokaIRFrameError, UnsupportedBytecode};
pub use identity::{BlockId, EdgeId, InstructionLocation, ValueId};
pub use method::{InstructionRef, MethodEntry, MokaIRMethod};
pub use operation::Operation;
pub use source_map::SourceMap;
pub use terminator::{Successor, Terminator};
pub use value_definition::ValueDefinition;
