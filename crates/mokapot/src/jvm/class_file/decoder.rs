//! Slice decoding for JVM wire values.

use crate::intrinsics::ExactSizeIteratorExt;

use zerocopy::{
    FromBytes, KnownLayout,
    byteorder::big_endian::{I16, I32, U16, U32},
};

use super::{ParseError, fixed_layout::FixedLayout};

/// Parses one raw JVM structure, advancing the input past it.
/// Composite failures may leave the input partially consumed.
pub(crate) trait Decode<'a>: Sized {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError>;
}

/// Copies a fixed wire value. Leaves the input unchanged on failure.
fn read_wire<T: FixedLayout>(bytes: &mut &[u8]) -> Result<T, ParseError> {
    let (value, remaining) = T::read_from_prefix(bytes)
        .map_err(|_| ParseError::malform("Truncated class-file value"))?;
    *bytes = remaining;
    Ok(value)
}

/// Decodes one value, advancing the input past it.
pub(super) fn decode_value<'a, T: Decode<'a>>(bytes: &mut &'a [u8]) -> Result<T, ParseError> {
    T::decode(bytes)
}

impl<'a> Decode<'a> for u16 {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        Ok(read_wire::<U16>(bytes)?.get())
    }
}

impl<'a> Decode<'a> for i16 {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        Ok(read_wire::<I16>(bytes)?.get())
    }
}

impl<'a> Decode<'a> for u32 {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        Ok(read_wire::<U32>(bytes)?.get())
    }
}

impl<'a> Decode<'a> for i32 {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        Ok(read_wire::<I32>(bytes)?.get())
    }
}

impl<'a, T: FixedLayout> Decode<'a> for T {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        read_wire::<Self>(bytes)
    }
}

/// Borrows `len` bytes. Leaves the input unchanged on failure.
pub(super) fn borrow_bytes<'a>(bytes: &mut &'a [u8], len: usize) -> Result<&'a [u8], ParseError> {
    let (value, remaining) = bytes
        .split_at_checked(len)
        .ok_or_else(|| ParseError::malform("Truncated class-file payload"))?;
    *bytes = remaining;
    Ok(value)
}

/// Borrows `count` fixed wire entries. Leaves the input unchanged on failure.
pub(super) fn borrow_slice<'a, T>(bytes: &mut &'a [u8], count: usize) -> Result<&'a [T], ParseError>
where
    T: FixedLayout + KnownLayout,
{
    let (values, remaining) = <[T]>::ref_from_prefix_with_elems(bytes, count)
        .map_err(|_| ParseError::malform("Truncated class-file table"))?;
    *bytes = remaining;
    Ok(values)
}

/// Parses an explicit number of elements, stopping at the first failure.
pub(super) fn parse_repeated<'a, E>(
    bytes: &mut &'a [u8],
    count: impl Into<usize>,
    mut parser: impl FnMut(&mut &'a [u8]) -> Result<E, ParseError>,
) -> Result<Box<[E]>, ParseError> {
    (0..count.into()).map(|_| parser(bytes)).try_collect_exact()
}

#[cfg(test)]
mod tests {
    use super::{ParseError, borrow_slice, decode_value, parse_repeated};
    use crate::jvm::errors::ParseErrorKind;
    use zerocopy::byteorder::big_endian::U16;

    #[test]
    fn repeated_parsing_stops_when_an_element_is_rejected() {
        let mut remaining = [1, 0xff, 3].as_slice();
        let error = parse_repeated(&mut remaining, 3_u8, |bytes| {
            let value: u8 = decode_value(bytes)?;
            if value == 0xff {
                return Err(ParseError::malform("Rejected element"));
            }
            Ok(value)
        })
        .unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
        assert_eq!(remaining, [3]);
    }

    #[test]
    fn borrowed_table_preserves_cursor_boundaries() {
        let bytes = [0xff, 0x12, 0x34, 0xab, 0xcd, 0xee];
        let mut remaining = &bytes[1..];
        borrow_slice::<U16>(&mut remaining, 2).unwrap();
        assert_eq!(remaining, [0xee]);
        borrow_slice::<U16>(&mut remaining, 0).unwrap();
        assert_eq!(remaining, [0xee]);
        for count in [1, usize::MAX] {
            let error = borrow_slice::<U16>(&mut remaining, count).unwrap_err();
            assert_eq!(error.kind(), ParseErrorKind::Malformed);
            assert_eq!(remaining, [0xee]);
        }
    }
}
