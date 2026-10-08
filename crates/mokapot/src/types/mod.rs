//! Module containing the APIs for the JVM type system.
pub mod class_name;
pub mod field_type;
pub mod method_descriptor;
mod name_validation;
pub mod package_name;
pub mod reference_type;

/// Trait for types that have a descriptor.
pub trait Descriptor {
    /// Returns the descriptor of the type.
    fn descriptor(&self) -> String;
}
