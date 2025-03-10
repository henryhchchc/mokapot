//! Module containing the APIs for the JVM type system.
pub mod field_type;
pub mod method_descriptor;

/// Trait for types that have a descriptor.
pub trait Descriptor {
    /// Returns the descriptor of the type.
    fn descriptor(&self) -> String;
}
