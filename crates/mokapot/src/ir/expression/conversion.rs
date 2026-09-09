use std::collections::HashSet;

use crate::{ir::ValueId, types::reference_type::ReferenceType};

/// An operation that converts between types.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation<OP: std::fmt::Display = ValueId> {
    /// Converts an `int` to a `long`.
    #[display("{_0} as long")]
    Int2Long(OP),
    /// Converts an `int` to a `float`.
    #[display("{_0} as float")]
    Int2Float(OP),
    /// Converts an `int` to a `double`.
    #[display("{_0} as double")]
    Int2Double(OP),
    /// Converts a `long` to an `int`.
    #[display("{_0} as int")]
    Long2Int(OP),
    /// Converts a `long` to a `float`.
    #[display("{_0} as float")]
    Long2Float(OP),
    /// Converts a `long` to a `double`.
    #[display("{_0} as double")]
    Long2Double(OP),
    /// Converts a `float` to an `int`.
    #[display("{_0} as int")]
    Float2Int(OP),
    /// Converts a `float` to a `long`.
    #[display("{_0} as long")]
    Float2Long(OP),
    /// Converts a `float` to a `double`.
    #[display("{_0} as double")]
    Float2Double(OP),
    /// Converts a `double` to an `int`.
    #[display("{_0} as int")]
    Double2Int(OP),
    /// Converts a `double` to a `long`.
    #[display("{_0} as long")]
    Double2Long(OP),
    /// Converts a `double` to a `float`.
    #[display("{_0} as float")]
    Double2Float(OP),
    /// Converts an `int` to a `byte`.
    #[display("{_0} as byte")]
    Int2Byte(OP),
    /// Converts an `int` to a `char`.
    #[display("{_0} as char")]
    Int2Char(OP),
    /// Converts an `int` to a `short`.
    #[display("{_0} as short")]
    Int2Short(OP),
    /// Checks if an object is an instance of a given type, and casts it to that type if so.
    #[display("{_0} as {_1}")]
    CheckCast(OP, ReferenceType),
    /// Checks whether an object is an instance of a given type.
    #[display("{_0} is {_1}")]
    InstanceOf(OP, ReferenceType),
}
impl Operation<ValueId> {
    /// Returns the values used by the expression.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
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
            | Self::InstanceOf(arg, _) => HashSet::from([*arg]),
        }
    }
}

#[cfg(test)]
mod tests {

    use proptest::prelude::*;

    use super::*;
    use crate::tests::arb_reference_type;

    proptest! {

        #[test]
        fn uses(
            arg in any::<ValueId>(),
            target_type in arb_reference_type(),
        ) {
            let arg_ids = HashSet::from([arg]);
            let conversions = [
                Operation::Int2Long(arg),
                Operation::Int2Float(arg),
                Operation::Int2Double(arg),
                Operation::Long2Int(arg),
                Operation::Long2Float(arg),
                Operation::Long2Double(arg),
                Operation::Float2Int(arg),
                Operation::Float2Long(arg),
                Operation::Float2Double(arg),
                Operation::Double2Int(arg),
                Operation::Double2Long(arg),
                Operation::Double2Float(arg),
                Operation::Int2Byte(arg),
                Operation::Int2Char(arg),
                Operation::Int2Short(arg),
                Operation::CheckCast(arg, target_type.clone()),
                Operation::InstanceOf(arg, target_type.clone()),
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
