use std::{io::Read, iter::repeat_with};

use bitflags::Flags;
use itertools::Itertools;

use super::{
    parsing_context::ParsingContext,
    reader_utils::{Readable, ValueReaderExt},
    Error,
};

pub(crate) trait JvmElement
where
    Self: Sized,
{
    fn parse<R: Read>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error>;

    fn parse_vec<C, R: Read>(reader: &mut R, ctx: &ParsingContext) -> Result<Vec<Self>, Error>
    where
        C: Into<usize> + Readable,
    {
        let count: C = reader.read_value()?;
        let count: usize = count.into();
        repeat_with(|| Self::parse(reader, ctx))
            .take(count)
            .try_collect()
    }
}

impl JvmElement for String {
    fn parse<R: Read>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
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
