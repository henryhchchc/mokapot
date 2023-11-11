use std::fmt::Display;

/// Denotes a program counter in an instruction sequence.
#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ProgramCounter(u16);

impl ProgramCounter {
    /// Creates a new program counter based on the given value with a given offset.
    pub fn offset(&self, offset: i32) -> Result<Self, InvalidOffset> {
        u16::try_from(self.0 as i32 + offset)
            .map(Self)
            .map_err(|_| InvalidOffset::I32(offset))
    }

    /// Creates a new program counter based on the given value with a given offset (in [`i16`]).
    pub fn offset_i16(&self, offset: i16) -> Result<Self, InvalidOffset> {
        u16::try_from(self.0 as i16 + offset)
            .map(Self)
            .map_err(|_| InvalidOffset::I16(offset))
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
