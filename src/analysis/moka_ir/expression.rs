use std::fmt::{Display, Formatter};

use itertools::Itertools;

use crate::{
    elements::{
        instruction::{Instruction, ProgramCounter},
        references::FieldReference,
        ConstantValue,
    },
    types::FieldType,
};

use super::ValueRef;

#[derive(Debug)]
pub enum Expression {
    Const(ConstantValue),
    Math(MathOperation),
    Field(FieldAccess),
    Array(ArrayOperation),
    ReturnAddress(ProgramCounter),
    Insn {
        instruction: Instruction,
        arguments: Vec<ValueRef>,
    },
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Expression::*;
        match self {
            Const(c) => write!(f, "{:?}", c),
            ReturnAddress(pc) => write!(f, "{:?}", pc),
            Field(field_op) => field_op.fmt(f),
            Array(array_op) => array_op.fmt(f),
            Math(math_op) => math_op.fmt(f),
            Insn {
                instruction,
                arguments,
            } => {
                write!(
                    f,
                    "{}({})",
                    instruction.name(),
                    arguments.iter().map(|it| it.to_string()).join(", ")
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum MathOperation {
    Add(ValueRef, ValueRef),
    Subtract(ValueRef, ValueRef),
    Multiply(ValueRef, ValueRef),
    Divide(ValueRef, ValueRef),
    Remainder(ValueRef, ValueRef),
    Negate(ValueRef),
    Increment(ValueRef),
    ShiftLeft(ValueRef, ValueRef),
    ShiftRight(ValueRef, ValueRef),
    LogicalShiftRight(ValueRef, ValueRef),
    BitwiseAnd(ValueRef, ValueRef),
    BitwiseOr(ValueRef, ValueRef),
    BitwiseXor(ValueRef, ValueRef),
}

impl Display for MathOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use MathOperation::*;
        match self {
            Add(a, b) => write!(f, "{} + {}", a, b),
            Subtract(a, b) => write!(f, "{} - {}", a, b),
            Multiply(a, b) => write!(f, "{} * {}", a, b),
            Divide(a, b) => write!(f, "{} / {}", a, b),
            Remainder(a, b) => write!(f, "{} % {}", a, b),
            Negate(a) => write!(f, "-{}", a),
            Increment(a) => write!(f, "{} + 1", a),
            ShiftLeft(a, b) => write!(f, "{} << {}", a, b),
            ShiftRight(a, b) => write!(f, "{} >> {}", a, b),
            LogicalShiftRight(a, b) => write!(f, "{} >>> {}", a, b),
            BitwiseAnd(a, b) => write!(f, "{} & {}", a, b),
            BitwiseOr(a, b) => write!(f, "{} | {}", a, b),
            BitwiseXor(a, b) => write!(f, "{} ^ {}", a, b),
        }
    }
}

#[derive(Debug)]
pub enum ArrayOperation {
    New {
        element_type: FieldType,
        length: ValueRef,
    },
    NewMultiDim {
        element_type: FieldType,
        dimensions: Vec<ValueRef>,
    },
    Read {
        array_ref: ValueRef,
        index: ValueRef,
    },
    Write {
        array_ref: ValueRef,
        index: ValueRef,
        value: ValueRef,
    },
    Length {
        array_ref: ValueRef,
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

#[derive(Debug)]
pub enum FieldAccess {
    ReadStatic {
        field: FieldReference,
    },
    WriteStatic {
        field: FieldReference,
        value: ValueRef,
    },
    ReadInstance {
        object_ref: ValueRef,
        field: FieldReference,
    },
    WriteInstance {
        object_ref: ValueRef,
        field: FieldReference,
        value: ValueRef,
    },
}

impl Display for FieldAccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use FieldAccess::*;
        match self {
            ReadStatic { field } => write!(f, "{}", field),
            WriteStatic { field, value } => write!(f, "{} = {}", field, value),
            ReadInstance { object_ref, field } => write!(f, "{}.{}", object_ref, field),
            WriteInstance {
                object_ref,
                field,
                value,
            } => write!(f, "{}.{} = {}", object_ref, field, value),
        }
    }
}
