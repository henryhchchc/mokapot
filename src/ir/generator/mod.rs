mod execution;
mod jvm_frame;

use std::{
    collections::{BTreeMap, HashSet},
    iter::once,
    mem,
};

use crate::{
    ir::ControlFlowEdge,
    jvm::{
        class::ClassReference,
        code::{ExceptionTableEntry, InstructionList, MethodBody, ProgramCounter},
        method::{Method, MethodAccessFlags},
    },
};

use crate::analysis::fixed_point::{self, FixedPointAnalyzer};

use self::jvm_frame::{Entry, JvmStackFrame};

pub use jvm_frame::JvmFrameError;

use super::{expression::Expression, ControlFlowGraph};
use super::{Argument, Identifier, MokaIRMethod, MokaInstruction};

/// An error that occurs when generating Moka IR.
#[derive(Debug, thiserror::Error)]
pub enum MokaIRGenerationError {
    /// An error that occurs when executing bytecode on a JVM frame.
    #[error("Error when executing bytecode on a JVM frame: {0}")]
    ExecutionError(#[from] JvmFrameError),
    /// An error that occurs when merging two stack frames.
    #[error("Error when merging two stack frames: {0}")]
    MergeError(JvmFrameError),
    /// An error that occurs when a method does not have a body.
    #[error("The method does not have a body")]
    NoMethodBody,
    /// An error that occurs when the method contains malformed control flow.
    #[error("The method contains malformed control flow")]
    MalformedControlFlow,
}

struct MokaIRGenerator<'m> {
    ir_instructions: BTreeMap<ProgramCounter, MokaInstruction>,
    method: &'m Method,
    body: &'m MethodBody,
    control_flow_edges: HashSet<ControlFlowEdge>,
}

impl FixedPointAnalyzer for MokaIRGenerator<'_> {
    type Location = ProgramCounter;
    type Fact = JvmStackFrame;
    type Err = MokaIRGenerationError;
    type AffectedLocations = Vec<(Self::Location, Self::Fact)>;

    fn entry_fact(&self) -> Result<(Self::Location, Self::Fact), Self::Err> {
        let first_pc = self
            .body
            .instructions
            .entry_point()
            .ok_or(MokaIRGenerationError::MalformedControlFlow)?
            .0
            .to_owned();
        Ok((
            first_pc,
            JvmStackFrame::new(
                self.method.access_flags.contains(MethodAccessFlags::STATIC),
                self.method.descriptor.clone(),
                self.body.max_locals,
                self.body.max_stack,
            ),
        ))
    }

    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::AffectedLocations, Self::Err> {
        let location = location.to_owned();
        let mut frame = fact.same_frame();
        let insn = self
            .body
            .instruction_at(location)
            .ok_or(MokaIRGenerationError::MalformedControlFlow)?;
        let ir_instruction = self.run_instruction(insn, location, &mut frame)?;
        let edges_and_frames = match &ir_instruction {
            MokaInstruction::Nop => {
                let next_pc = self.next_pc_of(location)?;
                vec![(ControlFlowEdge::unconditional(location, next_pc), frame)]
            }
            MokaInstruction::Return(_) => Vec::default(),
            MokaInstruction::Definition {
                expr: Expression::Throw(_),
                ..
            } => Self::exception_edges(&self.body.exception_table, location, &frame),
            MokaInstruction::Definition {
                expr:
                    Expression::Subroutine {
                        target,
                        return_address,
                    },
                ..
            } => {
                frame.possible_ret_addresses.insert(*return_address);
                vec![(ControlFlowEdge::unconditional(location, *target), frame)]
            }
            MokaInstruction::Definition { .. } => {
                let next_pc = self.next_pc_of(location)?;
                Self::exception_edges(&self.body.exception_table, location, &frame)
                    .into_iter()
                    .chain(once((
                        ControlFlowEdge::unconditional(location, next_pc),
                        frame,
                    )))
                    .collect()
            }
            MokaInstruction::Jump { condition, target } => {
                let target_edge = if condition.is_some() {
                    ControlFlowEdge::conditional(location, *target)
                } else {
                    ControlFlowEdge::unconditional(location, *target)
                };
                if condition.is_some() {
                    let next_pc = self.next_pc_of(location)?;
                    let next_pc_edge = ControlFlowEdge::unconditional(location, next_pc);
                    vec![
                        (target_edge, frame.same_frame()),
                        (next_pc_edge, frame.same_frame()),
                    ]
                } else {
                    vec![(target_edge, frame.same_frame())]
                }
            }
            MokaInstruction::Switch {
                default, branches, ..
            } => branches
                .values()
                .chain(once(default))
                .map(|&it| {
                    let edge = ControlFlowEdge::unconditional(location, it);
                    (edge, frame.same_frame())
                })
                .collect(),
            MokaInstruction::SubroutineRet(_) => mem::take(&mut frame.possible_ret_addresses)
                .into_iter()
                .map(|return_address| {
                    let edge = ControlFlowEdge::subroutine_return(location, return_address);
                    (edge, frame.same_frame())
                })
                .collect(),
        };
        self.ir_instructions.insert(location, ir_instruction);

        let (affected_locations, edges) = edges_and_frames
            .into_iter()
            .map(|(edge, frame)| ((edge.dst, frame), edge))
            .unzip();
        let edges: HashSet<_> = edges;
        self.control_flow_edges.extend(edges);
        Ok(affected_locations)
    }

    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err> {
        current_fact
            .merge(incoming_fact)
            .map_err(MokaIRGenerationError::MergeError)
    }
}

impl<'m> MokaIRGenerator<'m> {
    fn next_pc_of(&self, pc: ProgramCounter) -> Result<ProgramCounter, MokaIRGenerationError> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRGenerationError::MalformedControlFlow)
    }

    fn for_method(method: &'m Method) -> Result<Self, <Self as FixedPointAnalyzer>::Err> {
        let body = method
            .body
            .as_ref()
            .ok_or(MokaIRGenerationError::NoMethodBody)?;
        Ok(Self {
            ir_instructions: BTreeMap::default(),
            method,
            body,
            // The number of control flow edges is at least `body.instructions.len() - 1` if there
            // is no deadcode.
            control_flow_edges: HashSet::with_capacity(body.instructions.len()),
        })
    }

    fn exception_edges(
        exception_table: &[ExceptionTableEntry],
        pc: ProgramCounter,
        frame: &JvmStackFrame,
    ) -> Vec<(ControlFlowEdge, JvmStackFrame)> {
        exception_table
            .iter()
            .filter(|it| it.covers(pc))
            .map(|handler| {
                let caught_exception_ref = Argument::Id(Identifier::CaughtException);
                let handler_frame =
                    frame.same_locals_1_stack_item_frame(Entry::Value(caught_exception_ref));
                let exception_ref = handler
                    .catch_type
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| ClassReference::new("java/lang/Throwable"));
                (
                    ControlFlowEdge::exception(pc, handler.handler_pc, exception_ref),
                    handler_frame,
                )
            })
            .collect()
    }
}

/// An extension trait for [`Method`] that generates Moka IR.
pub trait MokaIRMethodExt {
    /// Genreates Moka IR for the method.
    /// # Errors
    /// See [`MokaIRGenerationError`] for more information.
    fn generate_moka_ir(&self) -> Result<MokaIRMethod, MokaIRGenerationError>;
}

impl MokaIRMethodExt for Method {
    fn generate_moka_ir(&self) -> Result<MokaIRMethod, MokaIRGenerationError> {
        let (instructions, control_flow_graph) = MokaIRGenerator::for_method(self)?.generate()?;
        Ok(MokaIRMethod {
            access_flags: self.access_flags,
            name: self.name.clone(),
            owner: self.owner.clone(),
            descriptor: self.descriptor.clone(),
            instructions,
            exception_table: self.body.as_ref().unwrap().exception_table.clone(),
            control_flow_graph,
        })
    }
}

impl MokaIRGenerator<'_> {
    fn generate(
        mut self,
    ) -> Result<(InstructionList<MokaInstruction>, ControlFlowGraph), MokaIRGenerationError> {
        fixed_point::analyze(&mut self)?;
        let cfg = ControlFlowGraph::from_edges(self.control_flow_edges);
        Ok((InstructionList::from(self.ir_instructions), cfg))
    }
}
