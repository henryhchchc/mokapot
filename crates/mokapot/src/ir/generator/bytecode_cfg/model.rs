use std::collections::BTreeMap;

use crate::jvm::{code::ProgramCounter, references::ClassRef};

/// The value comparison performed by a conditional bytecode transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ir::generator) enum BranchPredicate {
    IsZero,
    IsNonZero,
    IsNegative,
    IsNonNegative,
    IsPositive,
    IsNonPositive,
    IsNull,
    IsNotNull,
    Equal,
    NotEqual,
    LessThan,
    GreaterThanOrEqual,
    GreaterThan,
    LessThanOrEqual,
}

impl BranchPredicate {
    pub(in crate::ir::generator) const fn operand_count(self) -> usize {
        match self {
            Self::IsZero
            | Self::IsNonZero
            | Self::IsNegative
            | Self::IsNonNegative
            | Self::IsPositive
            | Self::IsNonPositive
            | Self::IsNull
            | Self::IsNotNull => 1,
            Self::Equal
            | Self::NotEqual
            | Self::LessThan
            | Self::GreaterThanOrEqual
            | Self::GreaterThan
            | Self::LessThanOrEqual => 2,
        }
    }
}

/// The stack shape consumed by a method return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ir::generator) enum ReturnOperand {
    Void,
    Category1,
    Category2,
}

/// A dense identifier for a structural bytecode block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::ir::generator) struct StructuralBlockId(usize);

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
pub(in crate::ir::generator) struct HandlerId(usize);

impl HandlerId {
    pub(super) const fn from_index(index: usize) -> Self {
        Self(index)
    }

    pub(super) const fn index(self) -> usize {
        self.0
    }
}

/// The target of an exceptional structural control-flow edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ir::generator) enum ExceptionalTarget {
    /// A synthetic entry that installs the caught exception before its block.
    Handler(HandlerId),
    /// The synthetic exit for an exception that escapes the method.
    Unwind,
}

/// One synthetic exception-handler entry.
#[derive(Debug, Clone)]
pub(in crate::ir::generator) struct HandlerEntry {
    /// The handler's bytecode entry PC.
    pub handler_pc: ProgramCounter,
    /// The exception type selected by this table entry, or `None` for catch-all.
    pub catch_type: Option<ClassRef>,
    /// The decoded bytecode entered after materializing the caught exception.
    pub target: StructuralBlockId,
}

/// The ordinary transfer ending a structural block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ir::generator) enum StructuralTerminator {
    /// Ordinary execution continues at the following block.
    Fallthrough { target: StructuralBlockId },
    /// An unconditional static jump.
    Goto { target: StructuralBlockId },
    /// A conditional static jump and its required fallthrough.
    Branch {
        predicate: BranchPredicate,
        taken: StructuralBlockId,
        fallthrough: StructuralBlockId,
    },
    /// A static switch dispatch. Each key is retained, including coincident targets.
    Switch {
        cases: BTreeMap<i32, StructuralBlockId>,
        default: StructuralBlockId,
    },
    /// A legacy subroutine call and its static return continuation.
    Jsr {
        target: StructuralBlockId,
        continuation: StructuralBlockId,
    },
    /// A legacy subroutine return and its statically over-approximated continuations.
    Ret {
        local: u16,
        continuations: BTreeMap<ProgramCounter, StructuralBlockId>,
    },
    /// A normal method exit.
    Return { operand: ReturnOperand },
    /// An explicit `athrow`; exceptional successors select handlers or unwind.
    Throw,
}

/// A maximal bytecode block ending at an ordinary transfer or fallible instruction.
///
/// Its successor targets are fixed by construction: `terminator` and
/// `exceptional_successors` are derived from decoded bytecode alone, never from
/// an execution frame. Block analysis relies on this to keep the predecessors
/// of a location monotone; a terminator with frame-dependent targets would
/// invalidate that assumption.
#[derive(Debug, Clone)]
pub(in crate::ir::generator) struct Block {
    /// The first decoded instruction in the block.
    pub start_pc: ProgramCounter,
    /// Every raw bytecode PC belonging to the block, in bytecode order.
    pub instruction_pcs: Vec<ProgramCounter>,
    /// The ordinary transfer after the final instruction.
    pub terminator: StructuralTerminator,
    /// Ordered exceptional successors of the final fallible instruction.
    pub exceptional_successors: Vec<ExceptionalTarget>,
}

/// A block-first CFG that preserves decoded JVM bytecode structure.
#[derive(Debug, Clone)]
pub(in crate::ir::generator) struct BytecodeCfg {
    pub(super) entry: StructuralBlockId,
    pub(super) blocks: Vec<Block>,
    pub(super) handlers: Vec<HandlerEntry>,
}

impl BytecodeCfg {
    /// The block containing the first decoded instruction.
    pub const fn entry_block(&self) -> StructuralBlockId {
        self.entry
    }

    /// Returns all bytecode blocks in dense ID order.
    ///
    /// This is intentionally test-only: production consumers address blocks by
    /// identity or start PC, rather than depending on storage order.
    #[cfg(test)]
    pub(super) fn blocks(&self) -> impl ExactSizeIterator<Item = &Block> {
        self.blocks.iter()
    }

    /// Looks up a bytecode block by its dense identity.
    ///
    /// Blocks are stored in identity order and identities are never reordered or
    /// removed, so the lookup is positional: `id.index()` is the block's index.
    pub(in crate::ir::generator) fn block(&self, id: StructuralBlockId) -> Option<&Block> {
        self.blocks.get(id.index())
    }

    /// Looks up a bytecode block by its first instruction PC.
    #[cfg(test)]
    pub(super) fn block_at_pc(&self, pc: ProgramCounter) -> Option<&Block> {
        self.blocks.iter().find(|block| block.start_pc == pc)
    }

    /// Looks up the dense identity of the block starting at `pc`.
    ///
    /// An identity is the block's position, so this is the start PC's position
    /// among block starts.
    #[cfg(test)]
    pub(super) fn block_id_at_pc(&self, pc: ProgramCounter) -> Option<StructuralBlockId> {
        self.blocks
            .iter()
            .position(|block| block.start_pc == pc)
            .map(StructuralBlockId::from_index)
    }

    /// Looks up a synthetic handler entry by its dense identity.
    ///
    /// Entries are stored in identity order, so the lookup is positional:
    /// `id.index()` is the entry's index.
    pub(in crate::ir::generator) fn handler(&self, id: HandlerId) -> Option<&HandlerEntry> {
        self.handlers.get(id.index())
    }
}
