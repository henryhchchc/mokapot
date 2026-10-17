use bitflags::Flags;

use super::ParseError;

pub(super) fn parse_flags<F: Flags<Bits = u16>>(bits: u16, message: &str) -> Result<F, ParseError> {
    F::from_bits(bits).ok_or_else(|| ParseError::malform(message))
}
