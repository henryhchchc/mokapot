//! Module containing the APIs for the JVM type system.
pub mod binary_name;
pub mod field_type;
pub mod method_descriptor;
pub mod reference_type;

/// Trait for types that have a descriptor.
pub trait Descriptor {
    /// Returns the descriptor of the type.
    fn descriptor(&self) -> String;
}
