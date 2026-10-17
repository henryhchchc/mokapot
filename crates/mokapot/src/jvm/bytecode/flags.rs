use bitflags::Flags;

use super::ParseError;

pub(super) fn parse_flags<F: Flags<Bits = u16>>(bits: u16, message: &str) -> Result<F, ParseError> {
    F::from_bits(bits).ok_or_else(|| ParseError::malform(message))
}

#[cfg(test)]
mod tests {
    use crate::jvm::{bytecode::ParseErrorKind, class::AccessFlags};

    use super::parse_flags;

    #[test]
    fn parses_valid_flags() {
        let flags = parse_flags::<AccessFlags>(AccessFlags::PUBLIC.bits(), "bad flags").unwrap();

        assert_eq!(flags, AccessFlags::PUBLIC);
    }

    #[test]
    fn rejects_unknown_bits_as_malformed() {
        let error = parse_flags::<AccessFlags>(0x0004, "bad flags").unwrap_err();

        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }
}
