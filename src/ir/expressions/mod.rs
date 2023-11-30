use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::{elements::references::FieldReference, types::FieldType};

use super::Argument;

mod math;

pub use math::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockOperation {
    Acquire(Argument),
    Release(Argument),
}

impl Display for LockOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use LockOperation::*;
        match self {
            Acquire(lock) => write!(f, "acquire {}", lock),
            Release(lock) => write!(f, "release {}", lock),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionOperation {
    Int2Long(Argument),
    Int2Float(Argument),
    Int2Double(Argument),
    Long2Int(Argument),
    Long2Float(Argument),
    Long2Double(Argument),
    Float2Int(Argument),
    Float2Long(Argument),
    Float2Double(Argument),
    Double2Int(Argument),
    Double2Long(Argument),
    Double2Float(Argument),
    Int2Byte(Argument),
    Int2Char(Argument),
    Int2Short(Argument),
    CheckCast(Argument, FieldType),
    InstanceOf(Argument, FieldType),
}

impl Display for ConversionOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ConversionOperation::*;
        match self {
            Int2Long(operand) => write!(f, "int2long({})", operand),
            Int2Float(operand) => write!(f, "int2float({})", operand),
            Int2Double(operand) => write!(f, "int2double({})", operand),
            Long2Int(operand) => write!(f, "long2int({})", operand),
            Long2Float(operand) => write!(f, "long2float({})", operand),
            Long2Double(operand) => write!(f, "long2double({})", operand),
            Float2Int(operand) => write!(f, "float2int({})", operand),
            Float2Long(operand) => write!(f, "float2long({})", operand),
            Float2Double(operand) => write!(f, "float2double({})", operand),
            Double2Int(operand) => write!(f, "double2int({})", operand),
            Double2Long(operand) => write!(f, "double2long({})", operand),
            Double2Float(operand) => write!(f, "double2float({})", operand),
            Int2Byte(operand) => write!(f, "int2byte({})", operand),
            Int2Char(operand) => write!(f, "int2char({})", operand),
            Int2Short(operand) => write!(f, "int2short({})", operand),
            CheckCast(operand, target_type) => {
                write!(f, "{} as {}", operand, target_type.descriptor_string())
            }
            InstanceOf(operand, target_type) => {
                write!(f, "{} is {}", operand, target_type.descriptor_string())
            }
        }
    }
}

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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
