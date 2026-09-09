use std::collections::HashSet;

use crate::{ir::ValueId, jvm::references::FieldRef};

/// An operation on a field.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Access<OP: std::fmt::Display = ValueId> {
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
        value: OP,
    },
    /// Reads an instance field.
    #[display("read {object_ref}.{}", field.name)]
    ReadInstance {
        /// The object to read from.
        object_ref: OP,
        /// The field to read.
        field: FieldRef,
    },
    /// Writes to an instance field.
    #[display("write {object_ref}.{}, {value}", field.name)]
    WriteInstance {
        /// The object to write to.
        object_ref: OP,
        /// The field to write to.
        field: FieldRef,
        /// The value to be written.
        value: OP,
    },
}
impl Access<ValueId> {
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

    use proptest::prelude::*;

    use super::*;
    use crate::jvm::references::tests::arb_field_ref;

    proptest! {

        #[test]
        fn uses(
            field in arb_field_ref(),
            object_ref in any::<ValueId>(),
            value in any::<ValueId>()
        ) {
            let value_ids = HashSet::from([value]);
            let object_ref_ids = HashSet::from([object_ref]);

            let read_static = Access::ReadStatic { field: field.clone() };
            assert!(read_static.uses().is_empty());

            let write_static = Access::WriteStatic {
                field: field.clone(),
                value,
            };
            assert_eq!(write_static.uses(), value_ids);

            let read_instance = Access::ReadInstance {
                object_ref,
                field: field.clone(),
            };
            assert_eq!(read_instance.uses(), object_ref_ids);

            let write_instance = Access::WriteInstance {
                object_ref,
                field: field.clone(),
                value,
            };
            assert_eq!(write_instance.uses(), value_ids.union(&object_ref_ids).copied().collect());
        }
    }
}
