use std::collections::BTreeSet;

use crate::{
    ir::{Identifier, Operand},
    jvm::references::FieldRef,
};

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
        value: Operand,
    },
    /// Reads an instance field.
    #[display("read {object_ref}.{}", field.name)]
    ReadInstance {
        /// The object to read from.
        object_ref: Operand,
        /// The field to read.
        field: FieldRef,
    },
    /// Writes to an instance field.
    #[display("write {object_ref}.{}, {value}", field.name)]
    WriteInstance {
        /// The object to write to.
        object_ref: Operand,
        /// The field to write to.
        field: FieldRef,
        /// The value to be written.
        value: Operand,
    },
}
impl Access {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::WriteStatic { value: u, .. } | Self::ReadInstance { object_ref: u, .. } => {
                u.iter().copied().collect()
            }
            Self::WriteInstance {
                object_ref, value, ..
            } => object_ref.iter().chain(value.iter()).copied().collect(),
            Self::ReadStatic { .. } => BTreeSet::default(),
        }
    }
}

#[cfg(test)]
mod tests {

    use proptest::prelude::*;

    use super::*;
    use crate::{ir::test::arb_argument, jvm::references::tests::arb_field_ref};

    proptest! {

        #[test]
        fn uses(
            field in arb_field_ref(),
            object_ref in arb_argument(),
            value in arb_argument()
        ) {
            let value_ids = value.iter().copied().collect::<BTreeSet<_>>();
            let object_ref_ids = object_ref.iter().copied().collect::<BTreeSet<_>>();

            let read_static = Access::ReadStatic { field: field.clone() };
            assert!(read_static.uses().is_empty());

            let write_static = Access::WriteStatic {
                field: field.clone(),
                value: value.clone(),
            };
            assert_eq!(write_static.uses(), value_ids);

            let read_instance = Access::ReadInstance {
                object_ref: object_ref.clone(),
                field: field.clone(),
            };
            assert_eq!(read_instance.uses(), object_ref_ids);

            let write_instance = Access::WriteInstance {
                object_ref: object_ref.clone(),
                field: field.clone(),
                value: value.clone(),
            };
            assert_eq!(write_instance.uses(), value_ids.union(&object_ref_ids).copied().collect());
        }
    }
}
