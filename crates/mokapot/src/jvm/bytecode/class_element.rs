use bitflags::Flags;

use super::{ParseError, ParsingContext};

pub(super) trait ClassElement<'a>: Sized {
    type Raw: Sized;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError>;
}

impl<T> ClassElement<'_> for T
where
    T: Flags<Bits = u16>,
{
    type Raw = u16;

    fn from_raw(raw: Self::Raw, _ctx: &ParsingContext) -> Result<Self, ParseError> {
        T::from_bits(raw).ok_or(ParseError::malform("Invalid access flag"))
    }
}
