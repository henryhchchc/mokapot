#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
//! # MokaPot
//! MokaPot is a Java bytecode analysis library written in Rust.

pub mod analysis;
pub mod ir;
pub mod jvm;
pub(crate) mod macros;
/// Module containing the APIs for the JVM type system.
pub mod types;
pub(crate) mod utils;
