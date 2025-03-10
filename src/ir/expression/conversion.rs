use std::collections::BTreeSet;

use crate::ir::Identifier;
use crate::types::field_type::FieldType;

use super::super::Operand;

/// An operation that converts between types.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation {
    /// Converts an `int` to a `long`.
    #[display("{_0} as long")]
    Int2Long(Operand),
    /// Converts an `int` to a `float`.
    #[display("{_0} as float")]
    Int2Float(Operand),
    /// Converts an `int` to a `double`.
    #[display("{_0} as double")]
    Int2Double(Operand),
    /// Converts a `long` to an `int`.
    #[display("{_0} as int")]
    Long2Int(Operand),
    /// Converts a `long` to a `float`.
    #[display("{_0} as float")]
    Long2Float(Operand),
    /// Converts a `long` to a `double`.
    #[display("{_0} as double")]
    Long2Double(Operand),
    /// Converts a `float` to an `int`.
    #[display("{_0} as int")]
    Float2Int(Operand),
    /// Converts a `float` to a `long`.
    #[display("{_0} as long")]
    Float2Long(Operand),
    /// Converts a `float` to a `double`.
    #[display("{_0} as double")]
    Float2Double(Operand),
    /// Converts a `double` to an `int`.
    #[display("{_0} as int")]
    Double2Int(Operand),
    /// Converts a `double` to a `long`.
    #[display("{_0} as long")]
    Double2Long(Operand),
    /// Converts a `double` to a `float`.
    #[display("{_0} as float")]
    Double2Float(Operand),
    /// Converts an `int` to a `byte`.
    #[display("{_0} as byte")]
    Int2Byte(Operand),
    /// Converts an `int` to a `char`.
    #[display("{_0} as char")]
    Int2Char(Operand),
    /// Converts an `int` to a `short`.
    #[display("{_0} as short")]
    Int2Short(Operand),
    /// Checks if an object is an instance of a given type, and casts it to that type if so.
    #[display("{_0} as {}", _1)]
    CheckCast(Operand, FieldType),
    /// Checks whether an object is an instance of a given type.
    #[display("{_0} is {}", _1)]
    InstanceOf(Operand, FieldType),
}
impl Operation {
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
                Operation::Int2Long(arg.clone()),
                Operation::Int2Float(arg.clone()),
                Operation::Int2Double(arg.clone()),
                Operation::Long2Int(arg.clone()),
                Operation::Long2Float(arg.clone()),
                Operation::Long2Double(arg.clone()),
                Operation::Float2Int(arg.clone()),
                Operation::Float2Long(arg.clone()),
                Operation::Float2Double(arg.clone()),
                Operation::Double2Int(arg.clone()),
                Operation::Double2Long(arg.clone()),
                Operation::Double2Float(arg.clone()),
                Operation::Int2Byte(arg.clone()),
                Operation::Int2Char(arg.clone()),
                Operation::Int2Short(arg.clone()),
                Operation::CheckCast(arg.clone(), target_type.clone()),
                Operation::InstanceOf(arg.clone(), target_type.clone()),
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
