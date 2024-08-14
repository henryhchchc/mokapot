use std::collections::BTreeSet;

use itertools::Itertools;

use crate::{
    ir::{Identifier, Operand},
    types::field_type::FieldType,
};

/// An operation on an array.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation {
    /// Create a new array.
    #[display("new {element_type}[{length}]")]
    New {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The length of the array.
        length: Operand,
    },
    /// Create a new multidimensional array.
    #[display(
        "new {element_type}[{}]",
        dimensions.iter().map(std::string::ToString::to_string).join(", ")
    )]
    NewMultiDim {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The legths of each of the dimensions of the array.
        dimensions: Vec<Operand>,
    },
    /// Gets an element from an array.
    #[display("{array_ref}[{index}]")]
    Read {
        /// The array to read from.
        array_ref: Operand,
        /// The index of the element to read.
        index: Operand,
    },
    /// Sets an element in an array.
    #[display("{array_ref}[{index}] = {value}")]
    Write {
        /// The array to write to.
        array_ref: Operand,
        /// The index of the element to write.
        index: Operand,
        /// The value to be written.
        value: Operand,
    },
    /// Gets the length of an array.
    #[display("array_len({array_ref})")]
    Length {
        /// The array to get the length of.
        array_ref: Operand,
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

#[cfg(test)]
mod tests {
    use crate::{ir::test::arb_argument, tests::arb_field_type};

    use super::*;
    use proptest::prelude::*;

    fn check_uses<'a>(op: &Operation, args: impl IntoIterator<Item = &'a Operand>) {
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
