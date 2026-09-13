//! Symbolically executes JVM frames to determine reachable states.

mod analyzer;
mod fact;
mod solver;

pub(super) use analyzer::JvmSymbolicExecutor;
pub(crate) use fact::{AnalyzedJvmCfg, AnalyzedLocation, JvmOutgoing, MergeIdentity, OperandState};

#[cfg(test)]
mod tests;
