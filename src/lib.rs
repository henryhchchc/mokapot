#![warn(
    missing_debug_implementations,
    rust_2018_idioms,
    missing_docs,
    rust_2021_compatibility,
    future_incompatible
)]
//! # MokaPot
//! MokaPot is a Java bytecode analysis library written in Rust.

pub mod analysis;
pub mod ir;
pub mod jvm;
pub(crate) mod macros;
pub mod types;
pub(crate) mod utils;
