//! JVM bootstrap methods.

use crate::jvm::{
    Class, ConstantValue,
    class::MethodHandle,
    class_file::constant_pool::{ConstantValueId, MethodHandleId},
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
    pub fn method(&self) -> &'class MethodHandle {
        self.class.constant_pool.method_handle(self.data.method)
    }

    /// The constants passed to the bootstrap method.
    pub fn arguments(
        self,
    ) -> impl ExactSizeIterator<Item = &'class ConstantValue> + DoubleEndedIterator {
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

impl std::fmt::Debug for BootstrapMethod<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("BootstrapMethod");
        debug.field("method", self.method());
        debug.field("arguments", &self.arguments().collect::<Vec<_>>());
        debug.finish()
    }
}
