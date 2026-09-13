//! Symbolically executes JVM frames to determine reachable states.

mod analyzer;
mod fact;
mod solver;

pub(super) use analyzer::Executor;
pub(crate) use fact::{Cfg, Edge, FrameMergeSite, Node, Value};

#[cfg(test)]
mod tests;
