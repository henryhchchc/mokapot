use std::{io::Read, iter::repeat_with};

use bitflags::Flags;
use itertools::Itertools;

use super::{
    reader_utils::{FromReader, ValueReaderExt},
    Context, Error,
};

pub(super) trait FromRaw: Sized {
    type Raw: Sized;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error>;
}

impl<T> JvmElement for T
where
    T: FromRaw,
    <T as FromRaw>::Raw: FromReader,
{
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let raw = <T::Raw as FromReader>::from_reader(reader)?;
        T::from_raw(raw, ctx)
    }
}

pub(super) trait JvmElement: Sized {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error>;

    fn parse_vec<C, R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Vec<Self>, Error>
    where
        C: Into<usize> + FromReader,
    {
        let count: C = reader.read_value()?;
        let count: usize = count.into();
        repeat_with(|| Self::parse(reader, ctx))
            .take(count)
            .try_collect()
    }
}

impl JvmElement for String {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &Context) -> Result<Self, Error> {
        let utf_8_index = reader.read_value()?;
        ctx.constant_pool.get_str(utf_8_index).map(str::to_owned)
    }
}

#[inline]
pub(super) fn parse_flags<F, R>(reader: &mut R) -> Result<F, Error>
where
    R: Read + ?Sized,
    F: Flags<Bits = u16>,
{
    let flag_bits = reader.read_value()?;
    F::from_bits(flag_bits).ok_or(Error::UnknownFlags(std::any::type_name::<F>(), flag_bits))
}
