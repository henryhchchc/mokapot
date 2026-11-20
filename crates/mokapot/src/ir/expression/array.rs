use std::fmt;

use derive_more::Display;
use itertools::Itertools;

use super::ValueId;
use crate::types::{field_type::PrimitiveType, reference_type::ReferenceType};

/// The component type of a single-dimension array allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayElementType<'class> {
    /// A primitive component.
    Primitive(PrimitiveType),
    /// A class, interface, or array component.
    Reference(&'class ReferenceType),
}

impl fmt::Display for ArrayElementType<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(primitive) => primitive.fmt(f),
            Self::Reference(ReferenceType::Class(class)) => class.fmt(f),
            Self::Reference(ReferenceType::Array(component)) => write!(f, "{component}[]"),
        }
    }
}

/// An operation on an array.
#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum Operation<'class> {
    /// Create a new array.
    #[display("new {element_type}[{length}]")]
    New {
        /// The type of the elements in the array.
        element_type: ArrayElementType<'class>,
        /// The length of the array.
        length: ValueId,
    },
    /// Create a new multidimensional array.
    #[display(
        "new {}[{}]",
        ArrayElementType::Reference(array_type),
        dimensions.iter().map(ToString::to_string).join(", ")
    )]
    NewMultiDim {
        /// The full array type.
        array_type: &'class ReferenceType,
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
