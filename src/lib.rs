#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]
#![doc = include_str!("../README.md")]

/// Module containing the APIs for static analysis.
pub mod analysis;
/// Module containing the APIs for the Moka IR.
pub mod ir;
/// Module containing the APIs for the JVM elements.
pub mod jvm;
pub(crate) mod macros;
/// Module containing the APIs for the JVM type system.
pub mod types;
pub(crate) mod utils;
