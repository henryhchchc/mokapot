//! Abstractly executes JVM frames to determine reachable states.

mod analyzer;
mod fact;
mod solver;

pub(in crate::ir::generator) use analyzer::JvmFrameAnalyzer;
pub(in crate::ir::generator) use fact::{
    AnalyzedJvmCfg, AnalyzedLocation, JvmOutgoing, MergeIdentity, OperandState,
};

#[cfg(test)]
mod tests;
