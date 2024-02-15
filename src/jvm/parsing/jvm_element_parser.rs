use std::{io::Read, iter::repeat_with};

use bitflags::Flags;
use itertools::Itertools;

use super::{
    parsing_context::ParsingContext,
    reader_utils::{ClassReader, ReadFromReader},
    Error,
};

pub(crate) trait ParseJvmElement<R>
where
    R: Read,
    Self: Sized,
{
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error>;
}

#[inline]
pub(crate) fn parse_jvm_element<R, T>(
    reader: &mut R,
    ctx: &ParsingContext,
) -> Result<T, Error>
where
    R: Read,
    T: ParseJvmElement<R>,
{
    T::parse(reader, ctx)
}

#[inline]
pub(crate) fn parse_jvm_element_vec<C, T, R>(
    reader: &mut R,
    ctx: &ParsingContext,
) -> Result<Vec<T>, Error>
where
    R: Read,
    T: ParseJvmElement<R>,
    C: Into<usize> + ReadFromReader<R>,
{
    let count: C = reader.read_value()?;
    let count: usize = count.into();
    repeat_with(|| parse_jvm_element(reader, ctx))
        .take(count)
        .try_collect()
}

macro_rules! parse_jvm {
    ($size_type: tt, $reader: ident, $ctx: ident) => {
        crate::jvm::parsing::jvm_element_parser::parse_jvm_element_vec::<$size_type, _, _>(
            $reader, $ctx,
        )
    };
    ($reader: ident, $ctx: ident) => {
        crate::jvm::parsing::jvm_element_parser::parse_jvm_element::<_, _>($reader, $ctx)
    };
}

pub(crate) use parse_jvm;

impl<R: Read> ParseJvmElement<R> for String {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let utf_8_index = reader.read_value()?;
        ctx.constant_pool.get_str(utf_8_index).map(str::to_owned)
    }
}

#[inline]
pub(crate) fn parse_flags<F, R>(reader: &mut R) -> Result<F, Error>
where
    R: Read,
    F: Flags<Bits = u16>,
{
    let flag_bits = reader.read_value()?;
    F::from_bits(flag_bits).ok_or(Error::UnknownFlags(flag_bits, std::any::type_name::<F>()))
}
