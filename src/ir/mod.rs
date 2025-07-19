//! `MokaIR` is an intermediate representation of JVM bytecode.
//! It is register based and is in SSA form, which make it easier to analyze.

pub mod control_flow;
pub mod data_flow;
pub mod expression;
mod generator;
mod moka_instruction;
#[cfg(feature = "petgraph")]
pub mod petgraph;

pub mod type_hierarchy;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[cfg(feature = "unstable-moka-ir")]
pub use generator::{MokaIRBrewingError, MokaIRMethodExt};
#[cfg(not(feature = "unstable-moka-ir"))]
pub(crate) use generator::{MokaIRBrewingError, MokaIRMethodExt};
#[cfg(feature = "unstable-moka-ir")]
pub use moka_instruction::*;
#[cfg(not(feature = "unstable-moka-ir"))]
pub(crate) use moka_instruction::*;

use self::control_flow::ControlTransfer;
use crate::{
    jvm::{
        code::{ExceptionTableEntry, InstructionList, ProgramCounter},
        method::{self},
        references::ClassRef,
    },
    types::method_descriptor::MethodDescriptor,
};

/// Represents a JVM method where the instructions have been converted to Moka IR.
#[derive(Debug, Clone)]
#[instability::unstable(feature = "moka-ir")]
pub struct MokaIRMethod {
    /// The access flags of the method.
    pub access_flags: method::AccessFlags,
    /// The name of the method.
    pub name: String,
    /// The descriptor of the method.
    pub descriptor: MethodDescriptor,
    /// The class that contains the method.
    pub owner: ClassRef,
    /// The body of the method.
    pub instructions: InstructionList<MokaInstruction>,
    /// The exception table of the method.
    pub exception_table: Vec<ExceptionTableEntry>,
    /// The control flow graph of the method.
    pub control_flow_graph: ControlFlowGraph<(), ControlTransfer>,
}

impl MokaIRMethod {
    /// Checks if the method is `static`.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.access_flags.contains(method::AccessFlags::STATIC)
    }
}

/// A control flow graph.
///
/// It is generic over the data associated with each node and edge.
#[derive(Debug, Clone, Default)]
pub struct ControlFlowGraph<N, E> {
    inner: BTreeMap<ProgramCounter, (N, BTreeMap<ProgramCounter, E>)>,
}

/// A def-use chain in data flow analysis.
#[derive(Debug)]
#[instability::unstable(feature = "moka-ir")]
pub struct DefUseChain<'a> {
    method: &'a MokaIRMethod,
    defs: HashMap<LocalValue, ProgramCounter>,
    uses: HashMap<Identifier, BTreeSet<ProgramCounter>>,
}

/// A class hierarchy based on super class relationships.
#[derive(Debug, Clone)]
#[instability::unstable(feature = "project-analyses")]
pub struct ClassHierarchy {
    inheritance: HashMap<ClassRef, HashSet<ClassRef>>,
    super_classes: HashMap<ClassRef, ClassRef>,
}

/// A class hierarchy based on interface implementations.
#[derive(Debug, Clone)]
#[instability::unstable(feature = "project-analyses")]
pub struct InterfaceImplHierarchy {
    implementations: HashMap<ClassRef, HashSet<ClassRef>>,
    implementers: HashMap<ClassRef, HashSet<ClassRef>>,
}
