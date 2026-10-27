//! Slice decoding for JVM wire values.

use crate::intrinsics::ExactSizeIteratorExt;

use zerocopy::{
    FromBytes, Immutable, KnownLayout, Unaligned,
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
        .map_err(|_| ParseError::truncated(std::mem::size_of::<T>(), bytes.len()))?;
    *bytes = remaining;
    Ok(value)
}

/// Decodes one value, advancing the input past it.
pub(super) fn decode_value<'a, T: Decode<'a>>(bytes: &mut &'a [u8]) -> Result<T, ParseError> {
    T::decode(bytes)
}
macro_rules! impl_decode_integer {
    ($type:ty, $wire_type:ty) => {
        impl<'a> Decode<'a> for $type {
            fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
                Ok(read_wire::<$wire_type>(bytes)?.get())
            }
        }
    };
}

impl_decode_integer!(u16, U16);
impl_decode_integer!(i16, I16);
impl_decode_integer!(u32, U32);
impl_decode_integer!(i32, I32);

impl<'a, T: FixedLayout> Decode<'a> for T {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        read_wire::<Self>(bytes)
    }
}

/// Borrows `len` bytes. Leaves the input unchanged on failure.
pub(super) fn borrow_bytes<'a>(bytes: &mut &'a [u8], len: usize) -> Result<&'a [u8], ParseError> {
    let (value, remaining) = bytes
        .split_at_checked(len)
        .ok_or_else(|| ParseError::truncated(len, bytes.len()))?;
    *bytes = remaining;
    Ok(value)
}

/// Borrows `count` fixed wire entries. Leaves the input unchanged on failure.
pub(super) fn borrow_slice<'a, T>(bytes: &mut &'a [u8], count: usize) -> Result<&'a [T], ParseError>
where
    T: FixedLayout + KnownLayout,
{
    borrow_view(bytes, count)
}

/// Borrows a wire structure with `count` trailing elements, preserving input on failure.
pub(super) fn borrow_view<'a, T>(bytes: &mut &'a [u8], count: usize) -> Result<&'a T, ParseError>
where
    T: ?Sized + FromBytes + Immutable + Unaligned + KnownLayout<PointerMetadata = usize>,
{
    let (value, remaining) = T::ref_from_prefix_with_elems(bytes, count).map_err(|_| {
        ParseError::truncated(
            T::size_for_metadata(count).unwrap_or(usize::MAX),
            bytes.len(),
        )
    })?;
    *bytes = remaining;
    Ok(value)
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
    use super::{ParseError, decode_value, parse_repeated};
    use crate::jvm::errors::ParseErrorKind;

    #[test]
    fn truncated_value_reports_size_without_consuming_input() {
        let mut remaining = [1].as_slice();
        let error = decode_value::<u16>(&mut remaining).unwrap_err();
        assert_eq!(
            error.kind(),
            ParseErrorKind::Truncated {
                requested: 2,
                available: 1
            }
        );
        assert_eq!(remaining, [1]);
    }

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
}
