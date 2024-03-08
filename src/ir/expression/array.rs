use std::{collections::BTreeSet, fmt::Display};

use itertools::Itertools;

use crate::{
    ir::{Argument, Identifier},
    types::field_type::FieldType,
};

/// An operation on an array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Create a new array.
    New {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The length of the array.
        length: Argument,
    },
    /// Create a new multidimensional array.
    NewMultiDim {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The legths of each of the dimensions of the array.
        dimensions: Vec<Argument>,
    },
    /// Gets an element from an array.
    Read {
        /// The array to read from.
        array_ref: Argument,
        /// The index of the element to read.
        index: Argument,
    },
    /// Sets an element in an array.
    Write {
        /// The array to write to.
        array_ref: Argument,
        /// The index of the element to write.
        index: Argument,
        /// The value to be written.
        value: Argument,
    },
    /// Gets the length of an array.
    Length {
        /// The array to get the length of.
        array_ref: Argument,
    },
}
impl Operation {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::New { length, .. } => length.iter().copied().collect(),
            Self::NewMultiDim { dimensions, .. } => dimensions.iter().flatten().copied().collect(),
            Self::Read { array_ref, index } => {
                array_ref.iter().chain(index.iter()).copied().collect()
            }
            Self::Write {
                array_ref,
                index,
                value,
            } => array_ref
                .iter()
                .chain(index.iter())
                .chain(value.iter())
                .copied()
                .collect(),
            Self::Length { array_ref } => array_ref.iter().copied().collect(),
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New {
                element_type,
                length,
            } => write!(f, "new {}[{}]", element_type.descriptor(), length),
            Self::NewMultiDim {
                element_type,
                dimensions,
            } => {
                write!(
                    f,
                    "new {}[{}]",
                    element_type.descriptor(),
                    dimensions
                        .iter()
                        .map(std::string::ToString::to_string)
                        .join(", ")
                )
            }
            Self::Read { array_ref, index } => write!(f, "{array_ref}[{index}]"),
            Self::Write {
                array_ref,
                index,
                value,
            } => write!(f, "{array_ref}[{index}] = {value}"),
            Self::Length { array_ref } => write!(f, "array_len({array_ref})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ir::test::arb_argument, tests::arb_field_type};

    use super::*;
    use proptest::prelude::*;

    fn check_uses<'a>(op: &Operation, args: impl IntoIterator<Item = &'a Argument>) {
        let uses = op.uses();
        args.into_iter().flatten().for_each(|a| {
            assert!(uses.contains(a));
        });
    }

    proptest! {

        #[test]
        fn uses(
            arg1 in arb_argument(),
            arg2 in arb_argument(),
            arg3 in arb_argument(),
            ty in arb_field_type()
        ) {
            let new_ops = Operation::New {
                element_type: ty.clone(),
                length: arg1.clone(),
            };
            check_uses(&new_ops, [&arg1]);

            let new_multi_ops = Operation::NewMultiDim {
                element_type: ty.clone(),
                dimensions: [&arg1, &arg2, &arg3].into_iter().cloned().collect()
            };
            check_uses(&new_multi_ops, [&arg1, &arg2, &arg3]);

            let read_ops = Operation::Read {
                array_ref: arg1.clone(),
                index: arg2.clone()
            };
            check_uses(&read_ops, [&arg1, &arg2]);

            let write_ops = Operation::Write {
                array_ref: arg1.clone(),
                index: arg2.clone(),
                value: arg3.clone()
            };
            check_uses(&write_ops, [&arg1,&arg2,&arg3]);

            let len_ops = Operation::Length {
                array_ref: arg1.clone()
            };
            check_uses(&len_ops, [&arg1]);
        }

    }
}
