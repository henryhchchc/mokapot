use super::{BasicBlock, BlockId, InstructionLocation, InstructionRef, MokaIRMethod};
use crate::{
    ir::MokaIRBuildError,
    jvm::{Method, method},
};

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
        super::generator::generate(method)
    }

    /// Checks if the method is `static`.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.access_flags.contains(method::AccessFlags::STATIC)
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
}

#[cfg(feature = "petgraph")]
mod petgraph_impl {
    use crate::ir::{BlockId, MokaIRMethod, Successor};
    use petgraph::visit::{GraphBase, IntoNeighbors, Visitable};
    use std::collections::HashSet;

    impl GraphBase for MokaIRMethod {
        type EdgeId = (BlockId, BlockId);
        type NodeId = BlockId;
    }

    impl IntoNeighbors for &MokaIRMethod {
        type Neighbors = <Vec<BlockId> as IntoIterator>::IntoIter;

        fn neighbors(self, a: Self::NodeId) -> Self::Neighbors {
            let successors: Vec<_> = self
                .block(a)
                .into_iter()
                .flat_map(|bb| bb.terminator.arms().filter_map(Successor::block_target))
                .collect();
            successors.into_iter()
        }
    }

    impl Visitable for MokaIRMethod {
        type Map = HashSet<BlockId>;

        fn visit_map(&self) -> Self::Map {
            HashSet::new()
        }

        fn reset_map(&self, map: &mut Self::Map) {
            map.clear();
        }
    }
}
