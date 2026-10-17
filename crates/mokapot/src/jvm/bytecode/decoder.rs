//! Slice decoding for JVM wire values.

use zerocopy::{FromBytes, byteorder::big_endian::U16};

use super::{FromBytecode, ParseError};
use crate::jvm::code::ProgramCounter;

pub(crate) fn read_value<T: FromBytes>(bytes: &mut &[u8]) -> Result<T, ParseError> {
    let (value, remaining) =
        T::read_from_prefix(bytes).map_err(|_| ParseError::malform("Truncated bytecode value"))?;
    *bytes = remaining;
    Ok(value)
}

pub(crate) trait BytecodeSlice {
    fn decode_value<T: FromBytecode>(&mut self) -> Result<T, ParseError>;
}

impl BytecodeSlice for &[u8] {
    fn decode_value<T: FromBytecode>(&mut self) -> Result<T, ParseError> {
        T::from_bytes(self)
    }
}

impl<const N: usize> FromBytecode for [u8; N] {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        read_value(bytes)
    }
}

impl FromBytecode for ProgramCounter {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        Ok(read_value::<U16>(bytes)?.get().into())
    }
}

/// Copies `len` bytes after checking availability and advances the slice.
pub(crate) fn read_bytes(bytes: &mut &[u8], len: usize) -> Result<Box<[u8]>, ParseError> {
    let (value, remaining) = bytes
        .split_at_checked(len)
        .ok_or_else(|| ParseError::malform("Truncated bytecode payload"))?;
    *bytes = remaining;
    Ok(value.into())
}

#[cfg(test)]
mod test {
    use crate::jvm::bytecode::ParseErrorKind::Malformed;

    use proptest::prelude::*;
    use zerocopy::byteorder::big_endian::{I16, I32, U16, U32};

    use super::{BytecodeSlice, read_value};

    #[test]
    fn read_bytes_success() {
        let mut reader = [0x01, 0x02, 0x03, 0x04].as_slice();
        let buf: [u8; 3] = reader.decode_value().unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04]);
    }

    #[test]
    fn read_bytes_failed() {
        let mut reader = [0x01, 0x02].as_slice();
        let err = reader.decode_value::<[u8; 3]>().unwrap_err();
        assert_eq!(err.kind(), Malformed);
    }

    macro_rules! integer_decode_properties {
        ($success:ident, $truncated:ident, $type:ty, $wire:ty) => {
            proptest! {
                #[test]
                fn $success(value in any::<$type>(), suffix in prop::collection::vec(any::<u8>(), 0..8)) {
                    let mut bytes = value.to_be_bytes().to_vec();
                    bytes.extend(&suffix);
                    let mut reader = bytes.as_slice();

                    let decoded = read_value::<$wire>(&mut reader).map(<$type>::from).unwrap();

                    prop_assert_eq!(decoded, value);
                    prop_assert_eq!(reader, suffix.as_slice());
                }

                #[test]
                fn $truncated(
                    value in any::<$type>(),
                    available in 0usize..size_of::<$type>(),
                ) {
                    let bytes = value.to_be_bytes();
                    let mut reader = &bytes[..available];

                    let error = read_value::<$wire>(&mut reader).unwrap_err();

                    prop_assert_eq!(error.kind(), Malformed);
                }
            }
        };
    }

    integer_decode_properties!(read_u32_success, read_u32_failed, u32, U32);
    integer_decode_properties!(read_i32_success, read_i32_failed, i32, I32);
    integer_decode_properties!(read_u16_success, read_u16_failed, u16, U16);
    integer_decode_properties!(read_i16_success, read_i16_failed, i16, I16);
    integer_decode_properties!(read_u8_success, read_u8_failed, u8, u8);
    integer_decode_properties!(read_i8_success, read_i8_failed, i8, i8);

    proptest! {
        #[test]
        fn miri_read_bytes_success(
            expected in prop::collection::vec(any::<u8>(), 0..64),
            suffix in prop::collection::vec(any::<u8>(), 0..16),
        ) {
            let mut bytes = expected.clone();
            bytes.extend(&suffix);
            let mut reader = bytes.as_slice();

            let decoded = super::read_bytes(&mut reader, expected.len()).unwrap();

            prop_assert_eq!(decoded.as_ref(), expected.as_slice());
            prop_assert_eq!(reader, suffix.as_slice());
        }

        #[test]
        fn miri_read_bytes_failed(
            available in prop::collection::vec(any::<u8>(), 0..64),
            missing in 1usize..16,
        ) {
            let requested = available.len() + missing;
            let mut reader = available.as_slice();

            let error = super::read_bytes(&mut reader, requested).unwrap_err();

            prop_assert_eq!(error.kind(), Malformed);
        }
    }
}
