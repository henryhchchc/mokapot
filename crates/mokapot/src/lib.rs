#![warn(
    clippy::pedantic,
    future_incompatible,
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    rust_2021_compatibility
)]
// #![allow(clippy::module_name_repetitions)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]

//! Welcome to `MokaPot`, a library to facilitate the analysis of JVM bytecode.
//! ## Features
#![doc = document_features::document_features!()]

pub mod analysis;

pub(crate) mod intrinsics;
#[cfg(feature = "unstable-moka-ir")]
pub mod ir;
#[cfg(not(feature = "unstable-moka-ir"))]
#[cfg_attr(not(feature = "unstable-moka-ir"), expect(unused))]
pub(crate) mod ir;
pub mod jvm;
pub mod types;

/// Test utilities
#[cfg(test)]
pub mod tests;
