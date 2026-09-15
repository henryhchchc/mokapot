//! Forms semantic maximal blocks from JVM instruction nodes and frame facts.

mod layout;
mod materialize;

use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            bytecode_analysis::{self, FrameMergeSite, jvm::Frame},
            error::Error,
            identity::SsaValueId,
        },
    },
    jvm::code::ProgramCounter,
};
use std::collections::BTreeMap;

use super::bytecode_analysis::NodeGraph;

use self::{layout::BlockLayout, materialize::materialize_blocks};

/// Forms maximal semantic blocks from completed JVM frame facts.
pub(super) fn form(node_graph: NodeGraph) -> Result<BlockGraph, Error> {
    let NodeGraph {
        entry_addr,
        initial_frame,
        nodes,
        phi_values,
        receiver_value,
        parameter_values,
    } = node_graph;
    let layout = BlockLayout::discover(nodes, entry_addr, initial_frame, &phi_values)?;
    // A site, its value, and its block are one relation: joining the layout's
    // sites here is what forms it, and SSA consumes it alone.
    let merges = layout
        .phi_blocks
        .iter()
        .map(|(&site, &block)| {
            let value = phi_values[&site];
            (site, FrameMerge { value, block })
        })
        .collect();
    let entry = layout.entry;
    let blocks = materialize_blocks(layout)?;
    debug_assert!(
        blocks
            .iter()
            .enumerate()
            .all(|(index, block)| usize::try_from(block.id.index()).is_ok_and(|id| id == index)),
        "a block id must index the blocks vector"
    );

    Ok(BlockGraph {
        entry,
        blocks,
        merges,
        this_value: receiver_value,
        parameter_values,
    })
}

/// A frame merge site resolved to the value that stands for it and the block
/// that computes it.
#[derive(Debug)]
pub(super) struct FrameMerge {
    pub(super) value: SsaValueId,
    pub(super) block: BlockId,
}

/// Block-level JVM graph consumed by SSA construction.
pub(super) struct BlockGraph {
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    /// The block and provisional value of every frame merge site.
    ///
    /// A site, its value, and the block that computes it are one relation, so
    /// it is carried once rather than recomposed.
    pub merges: BTreeMap<FrameMergeSite, FrameMerge>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// One exact outgoing edge from a formed JVM block.
#[derive(Debug)]
pub(crate) struct Arm {
    pub target: BlockId,
    pub transfer: ControlTransfer<bytecode_analysis::Value>,
    pub frame: Frame<bytecode_analysis::Value>,
}

/// The control-flow end of a formed JVM block.
///
/// A terminator's kind, source location, and ordered arms are one semantic
/// unit. Constructing this type validates the arm shape before SSA consumes
/// it, while retaining parallel arms and JVM handler precedence.
#[derive(Debug)]
pub(crate) struct BlockEnd {
    kind: TerminatorKind<bytecode_analysis::Value>,
    source: Option<ProgramCounter>,
    arms: Vec<Arm>,
}

impl BlockEnd {
    /// Creates a block end after checking that its terminator can own its arms.
    pub(crate) fn new(
        kind: TerminatorKind<bytecode_analysis::Value>,
        source: Option<ProgramCounter>,
        arms: Vec<Arm>,
    ) -> Result<Self, Error> {
        let end = Self { kind, source, arms };
        end.validate()?;
        Ok(end)
    }

    /// Borrows the ordered outgoing arms.
    pub(crate) fn arms(&self) -> &[Arm] {
        &self.arms
    }

    /// Separates the validated terminator data for scalar lowering.
    pub(crate) fn into_parts(
        self,
    ) -> (
        TerminatorKind<bytecode_analysis::Value>,
        Option<ProgramCounter>,
        Vec<Arm>,
    ) {
        (self.kind, self.source, self.arms)
    }

    /// Checks the control-transfer shapes emitted by bytecode analysis.
    ///
    /// The checks intentionally constrain transfer kinds and placement, not
    /// targets or arm multiplicity: switch arms may be parallel, and
    /// exceptional arms retain their source order.
    fn validate(&self) -> Result<(), Error> {
        use ControlTransfer::{Conditional, Exception, Unconditional, Unwind};

        let exceptional_arms = |arms: &[Arm]| {
            arms.iter()
                .position(|arm| matches!(arm.transfer, Unwind))
                .is_none_or(|unwind| unwind + 1 == arms.len())
                && arms
                    .iter()
                    .all(|arm| matches!(arm.transfer, Exception(_) | Unwind))
        };
        let valid = match &self.kind {
            TerminatorKind::Goto => {
                matches!(
                    self.arms.as_slice(),
                    [Arm {
                        transfer: Unconditional,
                        ..
                    }]
                )
            }
            TerminatorKind::Branch => {
                matches!(
                    self.arms.as_slice(),
                    [
                        Arm {
                            transfer: Conditional(_),
                            ..
                        },
                        Arm {
                            transfer: Conditional(_),
                            ..
                        }
                    ]
                )
            }
            TerminatorKind::Switch { .. } => {
                !self.arms.is_empty()
                    && self
                        .arms
                        .iter()
                        .all(|arm| matches!(arm.transfer, Conditional(_)))
            }
            TerminatorKind::Return(_) => exceptional_arms(&self.arms),
            TerminatorKind::Throw(_) => !self.arms.is_empty() && exceptional_arms(&self.arms),
            TerminatorKind::Fallible => {
                matches!(
                    self.arms.first(),
                    Some(Arm {
                        transfer: Unconditional,
                        ..
                    })
                ) && self.arms.len() > 1
                    && exceptional_arms(&self.arms[1..])
            }
            TerminatorKind::Unwind => self.arms.is_empty(),
        };
        valid
            .then_some(())
            .ok_or_else(|| Error::internal("a terminator has incompatible control-flow arms"))
    }
}

/// A maximal JVM block with exact register operands and outgoing frames.
#[derive(Debug)]
pub(crate) struct Block {
    pub id: BlockId,
    pub entry_frame: Frame<bytecode_analysis::Value>,
    pub operations: Vec<(ProgramCounter, OperationKind<bytecode_analysis::Value>)>,
    pub end: BlockEnd,
    pub caught_exception: Option<SsaValueId>,
}
