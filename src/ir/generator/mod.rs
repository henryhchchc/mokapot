mod execution;
mod jvm_frame;

use std::{
    collections::{BTreeMap, BTreeSet},
    iter::once,
    mem,
};

use crate::{
    ir::control_flow::path_condition::{PathCondition, Predicate},
    jvm::{
        ConstantValue, Method,
        code::{ExceptionTableEntry, InstructionList, MethodBody, ProgramCounter},
        method,
        references::ClassRef,
    },
};

use crate::analysis::fixed_point::Analyzer;

use self::jvm_frame::{Entry, JvmStackFrame};

use itertools::Itertools;
pub use jvm_frame::ExecutionError;

use super::{ControlFlowGraph, control_flow::ControlTransfer, expression::Expression};
use super::{Identifier, MokaIRMethod, MokaInstruction, Operand};

/// An error that occurs when generating Moka IR.
#[derive(Debug, thiserror::Error)]
pub enum MokaIRBrewingError {
    /// An error that occurs when executing bytecode on a JVM frame.
    #[error("Error when executing bytecode on a JVM frame: {0}")]
    ExecutionError(#[from] ExecutionError),
    /// An error that occurs when merging two stack frames.
    #[error("Error when merging two stack frames: {0}")]
    MergeError(ExecutionError),
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
    control_flow_edges: BTreeMap<(ProgramCounter, ProgramCounter), ControlTransfer>,
}

impl Analyzer for MokaIRGenerator<'_> {
    type Location = ProgramCounter;
    type Fact = JvmStackFrame;
    type Err = MokaIRBrewingError;
    type AffectedLocations = Vec<(Self::Location, Self::Fact)>;

    fn entry_fact(&self) -> Result<Self::AffectedLocations, Self::Err> {
        let first_pc = self
            .body
            .instructions
            .entry_point()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?
            .0
            .to_owned();
        JvmStackFrame::new(
            self.method
                .access_flags
                .contains(method::AccessFlags::STATIC),
            &self.method.descriptor,
            self.body.max_locals,
            self.body.max_stack,
        )
        .map(|frame| vec![(first_pc, frame)])
        .map_err(Into::into)
    }

    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::AffectedLocations, Self::Err> {
        use ControlTransfer::{Conditional, Unconditional};
        let location = location.to_owned();
        let mut frame = fact.same_frame();
        let insn = self
            .body
            .instruction_at(location)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let ir_instruction = self.run_instruction(insn, location, &mut frame)?;
        let edges_and_frames = match &ir_instruction {
            MokaInstruction::Nop => {
                let next_pc = self.next_pc_of(location)?;
                let edge = (location, next_pc, Unconditional);
                vec![(edge, frame)]
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
                let edge = (location, *target, Unconditional);
                vec![(edge, frame)]
            }
            MokaInstruction::Definition { .. } => {
                let next_pc = self.next_pc_of(location)?;
                Self::exception_edges(&self.body.exception_table, location, &frame)
                    .into_iter()
                    .chain(once(((location, next_pc, Unconditional), frame)))
                    .collect()
            }
            MokaInstruction::Jump { condition, target } => {
                if let Some(condition) = condition {
                    let cond: Predicate<_> = condition.clone().into();
                    let target_edge = (location, *target, Conditional(cond.clone().into()));
                    let next_pc = self.next_pc_of(location)?;
                    let next_pc_edge = (location, next_pc, Conditional((!cond).into()));
                    vec![
                        (target_edge, frame.same_frame()),
                        (next_pc_edge, frame.same_frame()),
                    ]
                } else {
                    vec![((location, *target, Unconditional), frame.same_frame())]
                }
            }
            MokaInstruction::Switch {
                default,
                branches,
                match_value,
            } => {
                let default_cond = PathCondition::conjunction_of(branches.keys().map(|it| {
                    let val = ConstantValue::Integer(*it).into();
                    Predicate::NotEqual(match_value.clone().into(), val)
                }));
                branches
                    .iter()
                    .map(|(&val, &pc)| {
                        let val = ConstantValue::Integer(val).into();
                        let cond = Predicate::Equal(match_value.clone().into(), val).into();
                        let edge = (location, pc, Conditional(cond));
                        (edge, frame.same_frame())
                    })
                    .chain(once((
                        (location, *default, Conditional(default_cond)),
                        frame.same_frame(),
                    )))
                    .collect()
            }
            MokaInstruction::SubroutineRet(_) => mem::take(&mut frame.possible_ret_addresses)
                .into_iter()
                .map(|return_address| {
                    let edge = (location, return_address, ControlTransfer::SubroutineReturn);
                    (edge, frame.same_frame())
                })
                .collect(),
        };
        self.ir_instructions.insert(location, ir_instruction);

        let (affected_locations, edges) = edges_and_frames
            .into_iter()
            .map(|(edge, frame)| ((edge.1, frame), edge))
            .unzip();
        self.control_flow_edges
            .extend(BTreeSet::into_iter(edges).map(|(src, tgt, ctr)| ((src, tgt), ctr)));
        Ok(affected_locations)
    }

    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err> {
        current_fact
            .merge(incoming_fact)
            .map_err(MokaIRBrewingError::MergeError)
    }
}

impl<'m> MokaIRGenerator<'m> {
    fn next_pc_of(&self, pc: ProgramCounter) -> Result<ProgramCounter, MokaIRBrewingError> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)
    }

    fn for_method(method: &'m Method) -> Result<Self, <Self as Analyzer>::Err> {
        let body = method
            .body
            .as_ref()
            .ok_or(MokaIRBrewingError::NoMethodBody)?;
        Ok(Self {
            ir_instructions: BTreeMap::default(),
            method,
            body,
            control_flow_edges: BTreeMap::default(),
        })
    }

    fn exception_edges(
        exception_table: &[ExceptionTableEntry],
        pc: ProgramCounter,
        frame: &JvmStackFrame,
    ) -> Vec<(
        (ProgramCounter, ProgramCounter, ControlTransfer),
        JvmStackFrame,
    )> {
        exception_table
            .iter()
            .filter(|&it| it.covers(pc))
            .into_group_map_by(|&it| it.handler_pc)
            .into_iter()
            .map(|(handler_pc, entries)| {
                let caught_exception_ref = Operand::Just(Identifier::CaughtException);
                let handler_frame =
                    frame.same_locals_1_stack_item_frame(Entry::Value(caught_exception_ref));
                let exceptions = entries
                    .into_iter()
                    .map(|it| {
                        it.catch_type
                            .clone()
                            .unwrap_or_else(|| ClassRef::new("java/lang/Throwable"))
                    })
                    .collect();
                (
                    (pc, handler_pc, ControlTransfer::Exception(exceptions)),
                    handler_frame,
                )
            })
            .collect()
    }
}

/// An extension trait for [`Method`] that generates Moka IR.
pub trait MokaIRMethodExt {
    /// Generates Moka IR for the method.
    /// # Errors
    /// See [`MokaIRBrewingError`] for more information.
    fn brew(&self) -> Result<MokaIRMethod, MokaIRBrewingError>;
}

impl MokaIRMethodExt for Method {
    fn brew(&self) -> Result<MokaIRMethod, MokaIRBrewingError> {
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
    ) -> Result<
        (
            InstructionList<MokaInstruction>,
            ControlFlowGraph<(), ControlTransfer>,
        ),
        MokaIRBrewingError,
    > {
        self.analyze()?;
        let cfg = ControlFlowGraph::from_edges(
            self.control_flow_edges
                .into_iter()
                .map(|((src, dst), trx)| (src, dst, trx)),
        );
        Ok((InstructionList::from(self.ir_instructions), cfg))
    }
}
