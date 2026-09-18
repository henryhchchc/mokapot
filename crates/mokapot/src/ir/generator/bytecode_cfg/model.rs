use std::collections::BTreeMap;

use crate::jvm::{code::ProgramCounter, references::ClassRef};

/// A dense identifier for a structural bytecode block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StructuralBlockId(usize);

impl StructuralBlockId {
    pub(super) const fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub(super) const fn index(self) -> usize {
        self.0
    }
}

/// A dense identifier for a synthetic exception-handler entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HandlerId(usize);

impl HandlerId {
    pub(super) const fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub(super) const fn index(self) -> usize {
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

/// One synthetic exception-handler entry.
#[derive(Debug, Clone)]
pub(crate) struct HandlerEntry {
    /// The decoded bytecode entered after materializing the caught exception.
    pub target: StructuralBlockId,
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
#[derive(Debug, Clone)]
pub(crate) struct Block {
    /// The first decoded instruction in the block.
    pub start_pc: ProgramCounter,
    /// The final decoded instruction in the block.
    pub end_pc: ProgramCounter,
    /// The ordinary control-flow topology after the final instruction.
    pub exit: BlockExit,
    /// Ordered exceptional successors of the final fallible instruction.
    pub exceptional_successors: Vec<ExceptionalTarget>,
}

/// A block-first CFG that preserves decoded JVM bytecode structure.
#[derive(Debug, Clone)]
pub(crate) struct BytecodeCfg {
    pub entry: StructuralBlockId,
    pub blocks: Vec<Block>,
    pub handlers: Vec<HandlerEntry>,
}

impl BytecodeCfg {
    /// The block containing the first decoded instruction.
    pub const fn entry_block(&self) -> StructuralBlockId {
        self.entry
    }

    /// Looks up a bytecode block by its dense identity.
    ///
    /// Blocks are stored in identity order and identities are never reordered or
    /// removed, so the lookup is positional: `id.index()` is the block's index.
    pub fn block(&self, id: StructuralBlockId) -> Option<&Block> {
        self.blocks.get(id.index())
    }

    /// Looks up a synthetic handler entry by its dense identity.
    ///
    /// Entries are stored in identity order, so the lookup is positional:
    /// `id.index()` is the entry's index.
    pub fn handler(&self, id: HandlerId) -> Option<&HandlerEntry> {
        self.handlers.get(id.index())
    }
}
