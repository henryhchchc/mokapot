//! Symbolically executes JVM frames to determine reachable states.

mod analyzer;
mod fact;
mod solver;

pub(super) use analyzer::JvmSymbolicExecutor;
pub(crate) use fact::{
    FrameMergeSite, SymbolicJvmCfg, SymbolicJvmEdge, SymbolicJvmNode, SymbolicValue,
};

#[cfg(test)]
mod tests;
