use std::fmt::Display;

/// Denotes a program counter in an instruction sequence.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(transparent)]
#[derive(Default)]
pub struct ProgramCounter(u16);

impl ProgramCounter {
    /// Creates a new program counter based on the given value with a given offset.
    /// # Errors
    /// - [`InvalidOffset::I32`] If the resulting value is too large to fit into a [`ProgramCounter`].
    pub fn offset(&self, offset: i32) -> Result<Self, InvalidOffset> {
        let self_i32 = i32::from(self.0);
        let result = self_i32 + offset;
        u16::try_from(result)
            .map(Self)
            .map_err(|_| InvalidOffset::I32(offset))
    }

    /// Creates a new program counter based on the given value with a given offset (in [`i16`]).
    /// # Errors
    /// - [`InvalidOffset::I16`] If the resulting value is too large to fit into a [`ProgramCounter`].
    pub fn offset_i16(&self, offset: i16) -> Result<Self, InvalidOffset> {
        let self_i32 = i32::from(self.0);
        let offset_i32 = i32::from(offset);
        let result = self_i32 + offset_i32;
        u16::try_from(result)
            .map(Self)
            .map_err(|_| InvalidOffset::I16(offset))
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
}

impl Display for ProgramCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:05}", self.0)
    }
}

impl From<u16> for ProgramCounter {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<ProgramCounter> for u16 {
    fn from(val: ProgramCounter) -> Self {
        val.0
    }
}

/// An error occurring when trying to offset a program counter.
#[derive(thiserror::Error, Debug)]
pub enum InvalidOffset {
    /// When the offset is given as an [`i16`].
    #[error("Invalid i16 offset {0}")]
    I16(i16),
    /// When the offset is given as an [`i32`].
    #[error("Invalid i32 offset {0}")]
    I32(i32),
}

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
        assert_eq!(pc.offset(5).unwrap(), ProgramCounter::from(15));
        assert_eq!(pc.offset(-5).unwrap(), ProgramCounter::from(5));
        assert!(pc.offset(i32::MAX).is_err());
    }

    #[test]
    fn test_offset_i16() {
        let pc = ProgramCounter::from(10);
        assert_eq!(pc.offset_i16(5).unwrap(), ProgramCounter::from(15));
        assert_eq!(pc.offset_i16(-5).unwrap(), ProgramCounter::from(5));
        assert!(pc.offset_i16(i16::MAX).is_err());
    }

    #[test]
    fn test_default() {
        assert_eq!(ProgramCounter::default(), ProgramCounter::from(0));
    }

    #[test]
    fn test_display() {
        let pc = ProgramCounter::from(10);
        assert_eq!(format!("{pc}"), "#00010");
    }
}
