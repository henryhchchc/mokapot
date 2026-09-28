use std::collections::HashSet;

use itertools::Itertools;

use super::ValueId;
use crate::types::field_type::FieldType;

/// An operation on an array.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation {
    /// Create a new array.
    #[display("new {element_type}[{length}]")]
    New {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The length of the array.
        length: ValueId,
    },
    /// Create a new multidimensional array.
    #[display(
        "new {element_type}[{}]",
        dimensions.iter().map(ToString::to_string).join(", ")
    )]
    NewMultiDim {
        /// The type of the elements in the array.
        element_type: FieldType,
        /// The lengths of each of the dimensions of the array.
        dimensions: Vec<ValueId>,
    },
    /// Gets an element from an array.
    #[display("{array_ref}[{index}]")]
    Read {
        /// The array to read from.
        array_ref: ValueId,
        /// The index of the element to read.
        index: ValueId,
    },
    /// Sets an element in an array.
    #[display("{array_ref}[{index}] = {value}")]
    Write {
        /// The array to write to.
        array_ref: ValueId,
        /// The index of the element to write.
        index: ValueId,
        /// The value to be written.
        value: ValueId,
    },
    /// Gets the length of an array.
    #[display("array_len({array_ref})")]
    Length {
        /// The array to get the length of.
        array_ref: ValueId,
    },
}

impl Operation {
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
    use super::*;
    use crate::{ir::test::prelude::ids, types::field_type::PrimitiveType};

    #[test]
    fn uses_reports_array_indices_values_and_dimensions() {
        let [array_ref, index, value, length_value] = ids(0);
        let new = Operation::New {
            element_type: PrimitiveType::Int.into(),
            length: length_value,
        };
        let new_multi = Operation::NewMultiDim {
            element_type: PrimitiveType::Int.into(),
            dimensions: vec![index, length_value],
        };
        let read = Operation::Read { array_ref, index };
        let write = Operation::Write {
            array_ref,
            index,
            value,
        };
        let array_length = Operation::Length { array_ref };

        assert_eq!(new.uses(), HashSet::from([length_value]));
        assert_eq!(new_multi.uses(), HashSet::from([index, length_value]));
        assert_eq!(read.uses(), HashSet::from([array_ref, index]));
        assert_eq!(write.uses(), HashSet::from([array_ref, index, value]));
        assert_eq!(array_length.uses(), HashSet::from([array_ref]));
    }
}
