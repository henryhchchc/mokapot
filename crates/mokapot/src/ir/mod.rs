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
//! use std::{collections::HashSet, fs::File, io::BufReader};
//!
//! use mokapot::{ir::{MokaIRMethod, Successor}, jvm::Class};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut reader = BufReader::new(File::open("Example.class")?);
//! let class = Class::from_reader(&mut reader)?;
//! for method in class.methods.iter().filter(|method| method.body.is_some()) {
//!     let ir = MokaIRMethod::from_method(method)?;
//!     let mut pending = vec![ir.entry_block()];
//!     let mut visited = HashSet::new();
//!     while let Some(block_id) = pending.pop() {
//!         if !visited.insert(block_id) {
//!             continue;
//!         }
//!         let block = ir.block(block_id).expect("successor blocks belong to the method");
//!         for (index, operation) in block.operations.iter().enumerate() {
//!             println!("{block_id}, operation {index}: {operation}");
//!         }
//!         println!("{block_id}, terminator: {}", block.terminator);
//!         pending.extend(block.terminator.successors().filter_map(Successor::block_target));
//!     }
//! }
//! # Ok(())
//! # }
//! ```

mod basic_block;
pub mod expression;
mod generator;
mod identity;
mod ir_method;
mod operation;
pub mod path_condition;
mod source_map;
mod terminator;
#[cfg(test)]
mod test;

use std::collections::HashMap;

pub use basic_block::{BasicBlock, BlockKind, BlockParameter};
pub use generator::{
    MalformedControlFlow, MokaIRBuildError, MokaIRFrameError, UnsupportedBytecode,
};
pub use identity::{BlockId, InstructionLocation, ValueId};
pub use operation::Operation;
pub use source_map::SourceMap;
pub use terminator::{BranchGuard, ControlTransfer, Successor, Terminator};

use crate::{
    jvm::{method, references::ClassRef},
    types::method_descriptor::MethodDescriptor,
};

mod id_allocation;
use id_allocation::{IdAllocator, NumericalId};

/// A completed scalar-SSA representation of the reachable part of a JVM method.
///
/// Blocks and values have opaque identities local to this method. These
/// identities may be sparse and must not be interpreted as positions, counts,
/// or creation order.
/// Instructions are addressed by structural locations, and block terminators
/// are the sole source of control-flow edges.
#[derive(Debug, Clone)]
pub struct MokaIRMethod {
    access_flags: method::AccessFlags,
    name: String,
    descriptor: MethodDescriptor,
    owner: ClassRef,
    entry: MethodEntry,
    blocks: HashMap<BlockId, BasicBlock>,
    source_map: SourceMap,
    this: Option<ValueId>,
    parameters: Vec<ValueId>,
    value_definitions: HashMap<ValueId, ValueDefinition>,
}

/// A borrowed IR instruction resolved from an [`InstructionLocation`].
///
/// This enum preserves which kind of instruction was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionRef<'method> {
    /// A parameter bound on block entry.
    BlockParameter(&'method BlockParameter),
    /// An ordinary operation.
    Operation(&'method Operation),
    /// A block terminator.
    Terminator(&'method Terminator),
}

/// The invocation boundary that supplies arguments to the method's entry block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodEntry {
    target: BlockId,
    arguments: Vec<ValueId>,
}

impl MethodEntry {
    /// Returns the invoked entry block.
    #[must_use]
    pub const fn target(&self) -> BlockId {
        self.target
    }

    /// Returns the values supplied to the entry block's parameters.
    #[must_use]
    pub fn arguments(&self) -> &[ValueId] {
        &self.arguments
    }
}

/// Describes where a scalar value is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueDefinition {
    /// The receiver of an instance method.
    This,
    /// A method parameter at the given parameter index.
    Parameter(u16),
    /// The exception introduced by a landing-pad block.
    CaughtException(BlockId),
    /// A value produced by a block parameter, operation, or terminator.
    Instruction(InstructionLocation),
}
