use std::fmt::{Display, Formatter};

use crate::{ir::Argument, jvm::field::FieldReference};

/// An operation on a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldAccess {
    /// Reads a static field.
    ReadStatic {
        /// The field to read.
        field: FieldReference,
    },
    /// Writes to a static field.
    WriteStatic {
        /// The field to write to.
        field: FieldReference,
        /// The value to be written.
        value: Argument,
    },
    /// Reads an instance field.
    ReadInstance {
        /// The object to read from.
        object_ref: Argument,
        /// The field to read.
        field: FieldReference,
    },
    /// Writes to an instance field.
    WriteInstance {
        /// The object to write to.
        object_ref: Argument,
        /// The field to write to.
        field: FieldReference,
        /// The value to be written.
        value: Argument,
    },
}

impl Display for FieldAccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadStatic { field } => write!(f, "read {field}"),
            Self::WriteStatic { field, value } => write!(f, "write {field}, {value}"),
            Self::ReadInstance { object_ref, field } => {
                write!(f, "read {}.{}", object_ref, field.name)
            }
            Self::WriteInstance {
                object_ref,
                field,
                value,
            } => write!(f, "write {}.{}, {}", object_ref, field.name, value),
        }
    }
}
