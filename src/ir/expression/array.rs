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
    ///
    /// # Examples
    /// ```
    /// use std::collections::BTreeSet;
    /// use mokapot::ir::{expression::ArrayOperation, Argument, Identifier, Value};
    ///
    /// let length = Value::new(0);
    /// let element_type = "I".parse().unwrap();
    /// let new_array = ArrayOperation::New { element_type, length: length.as_argument() };
    ///
    /// assert_eq!(new_array.uses(), BTreeSet::from([Identifier::Value(length)]));
    /// ```
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
