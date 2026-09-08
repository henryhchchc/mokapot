use std::{fmt::Debug, ops::Add};

/// Denotes a program counter in an instruction sequence.
#[derive(
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::From,
    derive_more::Into,
    derive_more::Display,
)]
#[repr(transparent)]
#[display("#{_0:04X}")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct ProgramCounter(u16);

impl ProgramCounter {
    /// Creates a new program counter based on the given value with a given offset.
    /// # Errors
    /// - [`InvalidOffset`] If the resulting value is too large to fit into a [`ProgramCounter`].
    #[deprecated(note = "Use the `+` operator instead.")]
    pub fn offset(&self, offset: i32) -> Result<Self, InvalidOffset> {
        *self + offset
    }

    /// Creates a new program counter based on the given value with a given offset (in [`i16`]).
    /// # Errors
    /// - [`InvalidOffset`] If the resulting value is too large to fit into a [`ProgramCounter`].
    #[deprecated(note = "Use the `+` operator instead.")]
    pub fn offset_i16(&self, offset: i16) -> Result<Self, InvalidOffset> {
        *self + offset
    }
}

impl Add<i16> for ProgramCounter {
    type Output = Result<Self, InvalidOffset>;

    fn add(self, rhs: i16) -> Self::Output {
        let self_i32 = i32::from(self.0);
        let offset_i32 = i32::from(rhs);
        self_i32
            .checked_add(offset_i32)
            .and_then(|it| u16::try_from(it).ok())
            .map(Self)
            .ok_or(InvalidOffset)
    }
}

impl Add<i32> for ProgramCounter {
    type Output = Result<Self, InvalidOffset>;

    fn add(self, rhs: i32) -> Self::Output {
        let self_i32 = i32::from(self.0);
        self_i32
            .checked_add(rhs)
            .and_then(|it| u16::try_from(it).ok())
            .map(Self)
            .ok_or(InvalidOffset)
    }
}

impl Add<u16> for ProgramCounter {
    type Output = Result<Self, InvalidOffset>;

    fn add(self, rhs: u16) -> Self::Output {
        let self_u32 = u32::from(self.0);
        let offeset_u32 = u32::from(rhs);
        self_u32
            .checked_add(offeset_u32)
            .and_then(|it| u16::try_from(it).ok())
            .map(Self)
            .ok_or(InvalidOffset)
    }
}

impl ProgramCounter {
    /// Denotes the entry point of a program.
    pub const ZERO: Self = Self(0);

    /// Checks if the program counter is an entry point.
    #[must_use]
    pub const fn is_entry_point(&self) -> bool {
        self.0 == 0
    }

    /// Converts the program counter into a different type.
    #[must_use]
    pub fn into<T>(self) -> T
    where
        u16: Into<T>,
    {
        self.0.into()
    }
}

impl Debug for ProgramCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProgramCounter(#{:04X})", self.0)
    }
}

/// An error occurring when trying to offset a program counter.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
#[error("Invalid PC Offset")]
pub struct InvalidOffset;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn entry_point_matches_zero(value in any::<u16>()) {
            prop_assert_eq!(ProgramCounter::from(value).is_entry_point(), value == 0);
        }

        #[test]
        fn i32_offset_matches_checked_arithmetic(value in any::<u16>(), offset in any::<i32>()) {
            let expected = i64::from(value) + i64::from(offset);
            let actual = ProgramCounter::from(value) + offset;

            if let Ok(expected) = u16::try_from(expected) {
                prop_assert_eq!(actual, Ok(ProgramCounter::from(expected)));
            } else {
                prop_assert_eq!(actual, Err(InvalidOffset));
            }
        }

        #[test]
        fn i16_offset_matches_checked_arithmetic(value in any::<u16>(), offset in any::<i16>()) {
            let expected = i32::from(value) + i32::from(offset);
            let actual = ProgramCounter::from(value) + offset;

            if let Ok(expected) = u16::try_from(expected) {
                prop_assert_eq!(actual, Ok(ProgramCounter::from(expected)));
            } else {
                prop_assert_eq!(actual, Err(InvalidOffset));
            }
        }

        #[test]
        fn u16_offset_matches_checked_arithmetic(value in any::<u16>(), offset in any::<u16>()) {
            let expected = u32::from(value) + u32::from(offset);
            let actual = ProgramCounter::from(value) + offset;

            if let Ok(expected) = u16::try_from(expected) {
                prop_assert_eq!(actual, Ok(ProgramCounter::from(expected)));
            } else {
                prop_assert_eq!(actual, Err(InvalidOffset));
            }
        }

        #[test]
        fn display_is_zero_padded_uppercase_hex(value in any::<u16>()) {
            let pc = ProgramCounter::from(value);
            prop_assert_eq!(format!("{pc}"), format!("#{value:04X}"));
        }
    }

    #[test]
    fn test_default() {
        assert_eq!(ProgramCounter::default(), ProgramCounter::from(0));
    }
}
