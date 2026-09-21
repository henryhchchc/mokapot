use std::collections::HashSet;

use crate::{ir::ValueId, types::reference_type::ReferenceType};

/// An operation that converts between types.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation {
    /// Converts an `int` to a `long`.
    #[display("{_0} as long")]
    Int2Long(ValueId),
    /// Converts an `int` to a `float`.
    #[display("{_0} as float")]
    Int2Float(ValueId),
    /// Converts an `int` to a `double`.
    #[display("{_0} as double")]
    Int2Double(ValueId),
    /// Converts a `long` to an `int`.
    #[display("{_0} as int")]
    Long2Int(ValueId),
    /// Converts a `long` to a `float`.
    #[display("{_0} as float")]
    Long2Float(ValueId),
    /// Converts a `long` to a `double`.
    #[display("{_0} as double")]
    Long2Double(ValueId),
    /// Converts a `float` to an `int`.
    #[display("{_0} as int")]
    Float2Int(ValueId),
    /// Converts a `float` to a `long`.
    #[display("{_0} as long")]
    Float2Long(ValueId),
    /// Converts a `float` to a `double`.
    #[display("{_0} as double")]
    Float2Double(ValueId),
    /// Converts a `double` to an `int`.
    #[display("{_0} as int")]
    Double2Int(ValueId),
    /// Converts a `double` to a `long`.
    #[display("{_0} as long")]
    Double2Long(ValueId),
    /// Converts a `double` to a `float`.
    #[display("{_0} as float")]
    Double2Float(ValueId),
    /// Converts an `int` to a `byte`.
    #[display("{_0} as byte")]
    Int2Byte(ValueId),
    /// Converts an `int` to a `char`.
    #[display("{_0} as char")]
    Int2Char(ValueId),
    /// Converts an `int` to a `short`.
    #[display("{_0} as short")]
    Int2Short(ValueId),
    /// Checks if an object is an instance of a given type, and casts it to that type if so.
    #[display("{_0} as {_1}")]
    CheckCast(ValueId, ReferenceType),
    /// Checks whether an object is an instance of a given type.
    #[display("{_0} is {_1}")]
    InstanceOf(ValueId, ReferenceType),
}
impl Operation {
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
