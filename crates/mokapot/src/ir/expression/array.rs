use std::collections::HashSet;

use itertools::Itertools;

use crate::{ir::ValueId, types::field_type::FieldType};

/// An operation on an array.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation<OP = ValueId> {
    /// Create a new array.
    #[display("new {element_type}[{length}]")]
    New {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The length of the array.
        length: OP,
    },
    /// Create a new multidimensional array.
    #[display(
        "new {element_type}[{}]",
        dimensions.iter().map(std::string::ToString::to_string).join(", ")
    )]
    NewMultiDim {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The lengths of each of the dimensions of the array.
        dimensions: Vec<OP>,
    },
    /// Gets an element from an array.
    #[display("{array_ref}[{index}]")]
    Read {
        /// The array to read from.
        array_ref: OP,
        /// The index of the element to read.
        index: OP,
    },
    /// Sets an element in an array.
    #[display("{array_ref}[{index}] = {value}")]
    Write {
        /// The array to write to.
        array_ref: OP,
        /// The index of the element to write.
        index: OP,
        /// The value to be written.
        value: OP,
    },
    /// Gets the length of an array.
    #[display("array_len({array_ref})")]
    Length {
        /// The array to get the length of.
        array_ref: OP,
    },
}

impl Operation<ValueId> {
    /// Returns the values used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::New { length, .. } => HashSet::from([*length]),
            Self::NewMultiDim { dimensions, .. } => dimensions.iter().copied().collect(),
            Self::Read { array_ref, index } => HashSet::from([*array_ref, *index]),
            Self::Write {
                array_ref,
                index,
                value,
            } => HashSet::from([*array_ref, *index, *value]),
            Self::Length { array_ref } => HashSet::from([*array_ref]),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::tests::arb_field_type;

    fn check_uses<'a>(op: &Operation, args: impl IntoIterator<Item = &'a ValueId>) {
        let uses = op.uses();
        args.into_iter().for_each(|a| {
            assert!(uses.contains(a));
        });
    }

    proptest! {

        #[test]
        fn uses(
            arg1 in any::<ValueId>(),
            arg2 in any::<ValueId>(),
            arg3 in any::<ValueId>(),
            ty in arb_field_type()
        ) {
            let new_ops = Operation::New {
                element_type: ty.clone(),
                length: arg1,
            };
            check_uses(&new_ops, [&arg1]);

            let new_multi_ops = Operation::NewMultiDim {
                element_type: ty.clone(),
                dimensions: vec![arg1, arg2, arg3]
            };
            check_uses(&new_multi_ops, [&arg1, &arg2, &arg3]);

            let read_ops = Operation::Read {
                array_ref: arg1,
                index: arg2
            };
            check_uses(&read_ops, [&arg1, &arg2]);

            let write_ops = Operation::Write {
                array_ref: arg1,
                index: arg2,
                value: arg3
            };
            check_uses(&write_ops, [&arg1,&arg2,&arg3]);

            let len_ops = Operation::Length {
                array_ref: arg1
            };
            check_uses(&len_ops, [&arg1]);
        }

    }
}
