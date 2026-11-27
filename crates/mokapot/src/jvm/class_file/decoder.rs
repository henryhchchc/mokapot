//! Slice decoding for JVM wire values.

use std::num::{NonZeroU8, NonZeroU16};

use zerocopy::{
    Immutable, KnownLayout, TryFromBytes, Unaligned,
    byteorder::big_endian::{I16, I32, U16, U32},
    error::{ConvertError, TryReadError},
};

use super::{ParseError, fixed_layout::FixedLayout};
use crate::{
    intrinsics::ExactSizeIteratorExt,
    jvm::{
        constant_pool::{ConstantPoolIndex, OptionalConstantPoolIndex},
        errors::ParsingErrorContext,
    },
};

/// Parses one raw JVM structure, advancing the input past it.
/// Composite failures may leave the input partially consumed.
pub(crate) trait Decode<'a>: Sized {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError>;
}

macro_rules! impl_decode_integer {
    ($type:ty, $wire_type:ty) => {
        impl<'a> Decode<'a> for $type {
            fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
                Ok(<$wire_type>::decode(bytes)?.get())
            }
        }
    };
}

impl_decode_integer!(u16, U16);
impl_decode_integer!(i16, I16);
impl_decode_integer!(u32, U32);
impl_decode_integer!(i32, I32);

impl<'a> Decode<'a> for Option<ConstantPoolIndex> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        Ok(OptionalConstantPoolIndex::decode(bytes)?.get())
    }
}

impl<'a> Decode<'a> for NonZeroU8 {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        Self::new(u8::decode(bytes)?).context("Expected a nonzero value")
    }
}

impl<'a> Decode<'a> for NonZeroU16 {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        Self::new(u16::decode(bytes)?).context("Expected a nonzero value")
    }
}

impl<'a, T: FixedLayout> Decode<'a> for T {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let (value, remaining) = T::try_read_from_prefix(bytes)
            .map_err(|error| wire_error(error, std::mem::size_of::<T>))?;
        *bytes = remaining;
        Ok(value)
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

/// Borrows a wire structure with `count` trailing elements, preserving input on failure.
pub(super) fn try_borrow_view<'a, T>(
    bytes: &mut &'a [u8],
    count: usize,
) -> Result<&'a T, ParseError>
where
    T: ?Sized + TryFromBytes + Immutable + Unaligned + KnownLayout<PointerMetadata = usize>,
{
    let (value, remaining) = T::try_ref_from_prefix_with_elems(bytes, count).map_err(|error| {
        wire_error(error.into(), || {
            T::size_for_metadata(count).unwrap_or(usize::MAX)
        })
    })?;
    *bytes = remaining;
    Ok(value)
}

fn wire_error<T: ?Sized + TryFromBytes>(
    error: TryReadError<&[u8], T>,
    requested: impl FnOnce() -> usize,
) -> ParseError {
    match error {
        ConvertError::Size(error) => ParseError::truncated(requested(), error.into_src().len()),
        // Required constant-pool indices are the only wire fields with invalid bit patterns.
        ConvertError::Validity(_) => ParseError::invalid_constant_pool_index(0),
        ConvertError::Alignment(error) => match error {},
    }
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
    use super::{ConstantPoolIndex, Decode, ParseError, parse_repeated};
    use crate::jvm::errors::ParseErrorKind;

    #[test]
    fn optional_pool_index_decodes_zero_as_absence() {
        let mut bytes = [0, 0, 0x12, 0x34].as_slice();
        let absent = Option::<ConstantPoolIndex>::decode(&mut bytes).unwrap();
        let present = Option::<ConstantPoolIndex>::decode(&mut bytes).unwrap();
        assert_eq!(absent, None);
        assert_eq!(present.map(ConstantPoolIndex::get), Some(0x1234));
        assert_eq!(bytes, []);
    }

    #[test]
    fn truncated_value_reports_size_without_consuming_input() {
        let mut remaining = [1].as_slice();
        let error = u16::decode(&mut remaining).unwrap_err();
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
            let value = u8::decode(bytes)?;
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
