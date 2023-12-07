use std::io::Read;

use bitflags::Flags;

use crate::jvm::{ClassFileParsingError, ClassFileParsingResult};

use super::{parsing_context::ParsingContext, reader_utils::read_u16};

pub(crate) trait ParseJvmElement<R>
where
    R: Read,
    Self: Sized,
{
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self>;
}

pub(crate) fn parse_jvm_element<R, T>(
    reader: &mut R,
    ctx: &ParsingContext,
) -> ClassFileParsingResult<T>
where
    R: std::io::Read,
    T: ParseJvmElement<R>,
{
    T::parse(reader, ctx)
}

impl<T, R: std::io::Read> ParseJvmElement<R> for Vec<T>
where
    T: ParseJvmElement<R>,
{
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let count = read_u16(reader)?;
        let mut result = Vec::with_capacity(count as usize);
        for _ in 0..count {
            result.push(parse_jvm_element(reader, ctx)?);
        }
        Ok(result)
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for String {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let utf_8_index = read_u16(reader)?;
        ctx.constant_pool.get_str(utf_8_index).map(str::to_owned)
    }
}

pub(crate) fn parse_flags<F, R>(reader: &mut R) -> ClassFileParsingResult<F>
where
    R: std::io::Read,
    F: Flags<Bits = u16>,
{
    let flag_bits = read_u16(reader)?;
    F::from_bits(flag_bits).ok_or(ClassFileParsingError::UnknownFlags(
        flag_bits,
        std::any::type_name::<F>(),
    ))
}
