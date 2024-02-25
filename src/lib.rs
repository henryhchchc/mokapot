#![warn(
    missing_debug_implementations,
    rust_2018_idioms,
    missing_docs,
    rust_2021_compatibility,
    future_incompatible,
    clippy::pedantic
)]
#![allow(clippy::module_name_repetitions)]
#![cfg_attr(unstable, feature(doc_auto_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]

//! Welcome to `MokaPot`, a library to faciliate the analysis of JVM bytecode.
//! ## Features
#![doc = document_features::document_features!()]

pub mod analysis;

pub mod ir;
pub mod jvm;
pub(crate) mod macros;
pub mod types;

#[cfg(test)]
pub mod tests;
