use std::collections::HashSet;

use super::ValueId;
use crate::jvm::references::FieldRef;

/// An operation on a field.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Access {
    /// Reads a static field.
    #[display("read {field}")]
    ReadStatic {
        /// The field to read.
        field: FieldRef,
    },
    /// Writes to a static field.
    #[display("write {field}, {value}")]
    WriteStatic {
        /// The field to write to.
        field: FieldRef,
        /// The value to be written.
        value: ValueId,
    },
    /// Reads an instance field.
    #[display("read {object_ref}.{}", field.name)]
    ReadInstance {
        /// The object to read from.
        object_ref: ValueId,
        /// The field to read.
        field: FieldRef,
    },
    /// Writes to an instance field.
    #[display("write {object_ref}.{}, {value}", field.name)]
    WriteInstance {
        /// The object to write to.
        object_ref: ValueId,
        /// The field to write to.
        field: FieldRef,
        /// The value to be written.
        value: ValueId,
    },
}
impl Access {
    /// Returns the values used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::WriteStatic { value: u, .. } | Self::ReadInstance { object_ref: u, .. } => {
                HashSet::from([*u])
            }
            Self::WriteInstance {
                object_ref, value, ..
            } => HashSet::from([*object_ref, *value]),
            Self::ReadStatic { .. } => HashSet::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test::prelude::{field_ref, ids};

    #[test]
    fn uses_reports_field_receiver_and_written_value() {
        let [object_ref, value] = ids(0);
        let static_read = Access::ReadStatic { field: field_ref() };
        let instance_read = Access::ReadInstance {
            object_ref,
            field: field_ref(),
        };
        let static_write = Access::WriteStatic {
            field: field_ref(),
            value,
        };
        let instance_write = Access::WriteInstance {
            object_ref,
            field: field_ref(),
            value,
        };
        assert!(static_read.uses().is_empty());
        assert_eq!(instance_read.uses(), HashSet::from([object_ref]));
        assert_eq!(static_write.uses(), HashSet::from([value]));
        assert_eq!(instance_write.uses(), HashSet::from([object_ref, value]));
    }
}
