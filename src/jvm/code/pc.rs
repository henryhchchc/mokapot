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

    #[test]
    fn test_entry_point() {
        assert!(ProgramCounter::ZERO.is_entry_point());
        assert!(!ProgramCounter::from(1).is_entry_point());
    }

    #[test]
    fn test_offset() {
        let pc = ProgramCounter::from(10);
        assert_eq!(pc + 5, Ok(ProgramCounter::from(15)));
        assert_eq!(pc + -5, Ok(ProgramCounter::from(5)));
        assert_eq!(pc + i32::MAX, Err(InvalidOffset));
    }

    #[test]
    fn test_offset_i16() {
        let pc = ProgramCounter::from(u16::MAX - 10);
        assert_eq!(pc + 5i16, Ok(ProgramCounter::from(u16::MAX - 5)));
        assert_eq!(pc + -5i16, Ok(ProgramCounter::from(u16::MAX - 15)));
        assert_eq!(pc + i16::MAX, Err(InvalidOffset));
    }

    #[test]
    fn test_default() {
        assert_eq!(ProgramCounter::default(), ProgramCounter::from(0));
    }

    #[test]
    fn test_display() {
        let pc = ProgramCounter::from(10);
        assert_eq!(format!("{pc}"), "#000A");
    }
}
