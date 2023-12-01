use std::fmt::Display;

use itertools::Itertools;

use crate::{ir::Argument, types::FieldType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayOperation {
    New {
        element_type: FieldType,
        length: Argument,
    },
    NewMultiDim {
        element_type: FieldType,
        dimensions: Vec<Argument>,
    },
    Read {
        array_ref: Argument,
        index: Argument,
    },
    Write {
        array_ref: Argument,
        index: Argument,
        value: Argument,
    },
    Length {
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
