use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter},
};

use crate::{
    ir::{Argument, Identifier},
    jvm::references::FieldRef,
};

/// An operation on a field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// Reads a static field.
    ReadStatic {
        /// The field to read.
        field: FieldRef,
    },
    /// Writes to a static field.
    WriteStatic {
        /// The field to write to.
        field: FieldRef,
        /// The value to be written.
        value: Argument,
    },
    /// Reads an instance field.
    ReadInstance {
        /// The object to read from.
        object_ref: Argument,
        /// The field to read.
        field: FieldRef,
    },
    /// Writes to an instance field.
    WriteInstance {
        /// The object to write to.
        object_ref: Argument,
        /// The field to write to.
        field: FieldRef,
        /// The value to be written.
        value: Argument,
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

impl Display for Access {
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

#[cfg(test)]
mod tests {

    use crate::{ir::test::arb_argument, jvm::references::tests::arb_field_ref};

    use super::*;
    use proptest::prelude::*;

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
