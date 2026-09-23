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
