mod execution;
mod stack_frame;

#[cfg(test)]
mod test;

use std::collections::{HashMap, HashSet};

use crate::elements::{
    instruction::{ExceptionTableEntry, ProgramCounter},
    Method,
};

use crate::analysis::fixed_point::{self, FixedPointAnalyzer};

use self::stack_frame::{FrameValue, StackFrame};

use super::{Identifier, MokaIRMethod, MokaInstruction, ValueRef};

#[derive(Debug, thiserror::Error)]
pub enum MokaIRGenerationError {
    #[error("Trying to pop an empty stack")]
    StackUnderflow,
    #[error("The stack size exceeds the max stack size")]
    StackOverflow,
    #[error("Expected a ValueRef but got a Padding or vice versa")]
    ValueMismatch,
    #[error("The local variable index exceeds the max local variable size")]
    LocalLimitExceed,
    #[error("The local variable is not initialized")]
    LocalUnset,
    #[error("The stack limit mismatch")]
    StackSizeMismatch,
    #[error("The local limit mismatch")]
    LocalLimitMismatch,
}

struct MokaIRGenerator<'m> {
    ir_instructions: HashMap<ProgramCounter, MokaInstruction>,
    method: &'m Method,
}

impl FixedPointAnalyzer for MokaIRGenerator<'_> {
    type Location = ProgramCounter;
    type Fact = StackFrame;
    type Error = MokaIRGenerationError;

    fn entry_fact(&self) -> Result<(Self::Location, Self::Fact), Self::Error> {
        let first_pc = self
            .method
            .body
            .as_ref()
            .expect("TODO")
            .instructions
            .first()
            .expect("TODO")
            .0;
        Ok((first_pc, StackFrame::new(self.method)))
    }

    fn execute_instruction(
        &mut self,
        location: Self::Location,
        fact: &Self::Fact,
    ) -> Result<HashMap<Self::Location, Self::Fact>, Self::Error> {
        let mut frame = fact.same_frame();
        let mut dirty_nodes = HashMap::new();
        let body = self.method.body.as_ref().expect("TODO");
        let insn = body.instruction_at(location).expect("TODO: Raise error");
        let ir_instruction = self.run_instruction(insn, location, &mut frame)?;
        match &ir_instruction {
            MokaInstruction::Nop => {
                let next_pc = body.next_pc_of(location).expect("Cannot get next pc");
                dirty_nodes.insert(next_pc, frame.same_frame());
            }
            MokaInstruction::Assignment { .. } | MokaInstruction::SideEffect(_) => {
                let next_pc = body.next_pc_of(location).expect("Cannot get next pc");
                dirty_nodes.insert(next_pc, frame.same_frame());
                self.add_exception_edges(&body.exception_table, location, &frame, &mut dirty_nodes);
            }
            MokaInstruction::Jump { condition, target } => {
                if condition.is_some() {
                    let next_pc = body.next_pc_of(location).expect("Cannot get next pc");
                    dirty_nodes.insert(next_pc, frame.same_frame());
                    dirty_nodes.insert(*target, frame.same_frame());
                } else {
                    dirty_nodes.insert(*target, frame.same_frame());
                }
            }
            MokaInstruction::Switch {
                default, branches, ..
            } => {
                branches.iter().for_each(|(_, it)| {
                    dirty_nodes.insert(*it, frame.same_frame());
                });
                dirty_nodes.insert(*default, frame.same_frame());
            }
            MokaInstruction::Return { .. } => {
                self.add_exception_edges(&body.exception_table, location, &frame, &mut dirty_nodes);
            }
            MokaInstruction::SubroutineRet(_) => {
                let reachable_subroutines = frame.reachable_subroutines;
                frame.reachable_subroutines = HashSet::new();
                reachable_subroutines.into_iter().for_each(|it| {
                    let next_pc = body.next_pc_of(it).expect("Cannot get next pc");
                    dirty_nodes.insert(next_pc, frame.same_frame());
                })
            }
        }
        self.ir_instructions.insert(location, ir_instruction);
        Ok(dirty_nodes)
    }
}

impl<'m> MokaIRGenerator<'m> {
    fn for_method(method: &'m Method) -> Self {
        Self {
            ir_instructions: Default::default(),
            method,
        }
    }

    fn add_exception_edges(
        &mut self,
        exception_table: &Vec<ExceptionTableEntry>,
        pc: ProgramCounter,
        frame: &StackFrame,
        dirty_nodes: &mut HashMap<ProgramCounter, StackFrame>,
    ) {
        for handler in exception_table.iter() {
            if handler.covers(pc) {
                let caught_exception_ref = ValueRef::Def(Identifier::CaughtException);
                let handler_frame = frame
                    .same_locals_1_stack_item_frame(FrameValue::ValueRef(caught_exception_ref));
                dirty_nodes.insert(handler.handler_pc, handler_frame);
            }
        }
    }
}

pub trait MokaIRMethodExt {
    /// Genreates Moka IR for the method.
    fn generate_moka_ir(&self) -> Result<MokaIRMethod, MokaIRGenerationError>;
}

impl MokaIRMethodExt for Method {
    fn generate_moka_ir(&self) -> Result<MokaIRMethod, MokaIRGenerationError> {
        let instructions = MokaIRGenerator::for_method(self).generate()?;
        Ok(MokaIRMethod {
            access_flags: self.access_flags,
            name: self.name.clone(),
            descriptor: self.descriptor.clone(),
            instructions,
        })
    }
}

impl MokaIRGenerator<'_> {
    fn generate(self) -> Result<HashMap<ProgramCounter, MokaInstruction>, MokaIRGenerationError> {
        let mut self_mut = self;
        fixed_point::analyze(&mut self_mut)?;
        Ok(self_mut.ir_instructions)
    }
}
