mod execution;
mod jvm_frame;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    iter::once,
    mem,
};

use itertools::Itertools;
use jvm_frame::Entry;
pub use jvm_frame::ExecutionError;

use self::jvm_frame::JvmStackFrame;
use super::{
    ControlFlowGraph, Identifier, MokaIRMethod, MokaInstruction, Operand,
    control_flow::{ControlTransfer, Edge},
    expression::Expression,
};
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::{
        control_flow::path_condition::{BooleanVariable, PathCondition, Value},
        expression::Condition,
    },
    jvm::{
        ConstantValue, Method,
        code::{ExceptionTableEntry, InstructionList, MethodBody, ProgramCounter},
        method,
        references::ClassRef,
    },
};

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
    body: &'m MethodBody,
    control_flow_edges: BTreeMap<(ProgramCounter, ProgramCounter), ControlTransfer>,
    initial_seed: Option<(ProgramCounter, JvmStackFrame)>,
}

impl DataflowProblem for MokaIRGenerator<'_> {
    type Location = ProgramCounter;
    type Fact = JvmStackFrame;
    type Err = MokaIRBrewingError;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        self.initial_seed.clone().into_iter().collect::<Vec<_>>()
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        let location = location.to_owned();
        let mut frame = fact.same_frame();
        let jvm_instruction = self
            .body
            .instruction_at(location)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
        let ir_instruction = self.run_instruction(jvm_instruction, location, &mut frame)?;
        let edges_and_frames =
            self.analyze_frame_and_conditions(location, frame, &ir_instruction)?;
        self.ir_instructions.insert(location, ir_instruction);

        let (affected_locations, edges): (Vec<_>, HashSet<_>) = edges_and_frames
            .into_iter()
            .map(|(edge, frame)| ((edge.target, frame), edge))
            .unzip();
        self.control_flow_edges.extend(edges.into_iter().map(
            |Edge {
                 source,
                 target,
                 data,
             }| ((source, target), data),
        ));
        Ok(affected_locations)
    }
}

impl<'m> MokaIRGenerator<'m> {
    fn next_pc_of(&self, pc: ProgramCounter) -> Result<ProgramCounter, MokaIRBrewingError> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBrewingError::MalformedControlFlow)
    }

    fn for_method(method: &'m Method) -> Result<Self, MokaIRBrewingError> {
        let body = method
            .body
            .as_ref()
            .ok_or(MokaIRBrewingError::NoMethodBody)?;
        let is_static = method.access_flags.contains(method::AccessFlags::STATIC);

        let first_pc = body
            .instructions
            .entry_point()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)?
            .0
            .to_owned();

        let initial_frame = JvmStackFrame::new(
            is_static,
            &method.descriptor,
            body.max_locals,
            body.max_stack,
        )?;

        Ok(Self {
            ir_instructions: BTreeMap::default(),
            body,
            control_flow_edges: BTreeMap::default(),
            initial_seed: Some((first_pc, initial_frame)),
        })
    }

    fn exception_edges(
        exception_table: &[ExceptionTableEntry],
        pc: ProgramCounter,
        frame: &JvmStackFrame,
    ) -> Vec<(Edge<ControlTransfer>, JvmStackFrame)> {
        exception_table
            .iter()
            .filter(|&it| it.covers(pc))
            .into_group_map_by(|&it| it.handler_pc)
            .into_iter()
            .map(|(handler_pc, entries)| {
                let caught_exception_ref = Operand::Just(Identifier::CaughtException(handler_pc));
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
                    Edge::new(pc, handler_pc, ControlTransfer::Exception(exceptions)),
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
        use crate::analysis::fixed_point::solve;

        let _facts: HashMap<_, _> = solve(&mut self)?;
        let cfg = ControlFlowGraph::from_edges(
            self.control_flow_edges
                .into_iter()
                .map(|((src, dst), trx)| (src, dst, trx)),
        );
        Ok((InstructionList::from(self.ir_instructions), cfg))
    }

    fn analyze_frame_and_conditions(
        &mut self,
        location: ProgramCounter,
        mut frame: JvmStackFrame,
        ir_instruction: &MokaInstruction,
    ) -> Result<Vec<(Edge<ControlTransfer>, JvmStackFrame)>, MokaIRBrewingError> {
        use ControlTransfer::{Conditional, SubroutineReturn, Unconditional};

        Ok(match ir_instruction {
            MokaInstruction::Nop => {
                let next_pc = self.next_pc_of(location)?;
                let edge = Edge::new(location, next_pc, Unconditional);
                vec![(edge, frame)]
            }
            MokaInstruction::Return(_) => Vec::new(),
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
                let edge = Edge::new(location, *target, Unconditional);
                vec![(edge, frame)]
            }
            MokaInstruction::Definition { .. } => {
                let next_pc = self.next_pc_of(location)?;
                Self::exception_edges(&self.body.exception_table, location, &frame)
                    .into_iter()
                    .chain(once((Edge::new(location, next_pc, Unconditional), frame)))
                    .collect()
            }
            MokaInstruction::Jump {
                condition: None,
                target,
            } => vec![(
                Edge::new(location, *target, Unconditional),
                frame.same_frame(),
            )],
            MokaInstruction::Jump {
                condition: Some(condition),
                target,
            } => {
                let cond: BooleanVariable<_> = condition.clone().into();
                let neg_cond = !cond.clone();
                let target_edge = {
                    let cond = PathCondition::of(cond);
                    Edge::new(location, *target, Conditional(cond))
                };
                let next_pc_edge = {
                    let neg_cond = PathCondition::of(neg_cond);
                    let next_pc = self.next_pc_of(location)?;
                    Edge::new(location, next_pc, Conditional(neg_cond))
                };
                vec![
                    (target_edge, frame.same_frame()),
                    (next_pc_edge, frame.same_frame()),
                ]
            }
            MokaInstruction::Switch {
                default,
                branches,
                match_value,
            } => {
                let default_cond = branches.keys().fold(PathCondition::one(), |acc, it| {
                    let val = ConstantValue::Integer(*it).into();
                    let not_equal_to_match_value = BooleanVariable::Negative(Condition::Equal(
                        match_value.clone().into(),
                        val,
                    ));
                    acc & not_equal_to_match_value
                });
                let default_edge = Edge::new(location, *default, Conditional(default_cond));
                let branch_edges = branches.iter().map(|(&val, &pc)| {
                    let val = Value::Constant(ConstantValue::Integer(val));
                    let cond = BooleanVariable::Positive(Condition::Equal(
                        match_value.clone().into(),
                        val,
                    ));
                    let edge = Edge::new(location, pc, Conditional(PathCondition::of(cond)));
                    (edge, frame.same_frame())
                });
                branch_edges
                    .chain(once((default_edge, frame.same_frame())))
                    .collect()
            }
            MokaInstruction::SubroutineRet(_) => mem::take(&mut frame.possible_ret_addresses)
                .into_iter()
                .map(|return_address| {
                    let edge = Edge::new(location, return_address, SubroutineReturn);
                    (edge, frame.same_frame())
                })
                .collect(),
        })
    }
}
