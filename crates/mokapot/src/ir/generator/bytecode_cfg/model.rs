use std::collections::BTreeMap;

use derive_more::From;

use crate::jvm::{code::ProgramCounter, references::ClassRef};

/// The identity of a structural bytecode block.
///
/// The identity is minted from the program counter at which the block starts,
/// because a PC is unique and therefore yields distinct identities without a
/// lookup table. It is otherwise opaque: a block's position is read from
/// [`JvmBlock::start_pc`], never recovered from its identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From)]
pub(crate) struct StructuralBlockId(#[from] ProgramCounter);

/// The target of an exceptional structural control-flow edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExceptionalTarget {
    /// A synthetic entry that installs the caught exception before entering the
    /// block at `handler_pc`.
    Handler {
        /// The bytecode block the selected handler enters.
        block: StructuralBlockId,
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
/// It is keyed by its identity in [`JvmBlockGraph::blocks`]; its bytecode span
/// is carried by its own `start_pc` and `end_pc`.
#[derive(Debug, Clone)]
pub(crate) struct JvmBlock {
    /// The first decoded instruction in the block.
    pub start_pc: ProgramCounter,
    /// The final decoded instruction in the block.
    pub end_pc: ProgramCounter,
    /// The ordinary control-flow topology after the final instruction.
    pub exit: BlockExit,
    /// Ordered exceptional successors of the final fallible instruction.
    pub exception_handlers: Vec<ExceptionalTarget>,
}

/// A block-first CFG that preserves decoded JVM bytecode structure.
///
/// Blocks are keyed by an opaque identity, so no renumbering or PC-to-index
/// table is needed; bytecode positions live on [`JvmBlock`].
#[derive(Debug, Clone)]
pub(crate) struct JvmBlockGraph {
    /// The block containing the first decoded instruction.
    pub entry: StructuralBlockId,
    /// The blocks, keyed by identity.
    pub blocks: BTreeMap<StructuralBlockId, JvmBlock>,
}

impl JvmBlockGraph {
    /// The block containing the first decoded instruction.
    pub const fn entry_block(&self) -> StructuralBlockId {
        self.entry
    }

    /// Looks up a bytecode block by its identity.
    pub fn block(&self, id: StructuralBlockId) -> Option<&JvmBlock> {
        self.blocks.get(&id)
    }
}
