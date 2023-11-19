use std::fmt::Display;

/// Denotes a program counter in an instruction sequence.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[repr(transparent)]
pub struct ProgramCounter(u16);

impl ProgramCounter {
    /// Creates a new program counter based on the given value with a given offset.
    pub fn offset(&self, offset: i32) -> Result<Self, InvalidOffset> {
        offset
            .checked_add(self.0 as i32)
            .ok_or(InvalidOffset::I32(offset))
            .map(|it| it as u16)
            .map(Self)
    }

    /// Creates a new program counter based on the given value with a given offset (in [`i16`]).
    pub fn offset_i16(&self, offset: i16) -> Result<Self, InvalidOffset> {
        offset
            .checked_add(self.0 as i16)
            .ok_or(InvalidOffset::I16(offset))
            .map(|it| it as u16)
            .map(Self)
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

impl Into<u16> for ProgramCounter {
    fn into(self) -> u16 {
        self.0
    }
}

impl Default for ProgramCounter {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum InvalidOffset {
    #[error("Invalid i16 offset {0}")]
    I16(i16),
    #[error("Invalid i32 offset {0}")]
    I32(i32),
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(format!("{}", pc), "#00010");
    }
}
