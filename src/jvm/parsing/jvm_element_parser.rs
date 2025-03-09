use bitflags::Flags;

use crate::jvm::class::ConstantPool;

use super::{Context, Error};

pub(super) trait ClassElement: Sized {
    type Raw: Sized;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error>;

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, Error>;
}

impl<T> ClassElement for T
where
    T: Flags<Bits = u16>,
{
    type Raw = u16;

    fn from_raw(raw: Self::Raw, _ctx: &Context) -> Result<Self, Error> {
        T::from_bits(raw).ok_or(Error::UnknownFlags(std::any::type_name::<Self>(), raw))
    }

    fn into_raw(self, _cp: &mut ConstantPool) -> Result<Self::Raw, Error> {
        Ok(self.bits())
    }
}
