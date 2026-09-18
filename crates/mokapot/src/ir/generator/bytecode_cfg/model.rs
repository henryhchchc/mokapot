use std::collections::BTreeMap;

use crate::jvm::{code::ProgramCounter, references::ClassRef};

/// The identity of a structural bytecode block: the program counter of its
/// first instruction.
///
/// A block's start PC is unique and stable, so a target PC *is* the identity of
/// the block it starts, and successors resolve without a lookup table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StructuralBlockId(ProgramCounter);

impl StructuralBlockId {
    pub(crate) const fn from_pc(pc: ProgramCounter) -> Self {
        Self(pc)
    }

    pub(crate) const fn pc(self) -> ProgramCounter {
        self.0
    }
}

/// The identity of a synthetic exception-handler entry: the handler's program
/// counter.
///
/// Distinct handler PCs are distinct entries, so exception-table arms that
/// share a handler share its identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HandlerId(ProgramCounter);

impl HandlerId {
    pub(crate) const fn from_pc(pc: ProgramCounter) -> Self {
        Self(pc)
    }

    pub(crate) const fn pc(self) -> ProgramCounter {
        self.0
    }
}

/// The target of an exceptional structural control-flow edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExceptionalTarget {
    /// A synthetic entry that installs the caught exception before its block.
    Handler {
        /// The handler entry selected by this exception-table arm.
        id: HandlerId,
        /// The exception type selected by this arm, or `None` for catch-all.
        catch_type: Option<ClassRef>,
    },
    /// The synthetic exit for an exception that escapes the method.
    Unwind,
}

/// The control-flow topology at the end of a structural block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlockExit {
    /// Ordinary execution continues at the following block.
    Fallthrough { target: StructuralBlockId },
    /// An unconditional static jump.
    Goto { target: StructuralBlockId },
    /// A conditional static jump and its required fallthrough.
    Branch {
        taken: StructuralBlockId,
        fallthrough: StructuralBlockId,
    },
    /// A static switch dispatch. Each key is retained, including coincident targets.
    Switch {
        cases: BTreeMap<i32, StructuralBlockId>,
        default: StructuralBlockId,
    },
    /// A return or explicit throw with no ordinary successor.
    Terminal,
}

/// A maximal bytecode block ending at an ordinary transfer or fallible instruction.
///
/// Its start PC is its key in [`BytecodeCfg::blocks`].
#[derive(Debug, Clone)]
pub(crate) struct Block {
    /// The final decoded instruction in the block.
    pub end_pc: ProgramCounter,
    /// The ordinary control-flow topology after the final instruction.
    pub exit: BlockExit,
    /// Ordered exceptional successors of the final fallible instruction.
    pub exceptional_successors: Vec<ExceptionalTarget>,
}

/// A block-first CFG that preserves decoded JVM bytecode structure.
///
/// Blocks are keyed by their start PC, which is also their identity, so no
/// renumbering or PC-to-index table is needed.
#[derive(Debug, Clone)]
pub(crate) struct BytecodeCfg {
    /// The block containing the first decoded instruction.
    pub entry: StructuralBlockId,
    /// The blocks, keyed by start PC.
    pub blocks: BTreeMap<ProgramCounter, Block>,
}

impl BytecodeCfg {
    /// The block containing the first decoded instruction.
    pub const fn entry_block(&self) -> StructuralBlockId {
        self.entry
    }

    /// Looks up a bytecode block by its identity, which is its start PC.
    pub fn block(&self, id: StructuralBlockId) -> Option<&Block> {
        self.blocks.get(&id.pc())
    }
}
