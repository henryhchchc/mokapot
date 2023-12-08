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
        match self {
            Self::Int2Long(arg) | Self::Float2Long(arg) | Self::Double2Long(arg) => {
                write!(f, "{arg} as long")
            }
            Self::Long2Int(arg) | Self::Float2Int(arg) | Self::Double2Int(arg) => {
                write!(f, "{arg} as int")
            }
            Self::Int2Float(arg) | Self::Long2Float(arg) | Self::Double2Float(arg) => {
                write!(f, "{arg} as float")
            }
            Self::Int2Double(arg) | Self::Long2Double(arg) | Self::Float2Double(arg) => {
                write!(f, "{arg} as double")
            }
            Self::Int2Byte(operand) => write!(f, "{operand} as byte"),
            Self::Int2Char(operand) => write!(f, "{operand} as char"),
            Self::Int2Short(operand) => write!(f, "{operand} as short"),
            Self::CheckCast(operand, target_type) => {
                write!(f, "{} as {}", operand, target_type.descriptor_string())
            }
            Self::InstanceOf(operand, target_type) => {
                write!(f, "{} is {}", operand, target_type.descriptor_string())
            }
        }
    }
}
