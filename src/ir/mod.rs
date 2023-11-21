mod expression;
pub mod expressions;
mod generator;
mod moka_instruction;

use std::collections::HashMap;

pub use expression::*;
pub use generator::{MokaIRGenerationError, MokaIRMethodExt};
pub use moka_instruction::*;

use crate::elements::{
    instruction::ProgramCounter, references::ClassReference, MethodAccessFlags, MethodDescriptor,
};

pub struct MokaIRMethod {
    pub access_flags: MethodAccessFlags,
    pub name: String,
    pub descriptor: MethodDescriptor,
    pub owner: ClassReference,
    pub instructions: HashMap<ProgramCounter, MokaInstruction>,
}
