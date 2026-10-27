//! JVM bootstrap methods.

use std::{fmt, iter::FusedIterator};

use crate::jvm::{
    Class, ConstantVal, DebugList,
    class::MethodHndl,
    constant_pool::{ConstantValueId, MethodHandleId},
};

#[derive(Debug, Clone)]
pub(crate) struct BootstrapMethodData {
    pub method: MethodHandleId,
    pub arguments: Box<[ConstantValueId]>,
}

/// A bootstrap method declared by a JVM class.
#[derive(Clone, Copy)]
pub struct BootstrapMethod<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class BootstrapMethodData,
}

impl<'class> BootstrapMethod<'class> {
    /// The class declaring this bootstrap method.
    #[must_use]
    pub const fn owner(&self) -> &'class Class {
        self.class
    }

    /// The method handle invoked as the bootstrap method.
    #[must_use]
    pub fn method(&self) -> MethodHndl<'class> {
        self.class.constant_pool.method_handle(self.data.method)
    }

    /// The constants passed to the bootstrap method.
    pub fn arguments(
        self,
    ) -> impl ExactSizeIterator<Item = ConstantVal<'class>> + DoubleEndedIterator + FusedIterator + Clone
    {
        self.data
            .arguments
            .iter()
            .map(|id| self.class.constant_pool.constant_value(*id))
    }
}

impl PartialEq for BootstrapMethod<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.method() == other.method() && self.arguments().eq(other.arguments())
    }
}

impl Eq for BootstrapMethod<'_> {}

impl fmt::Debug for BootstrapMethod<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("BootstrapMethod");
        debug.field("method", &self.method());
        debug.field("arguments", &DebugList::new(|| self.arguments()));
        debug.finish()
    }
}
