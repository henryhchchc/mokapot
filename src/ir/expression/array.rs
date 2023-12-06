use std::fmt::Display;

use itertools::Itertools;

use crate::{ir::Argument, types::field_type::FieldType};

/// An operation on an array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayOperation {
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

impl Display for ArrayOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ArrayOperation::*;
        match self {
            New {
                element_type,
                length,
            } => write!(f, "new {}[{}]", element_type.descriptor_string(), length),
            NewMultiDim {
                element_type,
                dimensions,
            } => {
                write!(
                    f,
                    "new {}[{}]",
                    element_type.descriptor_string(),
                    dimensions.iter().map(|it| it.to_string()).join(", ")
                )
            }
            Read { array_ref, index } => write!(f, "{}[{}]", array_ref, index),
            Write {
                array_ref,
                index,
                value,
            } => write!(f, "{}[{}] = {}", array_ref, index, value),
            Length { array_ref } => write!(f, "array_len({})", array_ref),
        }
    }
}
