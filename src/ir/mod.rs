//! `MokaIR` is an intermediate representation of JVM bytecode.
//! It is register based and is in SSA form, which make it easier to analyze.

pub mod control_flow;
pub mod expression;
mod generator;
mod method;
mod moka_instruction;

pub use generator::{MokaIRGenerationError, MokaIRMethodExt};
pub use method::MokaIRMethod;
pub use moka_instruction::*;
