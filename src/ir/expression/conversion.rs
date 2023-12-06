use std::fmt::Formatter;

use std::fmt::Display;

use crate::types::field_type::FieldType;

use super::super::Argument;

/// An operation that converts between types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionOperation {
    /// Converts an `int` to a `long`.
    Int2Long(Argument),
    /// Converts an `int` to a `float`.
    Int2Float(Argument),
    /// Converts an `int` to a `double`.
    Int2Double(Argument),
    /// Converts a `long` to an `int`.
    Long2Int(Argument),
    /// Converts a `long` to a `float`.
    Long2Float(Argument),
    /// Converts a `long` to a `double`.
    Long2Double(Argument),
    /// Converts a `float` to an `int`.
    Float2Int(Argument),
    /// Converts a `float` to a `long`.
    Float2Long(Argument),
    /// Converts a `float` to a `double`.
    Float2Double(Argument),
    /// Converts a `double` to an `int`.
    Double2Int(Argument),
    /// Converts a `double` to a `long`.
    Double2Long(Argument),
    /// Converts a `double` to a `float`.
    Double2Float(Argument),
    /// Converts an `int` to a `byte`.
    Int2Byte(Argument),
    /// Converts an `int` to a `char`.
    Int2Char(Argument),
    /// Converts an `int` to a `short`.
    Int2Short(Argument),
    /// Checks if an object is an instance of a given type, and casts it to that type if so.
    CheckCast(Argument, FieldType),
    /// Checks whether an object is an instance of a given type.
    InstanceOf(Argument, FieldType),
}

impl Display for ConversionOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ConversionOperation::*;
        match self {
            Int2Long(arg) | Float2Long(arg) | Double2Long(arg) => write!(f, "{} as long", arg),
            Long2Int(arg) | Float2Int(arg) | Double2Int(arg) => write!(f, "{} as int", arg),
            Int2Float(arg) | Long2Float(arg) | Double2Float(arg) => write!(f, "{} as float", arg),
            Int2Double(arg) | Long2Double(arg) | Float2Double(arg) => {
                write!(f, "{} as double", arg)
            }
            Int2Byte(operand) => write!(f, "{} as byte", operand),
            Int2Char(operand) => write!(f, "{} as char", operand),
            Int2Short(operand) => write!(f, "{} as short", operand),
            CheckCast(operand, target_type) => {
                write!(f, "{} as {}", operand, target_type.descriptor_string())
            }
            InstanceOf(operand, target_type) => {
                write!(f, "{} is {}", operand, target_type.descriptor_string())
            }
        }
    }
}
