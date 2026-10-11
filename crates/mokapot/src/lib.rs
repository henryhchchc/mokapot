#![cfg_attr(docsrs, feature(doc_cfg))]

//! Welcome to `MokaPot`, a library to facilitate the analysis of JVM bytecode.
//! ## Features
#![doc = document_features::document_features!()]

pub mod analysis;

pub(crate) mod intrinsics;
pub mod ir;
pub mod jvm;
pub mod types;

/// Test utilities
#[cfg(test)]
pub mod tests;
