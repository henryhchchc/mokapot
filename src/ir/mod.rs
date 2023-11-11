use std::collections::{BTreeMap, HashMap, HashSet};

use crate::elements::{
    instruction::{Instruction, MethodBody, ProgramCounter},
    Method, MethodAccessFlags, MethodDescriptor,
};
mod execution;
mod expression;
mod moka_instruction;
mod stack_frame;

pub use expression::*;
pub use moka_instruction::*;

#[cfg(test)]
mod test;

use self::stack_frame::StackFrame;

use crate::analysis::jvm_fixed_point::{self, FixedPointAnalyzer};

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

struct MokaIRGenerator {
    ir_instructions: HashMap<ProgramCounter, MokaInstruction>,
}

impl FixedPointAnalyzer for MokaIRGenerator {
    type Fact = StackFrame;
    type Error = MokaIRGenerationError;

    fn entry_fact(&self, method: &Method) -> StackFrame {
        StackFrame::new(method)
    }

    fn execute_instruction(
        &mut self,
        body: &MethodBody,
        pc: ProgramCounter,
        insn: &Instruction,
        fact: &StackFrame,
    ) -> Result<BTreeMap<ProgramCounter, StackFrame>, MokaIRGenerationError> {
        let mut frame = fact.same_frame();
        let mut dirty_pcs = BTreeMap::new();
        self.run_instruction(insn, pc, &mut frame)?;
        use Instruction::*;
        match insn {
            IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
            | IfLe(target) | IfICmpEq(target) | IfICmpNe(target) | IfICmpLt(target)
            | IfICmpGe(target) | IfICmpGt(target) | IfICmpLe(target) | IfACmpEq(target)
            | IfACmpNe(target) | IfNull(target) | IfNonNull(target) => {
                let next_pc = body.next_pc_of(pc).expect("Cannot get next pc");
                dirty_pcs.insert(*target, frame.same_frame());
                dirty_pcs.insert(next_pc, frame.same_frame());
            }
            Goto(target) | GotoW(target) => {
                dirty_pcs.insert(*target, frame.same_frame());
            }
            TableSwitch {
                default,
                jump_targets,
                ..
            } => {
                jump_targets.iter().for_each(|it| {
                    dirty_pcs.insert(*it, frame.same_frame());
                });
                dirty_pcs.insert(*default, frame.same_frame());
            }
            LookupSwitch {
                default,
                match_targets,
            } => {
                match_targets.iter().for_each(|(_, it)| {
                    dirty_pcs.insert(*it, frame.same_frame());
                });
                dirty_pcs.insert(*default, frame.same_frame());
            }
            Jsr(target) | JsrW(target) => {
                frame.reachable_subroutines.insert(*target);
                dirty_pcs.insert(*target, frame.same_frame());
            }
            AThrow => {}
            Ret(_) | WideRet(_) => {
                let reachable_subroutines = frame.reachable_subroutines;
                frame.reachable_subroutines = HashSet::new();
                reachable_subroutines.into_iter().for_each(|it| {
                    let next_pc = body.next_pc_of(it).expect("Cannot get next pc");
                    dirty_pcs.insert(next_pc, frame.same_frame());
                })
            }
            Return | AReturn | IReturn | LReturn | FReturn | DReturn => {}
            _ => {
                let next_pc = body.next_pc_of(pc).expect("Cannot get next pc");
                dirty_pcs.insert(next_pc, frame.same_frame());
            }
        }
        for handler in body.exception_table.iter() {
            if handler.covers(pc) {
                let handler_frame =
                    frame.same_locals_1_stack_item_frame(Identifier::CaughtException.into());
                dirty_pcs.insert(handler.handler_pc, handler_frame);
            }
        }

        Ok(dirty_pcs)
    }
}

impl MokaIRGenerator {
    fn new() -> Self {
        Self {
            ir_instructions: Default::default(),
        }
    }
}

pub struct MokaIRMethod {
    pub access_flags: MethodAccessFlags,
    pub name: String,
    pub descriptor: MethodDescriptor,
    pub instructions: HashMap<ProgramCounter, MokaInstruction>,
}

pub trait MokaIRMethodExt {
    /// Genreates Moka IR for the method.
    fn generate_moka_ir(&self) -> Result<MokaIRMethod, MokaIRGenerationError>;
}

impl MokaIRMethodExt for Method {
    fn generate_moka_ir(&self) -> Result<MokaIRMethod, MokaIRGenerationError> {
        let instructions = MokaIRGenerator::new().generate(self)?;
        Ok(MokaIRMethod {
            access_flags: self.access_flags,
            name: self.name.clone(),
            descriptor: self.descriptor.clone(),
            instructions,
        })
    }
}

impl MokaIRGenerator {
    fn generate(
        self,
        method: &Method,
    ) -> Result<HashMap<ProgramCounter, MokaInstruction>, MokaIRGenerationError> {
        let mut self_mut = self;
        jvm_fixed_point::analyze(method, &mut self_mut)?;
        Ok(self_mut.ir_instructions)
    }
}
