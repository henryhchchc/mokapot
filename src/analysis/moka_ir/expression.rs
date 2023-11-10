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
    Conversion(ConversionOperation),
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
            Conversion(conv_op) => conv_op.fmt(f),
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
pub enum ConversionOperation {
    Int2Long(ValueRef),
    Int2Float(ValueRef),
    Int2Double(ValueRef),
    Long2Int(ValueRef),
    Long2Float(ValueRef),
    Long2Double(ValueRef),
    Float2Int(ValueRef),
    Float2Long(ValueRef),
    Float2Double(ValueRef),
    Double2Int(ValueRef),
    Double2Long(ValueRef),
    Double2Float(ValueRef),
    Int2Byte(ValueRef),
    Int2Char(ValueRef),
    Int2Short(ValueRef),
    CheckCast {
        value: ValueRef,
        target_type: FieldType,
    },
    InstanceOf {
        value: ValueRef,
        target_type: FieldType,
    },
}

impl Display for ConversionOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ConversionOperation::*;
        match self {
            Int2Long(a) => write!(f, "int2long({})", a),
            Int2Float(a) => write!(f, "int2float({})", a),
            Int2Double(a) => write!(f, "int2double({})", a),
            Long2Int(a) => write!(f, "long2int({})", a),
            Long2Float(a) => write!(f, "long2float({})", a),
            Long2Double(a) => write!(f, "long2double({})", a),
            Float2Int(a) => write!(f, "float2int({})", a),
            Float2Long(a) => write!(f, "float2long({})", a),
            Float2Double(a) => write!(f, "float2double({})", a),
            Double2Int(a) => write!(f, "double2int({})", a),
            Double2Long(a) => write!(f, "double2long({})", a),
            Double2Float(a) => write!(f, "double2float({})", a),
            Int2Byte(a) => write!(f, "int2byte({})", a),
            Int2Char(a) => write!(f, "int2char({})", a),
            Int2Short(a) => write!(f, "int2short({})", a),
            CheckCast { value, target_type } => write!(
                f,
                "check_cast({}, {})",
                value,
                target_type.descriptor_string()
            ),
            InstanceOf { value, target_type } => {
                write!(
                    f,
                    "instance_of({}, {})",
                    value,
                    target_type.descriptor_string()
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
