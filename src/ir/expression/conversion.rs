use std::fmt::Formatter;

use std::fmt::Display;

use crate::types::FieldType;

use super::super::Argument;

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
