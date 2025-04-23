use bitflags::Flags;

use super::{ParseError, ParsingContext, ToWriter, errors::GenerationError};
use crate::jvm::class::ConstantPool;

pub(super) trait ClassElement: Sized {
    type Raw: Sized;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError>;

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError>;

    fn into_bytes(self, cp: &mut ConstantPool) -> Result<Vec<u8>, GenerationError>
    where
        Self::Raw: ToWriter,
    {
        let mut bytes = Vec::new();
        self.into_raw(cp)?.to_writer(&mut bytes)?;
        Ok(bytes)
    }
}

impl<T> ClassElement for T
where
    T: Flags<Bits = u16>,
{
    type Raw = u16;

    fn from_raw(raw: Self::Raw, _ctx: &ParsingContext) -> Result<Self, ParseError> {
        T::from_bits(raw).ok_or(ParseError::malform("Invalid access flag"))
    }

    fn into_raw(self, _cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        Ok(self.bits())
    }
}
