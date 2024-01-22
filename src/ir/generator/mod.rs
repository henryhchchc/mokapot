mod execution;
mod jvm_frame;

use std::{collections::BTreeMap, iter::once, mem};

use crate::jvm::{
    code::{ExceptionTableEntry, InstructionList, MethodBody, ProgramCounter},
    method::{Method, MethodAccessFlags},
};

use crate::analysis::fixed_point::{self, FixedPointAnalyzer};

use self::jvm_frame::{Entry, JvmStackFrame};

pub use jvm_frame::JvmFrameError;

use super::expression::Expression;
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
}

impl FixedPointAnalyzer for MokaIRGenerator<'_> {
    type Location = ProgramCounter;
    type Fact = JvmStackFrame;
    type Err = MokaIRGenerationError;

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
    ) -> Result<BTreeMap<Self::Location, Self::Fact>, Self::Err> {
        let location = location.to_owned();
        let mut frame = fact.same_frame();
        let mut dirty_nodes = BTreeMap::new();
        let insn = self
            .body
            .instruction_at(location)
            .ok_or(MokaIRGenerationError::MalformedControlFlow)?;
        let ir_instruction = self.run_instruction(insn, location, &mut frame)?;
        match &ir_instruction {
            MokaInstruction::Nop => {
                let next_pc = self.next_pc_of(location)?;
                dirty_nodes.insert(next_pc, frame);
            }
            MokaInstruction::Definition {
                expr: Expression::Throw(_),
                ..
            }
            | MokaInstruction::Return(_) => {
                Self::add_exception_edges(
                    &self.body.exception_table,
                    location,
                    &frame,
                    &mut dirty_nodes,
                );
            }
            MokaInstruction::Definition {
                expr:
                    Expression::Subroutine {
                        target,
                        return_address,
                    },
                ..
            } => {
                frame.possible_ret_addresses.insert(*return_address);
                dirty_nodes.insert(*target, frame);
            }
            MokaInstruction::Definition { .. } => {
                let next_pc = self.next_pc_of(location)?;
                dirty_nodes.insert(next_pc, frame.same_frame());
                Self::add_exception_edges(
                    &self.body.exception_table,
                    location,
                    &frame,
                    &mut dirty_nodes,
                );
            }
            MokaInstruction::Jump { condition, target } => {
                if condition.is_some() {
                    let next_pc = self.next_pc_of(location)?;
                    dirty_nodes.insert(next_pc, frame.same_frame());
                    dirty_nodes.insert(*target, frame.same_frame());
                } else {
                    dirty_nodes.insert(*target, frame.same_frame());
                }
            }
            MokaInstruction::Switch {
                default, branches, ..
            } => branches.values().chain(once(default)).for_each(|&it| {
                dirty_nodes.insert(it, frame.same_frame());
            }),
            MokaInstruction::SubroutineRet(_) => mem::take(&mut frame.possible_ret_addresses)
                .into_iter()
                .for_each(|return_address| {
                    dirty_nodes.insert(return_address, frame.same_frame());
                }),
        }
        self.ir_instructions.insert(location, ir_instruction);
        Ok(dirty_nodes)
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
        })
    }

    fn add_exception_edges(
        exception_table: &[ExceptionTableEntry],
        pc: ProgramCounter,
        frame: &JvmStackFrame,
        dirty_nodes: &mut BTreeMap<ProgramCounter, JvmStackFrame>,
    ) {
        for handler in exception_table {
            if handler.covers(pc) {
                let caught_exception_ref = Argument::Id(Identifier::CaughtException);
                let handler_frame =
                    frame.same_locals_1_stack_item_frame(Entry::Value(caught_exception_ref));
                dirty_nodes.insert(handler.handler_pc, handler_frame);
            }
        }
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
        let instructions = MokaIRGenerator::for_method(self)?.generate()?;
        Ok(MokaIRMethod {
            access_flags: self.access_flags,
            name: self.name.clone(),
            owner: self.owner.clone(),
            descriptor: self.descriptor.clone(),
            instructions,
            exception_table: self.body.as_ref().unwrap().exception_table.clone(),
        })
    }
}

impl MokaIRGenerator<'_> {
    fn generate(mut self) -> Result<InstructionList<MokaInstruction>, MokaIRGenerationError> {
        fixed_point::analyze(&mut self)?;
        Ok(InstructionList::from(self.ir_instructions))
    }
}
