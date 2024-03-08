use std::collections::BTreeSet;
use std::fmt::Formatter;

use std::fmt::Display;

use crate::ir::Identifier;
use crate::types::field_type::FieldType;

use super::super::Argument;

/// An operation that converts between types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operaion {
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
impl Operaion {
    /// Returns the set of [`Identifier`]s used by the expression.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Int2Long(arg)
            | Self::Float2Long(arg)
            | Self::Double2Long(arg)
            | Self::Long2Int(arg)
            | Self::Float2Int(arg)
            | Self::Double2Int(arg)
            | Self::Long2Float(arg)
            | Self::Int2Float(arg)
            | Self::Double2Float(arg)
            | Self::Long2Double(arg)
            | Self::Int2Double(arg)
            | Self::Float2Double(arg)
            | Self::Int2Byte(arg)
            | Self::Int2Char(arg)
            | Self::Int2Short(arg)
            | Self::CheckCast(arg, _)
            | Self::InstanceOf(arg, _) => arg.iter().copied().collect(),
        }
    }
}

impl Display for Operaion {
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
                write!(f, "{} as {}", operand, target_type.descriptor())
            }
            Self::InstanceOf(operand, target_type) => {
                write!(f, "{} is {}", operand, target_type.descriptor())
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{ir::test::arb_argument, tests::arb_field_type};

    use super::*;
    use proptest::prelude::*;

    proptest! {

        #[test]
        fn uses(
            arg in arb_argument(),
            target_type in arb_field_type(),
        ) {
            let arg_ids: BTreeSet<_> = arg.clone().into_iter().collect();
            let conversions = [
                Operaion::Int2Long(arg.clone()),
                Operaion::Int2Float(arg.clone()),
                Operaion::Int2Double(arg.clone()),
                Operaion::Long2Int(arg.clone()),
                Operaion::Long2Float(arg.clone()),
                Operaion::Long2Double(arg.clone()),
                Operaion::Float2Int(arg.clone()),
                Operaion::Float2Long(arg.clone()),
                Operaion::Float2Double(arg.clone()),
                Operaion::Double2Int(arg.clone()),
                Operaion::Double2Long(arg.clone()),
                Operaion::Double2Float(arg.clone()),
                Operaion::Int2Byte(arg.clone()),
                Operaion::Int2Char(arg.clone()),
                Operaion::Int2Short(arg.clone()),
                Operaion::CheckCast(arg.clone(), target_type.clone()),
                Operaion::InstanceOf(arg.clone(), target_type.clone()),
            ];

            for conv in conversions {
                let uses = conv.uses();
                for id in &arg_ids {
                    assert!(uses.contains(id));
                }
            }
        }
    }
}
