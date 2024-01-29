//! Module containing the APIs for the Moka IR.
mod control_flow;
pub mod expression;
mod generator;
mod method;
mod moka_instruction;

pub use generator::{MokaIRGenerationError, MokaIRMethodExt};
pub use method::MokaIRMethod;
pub use moka_instruction::*;
