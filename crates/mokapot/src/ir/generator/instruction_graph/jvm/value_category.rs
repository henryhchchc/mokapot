use crate::types::field_type::{FieldType, PrimitiveType};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum ValueCategory {
    Category1,
    Category2,
}

impl ValueCategory {
    pub const fn slot_count(self) -> usize {
        match self {
            Self::Category1 => 1,
            Self::Category2 => 2,
        }
    }

    pub const fn of_field_type(value_type: &FieldType) -> Self {
        match value_type {
            FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) => Self::Category2,
            _ => Self::Category1,
        }
    }
}
