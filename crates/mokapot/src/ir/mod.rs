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
pub mod control_flow;
pub mod expression;
mod generator;
mod identity;
mod operation;
mod source_map;
mod terminator;
#[cfg(test)]
mod test;
mod value_definition;

use std::collections::HashMap;

pub use basic_block::{BasicBlock, BlockKind, BlockParameter};
pub use generator::{MalformedBytecode, MokaIRBuildError, MokaIRFrameError, UnsupportedBytecode};
pub use identity::{BlockId, InstructionLocation, ValueId};
pub use operation::Operation;
pub use source_map::SourceMap;
pub use terminator::{Successor, Terminator};
pub use value_definition::ValueDefinition;

use crate::{
    ir::{
        control_flow::{
            Edge,
            path_condition::{PathCondition, SolvingBudget},
        },
        expression::Predicate,
    },
    jvm::{Method, method, references::ClassRef},
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

impl MokaIRMethod {
    /// Builds completed `MokaIR` from a JVM method.
    ///
    /// JVM stack and local state are eliminated during construction, trivial
    /// block parameters are simplified, and only reachable blocks are emitted.
    ///
    /// # Errors
    ///
    /// Returns [`MokaIRBuildError`] when the method has no body, uses unsupported
    /// bytecode, has invalid bytecode structure or reachable frame state, or an
    /// internal construction invariant is violated.
    pub fn from_method(method: &Method) -> Result<Self, MokaIRBuildError> {
        generator::generate(method)
    }

    /// Returns the method access flags.
    #[must_use]
    pub const fn access_flags(&self) -> method::AccessFlags {
        self.access_flags
    }

    /// Returns the method name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the method descriptor.
    #[must_use]
    pub const fn descriptor(&self) -> &MethodDescriptor {
        &self.descriptor
    }

    /// Returns the class containing this method.
    #[must_use]
    pub const fn owner(&self) -> &ClassRef {
        &self.owner
    }

    /// Checks if the method is `static`.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.access_flags.contains(method::AccessFlags::STATIC)
    }

    /// Returns the entry block identity.
    #[must_use]
    pub const fn entry_block(&self) -> BlockId {
        self.entry.target
    }

    /// Returns the method-entry invocation.
    #[must_use]
    pub const fn entry(&self) -> &MethodEntry {
        &self.entry
    }

    /// Looks up a block by its method-local identity.
    ///
    /// Identities outside this method's block set yield `None`.
    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(&id)
    }

    /// Resolves a block parameter, operation, or terminator by structural location.
    ///
    /// Locations outside this method's block structure yield `None`.
    #[must_use]
    pub fn instruction(&self, location: InstructionLocation) -> Option<InstructionRef<'_>> {
        Some(match location {
            InstructionLocation::BlockParameter { block, index } => {
                InstructionRef::BlockParameter(self.block(block)?.parameters.get(index)?)
            }
            InstructionLocation::Operation { block, index } => {
                InstructionRef::Operation(self.block(block)?.operations.get(index)?)
            }
            InstructionLocation::Terminator { block } => {
                InstructionRef::Terminator(&self.block(block)?.terminator)
            }
        })
    }

    /// Returns this method's source-provenance relation.
    #[must_use]
    pub const fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Returns the SSA value representing `this`, if this is an instance method.
    #[must_use]
    pub const fn this_value(&self) -> Option<ValueId> {
        self.this
    }

    /// Returns the SSA values representing method parameters in descriptor order.
    #[must_use]
    pub fn parameter_values(&self) -> &[ValueId] {
        &self.parameters
    }

    /// Returns the unique definition of a method-local SSA value.
    ///
    /// Value identities are opaque and may be sparse. An identity with no
    /// retained definition yields `None`.
    #[must_use]
    pub fn definition_of(&self, value: ValueId) -> Option<ValueDefinition> {
        self.value_definitions.get(&value).copied()
    }

    /// Returns all outgoing block-to-block successor arms from `source`.
    ///
    /// An identity outside this method yields no arms. Method-exiting unwind
    /// arms are not returned because they have no basic-block target.
    pub fn outgoing_edges(&self, source: BlockId) -> impl Iterator<Item = Edge<'_>> {
        control_flow::outgoing_edges(&self.blocks, source)
    }

    /// Computes path conditions at reachable blocks.
    #[must_use]
    pub fn path_conditions(&self) -> HashMap<BlockId, PathCondition<&Predicate>> {
        self.path_conditions_with_budget(SolvingBudget::default())
    }

    /// Computes path conditions with a custom minimization budget.
    #[must_use]
    pub fn path_conditions_with_budget(
        &self,
        budget: SolvingBudget,
    ) -> HashMap<BlockId, PathCondition<&Predicate>> {
        control_flow::path_condition::analyze(&self.blocks, self.entry.target, budget)
    }
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
    pub(super) target: BlockId,
    pub(super) arguments: Vec<ValueId>,
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
