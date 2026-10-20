use derive_more::Display;

use super::ValueId;
use crate::jvm::references::FieldReference;

/// An operation on a field.
#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum Access {
    /// Reads a static field.
    #[display("read {field}")]
    ReadStatic {
        /// The field to read.
        field: FieldReference,
    },
    /// Writes to a static field.
    #[display("write {field}, {value}")]
    WriteStatic {
        /// The field to write to.
        field: FieldReference,
        /// The value to be written.
        value: ValueId,
    },
    /// Reads an instance field.
    #[display("read {object_ref}.{}", field.name)]
    ReadInstance {
        /// The object to read from.
        object_ref: ValueId,
        /// The field to read.
        field: FieldReference,
    },
    /// Writes to an instance field.
    #[display("write {object_ref}.{}, {value}", field.name)]
    WriteInstance {
        /// The object to write to.
        object_ref: ValueId,
        /// The field to write to.
        field: FieldReference,
        /// The value to be written.
        value: ValueId,
    },
}
