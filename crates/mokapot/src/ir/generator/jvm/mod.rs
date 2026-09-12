//! JVM-specific normalization, frame analysis, and instruction lifting.

pub(in crate::ir::generator) mod analysis;
pub(in crate::ir::generator) mod frame;
pub(in crate::ir::generator) mod instruction;
pub(in crate::ir::generator) mod lifting;
pub(in crate::ir::generator) mod normalization;
