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
//! [`SourceMap`] records sparse, bidirectional JVM provenance: erased stack
//! operations may have no IR node, while synthetic nodes such as block
//! parameters may have no JVM origin.

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
/// Blocks and values have opaque identities local to this method. Instructions
/// are addressed by structural locations, and block terminators are the sole
/// source of control-flow edges.
#[derive(Debug, Clone)]
pub struct MokaIRMethod {
    /// The access flags of the method.
    pub access_flags: method::AccessFlags,
    /// The name of the method.
    pub name: String,
    /// The descriptor of the method.
    pub descriptor: MethodDescriptor,
    /// The class that owns the method.
    pub owner: ClassRef,
    /// The invocation of the entry block.
    pub entry: MethodEntry,
    /// The mapping between JVM bytecode and the IR.
    pub source_map: SourceMap,
    /// The value representing `this`, if this is an instance method.
    pub this: Option<ValueId>,
    /// The values representing the method parameters.
    pub parameters: Vec<ValueId>,
    blocks: HashMap<BlockId, BasicBlock>,
    value_definitions: HashMap<ValueId, ValueDefinition>,
}

/// A borrowed IR instruction resolved from an [`InstructionLocation`].
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
    /// The invoked entry block.
    pub block: BlockId,
    /// The values supplied to the entry block's parameters.
    pub arguments: Vec<ValueId>,
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
