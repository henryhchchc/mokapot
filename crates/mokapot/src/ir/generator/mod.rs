//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`jvm_frame_analysis`] produces a reachable JVM control-flow graph with
//!    abstract frame facts.
//! 2. [`block_formation`] consumes that graph and produces a block-level JVM graph.
//! 3. [`ssa`] consumes the block graph, replays each block with exact values,
//!    constructs and simplifies predecessor-indexed phis, and lowers the result
//!    to scalar operations and explicit terminators without JVM frame state.
//! 4. [`emission`] consumes the lowered SSA graph, assigns public identities, and emits
//!    the completed [`MokaIRMethod`].
//!
//! The [`lifting`] module contains JVM opcode semantics shared by frame analysis
//! and SSA construction.

mod block_formation;
mod emission;
mod error;
mod identity;
mod instruction;
mod jvm_frame;
mod jvm_frame_analysis;
mod lifting;
mod normalized_jvm;
mod ssa;

use std::collections::{BTreeMap, BTreeSet};

pub use error::MokaIRBuildError;
use jvm_frame::Entry;
pub use jvm_frame::ExecutionError;

use self::identity::SsaValueId;
use self::instruction::Instruction;
use self::jvm_frame_analysis::operand_state::OperandState;
use self::jvm_frame_analysis::{
    AnalyzedJvmCfg, JvmFrameAnalyzer, JvmReplayPlan, JvmTransferCategory,
};
use self::lifting::frame_operand::FrameOperand;
use self::ssa::value::SsaFrameValue;

use self::jvm_frame::JvmStackFrame;
use self::normalized_jvm::{Location, NormalizedJvm, Normalizer, ReturnAddress};
use super::{
    BasicBlock, BlockId, EdgeId, InstructionId, MokaIRMethod, Operation, OperationKind, Phi,
    PhiInput, SourceMap, Successor, Terminator, TerminatorKind, ValueDefinition, ValueId,
    control_flow::ControlTransfer,
};
use crate::{
    analysis::fixed_point::DataflowProblem,
    jvm::{
        Method,
        code::{MethodBody, ProgramCounter},
        method,
    },
};

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let analyzed_cfg = JvmFrameAnalyzer::for_method(method)?.run()?;
    let block_graph = block_formation::form(analyzed_cfg)?;
    let ssa_graph = ssa::construct(method, block_graph)?;
    emission::emit(method, ssa_graph)
}

#[cfg(test)]
mod tests;
