//! Slice decoding for JVM wire values.

use zerocopy::{
    FromBytes, KnownLayout,
    byteorder::big_endian::{I16, I32, U16, U32},
};

use super::{ParseError, ParsingContext, fixed_layout::FixedLayout, from_raw::FromRaw};

/// Parses one raw JVM structure, advancing the input past it.
/// Composite failures may leave the input partially consumed.
pub(crate) trait FromBytecode<'a>: Sized {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError>;
}

/// Copies a fixed wire value. Leaves the input unchanged on failure.
pub(super) fn read_wire<T: FixedLayout>(bytes: &mut &[u8]) -> Result<T, ParseError> {
    let (value, remaining) =
        T::read_from_prefix(bytes).map_err(|_| ParseError::malform("Truncated bytecode value"))?;
    *bytes = remaining;
    Ok(value)
}

pub(super) fn read_u8(bytes: &mut &[u8]) -> Result<u8, ParseError> {
    read_wire(bytes)
}

pub(super) fn read_i8(bytes: &mut &[u8]) -> Result<i8, ParseError> {
    read_wire(bytes)
}

pub(super) fn read_u16(bytes: &mut &[u8]) -> Result<u16, ParseError> {
    Ok(read_wire::<U16>(bytes)?.get())
}

pub(super) fn read_i16(bytes: &mut &[u8]) -> Result<i16, ParseError> {
    Ok(read_wire::<I16>(bytes)?.get())
}

pub(super) fn read_u32(bytes: &mut &[u8]) -> Result<u32, ParseError> {
    Ok(read_wire::<U32>(bytes)?.get())
}

pub(super) fn read_i32(bytes: &mut &[u8]) -> Result<i32, ParseError> {
    Ok(read_wire::<I32>(bytes)?.get())
}

/// Borrows `len` bytes. Leaves the input unchanged on failure.
pub(super) fn borrow_bytes<'a>(bytes: &mut &'a [u8], len: usize) -> Result<&'a [u8], ParseError> {
    let (value, remaining) = bytes
        .split_at_checked(len)
        .ok_or_else(|| ParseError::malform("Truncated bytecode payload"))?;
    *bytes = remaining;
    Ok(value)
}

/// Borrows `count` fixed wire entries. Leaves the input unchanged on failure.
pub(super) fn borrow_slice<'a, T>(bytes: &mut &'a [u8], count: usize) -> Result<&'a [T], ParseError>
where
    T: FixedLayout + KnownLayout,
{
    let (values, remaining) = <[T]>::ref_from_prefix_with_elems(bytes, count)
        .map_err(|_| ParseError::malform("Truncated bytecode table"))?;
    *bytes = remaining;
    Ok(values)
}

/// Decodes a raw structure and resolves it against the class context.
pub(super) fn parse_element<'a, T>(
    bytes: &mut &'a [u8],
    ctx: &ParsingContext,
) -> Result<T, ParseError>
where
    T: FromRaw<'a>,
    T::Raw: FromBytecode<'a>,
{
    T::from_raw(T::Raw::from_bytes(bytes)?, ctx)
}

/// Parses an explicit number of elements, stopping at the first failure.
pub(super) fn parse_repeated<'a, Cnt, E, Elts>(
    bytes: &mut &'a [u8],
    count: Cnt,
    mut parser: impl FnMut(&mut &'a [u8]) -> Result<E, ParseError>,
) -> Result<Elts, ParseError>
where
    Cnt: Default,
    std::ops::Range<Cnt>: Iterator,
    Elts: FromIterator<E>,
{
    (Cnt::default()..count).map(|_| parser(bytes)).collect()
}

#[cfg(test)]
mod test {
    use crate::jvm::bytecode::ParseErrorKind::Malformed;

    use proptest::prelude::*;
    use zerocopy::byteorder::big_endian::U16;

    use super::{borrow_bytes, borrow_slice, parse_repeated, read_u8, read_u16, read_wire};

    #[test]
    fn repeated_empty_collection_preserves_input_without_calling_parser() {
        let bytes = [0xff];
        let mut remaining = bytes.as_slice();
        let values: Vec<u16> = parse_repeated(&mut remaining, 0, |_| {
            panic!("empty collection must not invoke the parser")
        })
        .unwrap();
        assert_eq!(values, Vec::<u16>::new());
        assert_eq!(remaining, bytes);
    }

    #[test]
    fn repeated_values_collect_into_boxed_slice_and_preserve_suffix() {
        let mut remaining = [0, 1, 0, 2, 0xff].as_slice();
        let values: Box<[u16]> = parse_repeated(&mut remaining, 2, read_u16).unwrap();
        assert_eq!(&*values, [1, 2]);
        assert_eq!(remaining, [0xff]);
    }

    #[test]
    fn repeated_nested_counts_consume_only_their_elements() {
        let mut remaining = [2, 0, 1, 0, 2, 0, 0xff].as_slice();
        let values: Vec<Vec<u16>> = parse_repeated(&mut remaining, 2, |bytes| {
            let count = read_u8(bytes)?;
            parse_repeated(bytes, usize::from(count), read_u16)
        })
        .unwrap();
        assert_eq!(values, [vec![1, 2], vec![]]);
        assert_eq!(remaining, [0xff]);
    }

    #[test]
    fn repeated_failure_stops_parsing_and_retains_unread_input() {
        let mut remaining = [0, 1, 0xff].as_slice();
        let mut calls = 0;
        let result: Result<Vec<_>, _> = parse_repeated(&mut remaining, 3, |bytes| {
            calls += 1;
            read_u16(bytes)
        });
        assert_eq!(result.unwrap_err().kind(), Malformed);
        assert_eq!(calls, 2);
        assert_eq!(remaining, [0xff]);
    }

    #[test]
    fn repeated_payloads_borrow_original_input() {
        let bytes = [1, 0xaa, 2, 0xbb, 0xcc, 0xff];
        let mut remaining = bytes.as_slice();
        let values: Vec<&[u8]> = parse_repeated(&mut remaining, 2, |bytes| {
            let len = read_u8(bytes)?;
            borrow_bytes(bytes, usize::from(len))
        })
        .unwrap();
        assert_eq!(values, [&bytes[1..2], &bytes[3..5]]);
        assert!(std::ptr::eq(values[0].as_ptr(), bytes[1..].as_ptr()));
        assert!(std::ptr::eq(values[1].as_ptr(), bytes[3..].as_ptr()));
        assert_eq!(remaining, [0xff]);
    }

    #[test]
    fn wire_array_advances_input() {
        let mut reader = [0x01, 0x02, 0x03, 0x04].as_slice();
        let buf: [u8; 3] = read_wire(&mut reader).unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04]);
    }

    #[test]
    fn truncated_wire_array_preserves_input() {
        let mut reader = [0x01, 0x02].as_slice();
        let err = read_wire::<[u8; 3]>(&mut reader).unwrap_err();
        assert_eq!(err.kind(), Malformed);
        assert_eq!(reader, [0x01, 0x02]);
    }

    #[test]
    fn typed_tables_borrow_unaligned_input_and_advance() {
        let bytes = [0xff, 0x12, 0x34, 0xab, 0xcd, 0xee];
        let mut remaining = &bytes[1..];
        let table = borrow_slice::<U16>(&mut remaining, 2).unwrap();
        assert_eq!(
            table.iter().map(|value| value.get()).collect::<Vec<_>>(),
            [0x1234, 0xabcd]
        );
        assert!(std::ptr::eq(
            table.as_ptr().cast::<u8>(),
            bytes[1..].as_ptr()
        ));
        assert_eq!(remaining, [0xee]);
    }

    #[test]
    fn empty_typed_table_preserves_input() {
        let bytes = [0x12, 0x34];
        let mut remaining = bytes.as_slice();
        let empty: &[U16] = &[];
        assert_eq!(borrow_slice::<U16>(&mut remaining, 0).unwrap(), empty);
        assert_eq!(remaining, bytes);
    }

    #[test]
    fn typed_tables_reject_unavailable_or_overflowing_lengths() {
        for count in [1, usize::MAX] {
            let bytes = [0];
            let mut remaining = bytes.as_slice();
            let error = borrow_slice::<U16>(&mut remaining, count).unwrap_err();
            assert_eq!(error.kind(), Malformed);
            assert_eq!(remaining, bytes);
        }
    }

    macro_rules! integer_decode_properties {
        ($success:ident, $truncated:ident, $type:ty, $read:ident) => {
            proptest! {
                #[test]
                fn $success(value in any::<$type>(), suffix in prop::collection::vec(any::<u8>(), 0..8)) {
                    let mut bytes = value.to_be_bytes().to_vec();
                    bytes.extend(&suffix);
                    let mut reader = bytes.as_slice();

                    let decoded = super::$read(&mut reader).unwrap();

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

                    let error = super::$read(&mut reader).unwrap_err();

                    prop_assert_eq!(error.kind(), Malformed);
                    prop_assert_eq!(reader, &bytes[..available]);
                }
            }
        };
    }

    integer_decode_properties!(read_u32_success, read_u32_failed, u32, read_u32);
    integer_decode_properties!(read_i32_success, read_i32_failed, i32, read_i32);
    integer_decode_properties!(read_u16_success, read_u16_failed, u16, read_u16);
    integer_decode_properties!(read_i16_success, read_i16_failed, i16, read_i16);
    integer_decode_properties!(read_u8_success, read_u8_failed, u8, read_u8);
    integer_decode_properties!(read_i8_success, read_i8_failed, i8, read_i8);

    proptest! {
        #[test]
        fn miri_read_bytes_success(
            expected in prop::collection::vec(any::<u8>(), 0..64),
            suffix in prop::collection::vec(any::<u8>(), 0..16),
        ) {
            let mut bytes = expected.clone();
            bytes.extend(&suffix);
            let mut reader = bytes.as_slice();

            let decoded = super::borrow_bytes(&mut reader, expected.len()).unwrap();

            prop_assert_eq!(decoded, expected.as_slice());
            prop_assert!(std::ptr::eq(decoded.as_ptr(), bytes.as_ptr()));
            prop_assert_eq!(reader, suffix.as_slice());
        }

        #[test]
        fn miri_read_bytes_failed(
            available in prop::collection::vec(any::<u8>(), 0..64),
            missing in 1usize..16,
        ) {
            let requested = available.len() + missing;
            let mut reader = available.as_slice();

            let error = super::borrow_bytes(&mut reader, requested).unwrap_err();

            prop_assert_eq!(error.kind(), Malformed);
            prop_assert_eq!(reader, available.as_slice());
        }
    }
}
