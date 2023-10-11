#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ProgramCounter(pub u16);

impl ProgramCounter {
    pub fn offset(&self, offset: i32) -> Result<Self, InvalidOffset> {
        u16::try_from(self.0 as i32 + offset)
            .map(Self)
            .map_err(|_| InvalidOffset::I32(offset))
    }

    pub fn offset_i16(&self, offset: i16) -> Result<Self, InvalidOffset> {
        u16::try_from(self.0 as i16 + offset)
            .map(Self)
            .map_err(|_| InvalidOffset::I16(offset))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum InvalidOffset {
    #[error("Invalid i16 offset {0}")]
    I16(i16),
    #[error("Invalid i32 offset {0}")]
    I32(i32),
}
