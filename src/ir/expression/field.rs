use std::fmt::{Display, Formatter};

use crate::{ir::Argument, jvm::field::FieldReference};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldAccess {
    ReadStatic {
        field: FieldReference,
    },
    WriteStatic {
        field: FieldReference,
        value: Argument,
    },
    ReadInstance {
        object_ref: Argument,
        field: FieldReference,
    },
    WriteInstance {
        object_ref: Argument,
        field: FieldReference,
        value: Argument,
    },
}

impl Display for FieldAccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use FieldAccess::*;
        match self {
            ReadStatic { field } => write!(f, "{}", field),
            WriteStatic { field, value } => write!(f, "{} <- {}", field, value),
            ReadInstance { object_ref, field } => write!(f, "{}.{}", object_ref, field.name),
            WriteInstance {
                object_ref,
                field,
                value,
            } => write!(f, "{}.{} <- {}", object_ref, field.name, value),
        }
    }
}
