use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        constant_pool::{ConstantPool, FieldRefId, MethodRefId},
        references::{FieldRef, FieldRefView, MethodRef, MethodRefView},
    },
};

/// A method handle.
#[doc = see_jvm_spec!(4, 4, 8)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MethodHandle {
    /// Get an instance field.
    RefGetField(FieldRef) = 1,
    /// Get a static field.
    RefGetStatic(FieldRef) = 2,
    /// Writes to an instance field.
    RefPutField(FieldRef) = 3,
    /// Writes to a static field.
    RefPutStatic(FieldRef) = 4,
    /// Invoke an instance method.
    RefInvokeVirtual(MethodRef) = 5,
    /// Invoke a static method.
    RefInvokeStatic(MethodRef) = 6,
    /// Invoke a special method (e.g., a constructor).
    RefInvokeSpecial(MethodRef) = 7,
    /// The new version a special method (e.g., a constructor).
    RefNewInvokeSpecial(MethodRef) = 8,
    /// Invoke an interface method.
    RefInvokeInterface(MethodRef) = 9,
}

impl MethodHandle {
    /// Gets the reference kind of this method handle.
    #[must_use]
    pub const fn reference_kind(&self) -> u8 {
        match self {
            Self::RefGetField(..) => 1,
            Self::RefGetStatic(..) => 2,
            Self::RefPutField(..) => 3,
            Self::RefPutStatic(..) => 4,
            Self::RefInvokeVirtual(..) => 5,
            Self::RefInvokeStatic(..) => 6,
            Self::RefInvokeSpecial(..) => 7,
            Self::RefNewInvokeSpecial(..) => 8,
            Self::RefInvokeInterface(..) => 9,
        }
    }
}

/// A borrowed method handle.
#[doc = see_jvm_spec!(4, 4, 8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodHandleView<'pool> {
    /// Get an instance field.
    RefGetField(FieldRefView<'pool>),
    /// Get a static field.
    RefGetStatic(FieldRefView<'pool>),
    /// Write to an instance field.
    RefPutField(FieldRefView<'pool>),
    /// Write to a static field.
    RefPutStatic(FieldRefView<'pool>),
    /// Invoke an instance method.
    RefInvokeVirtual(MethodRefView<'pool>),
    /// Invoke a static method.
    RefInvokeStatic(MethodRefView<'pool>),
    /// Invoke a special method.
    RefInvokeSpecial(MethodRefView<'pool>),
    /// Invoke a constructor.
    RefNewInvokeSpecial(MethodRefView<'pool>),
    /// Invoke an interface method.
    RefInvokeInterface(MethodRefView<'pool>),
}

impl MethodHandleView<'_> {
    /// Converts this borrowed method handle into an owned value.
    #[must_use]
    pub fn into_owned(self) -> MethodHandle {
        match self {
            Self::RefGetField(value) => MethodHandle::RefGetField(value.into_owned()),
            Self::RefGetStatic(value) => MethodHandle::RefGetStatic(value.into_owned()),
            Self::RefPutField(value) => MethodHandle::RefPutField(value.into_owned()),
            Self::RefPutStatic(value) => MethodHandle::RefPutStatic(value.into_owned()),
            Self::RefInvokeVirtual(value) => MethodHandle::RefInvokeVirtual(value.into_owned()),
            Self::RefInvokeStatic(value) => MethodHandle::RefInvokeStatic(value.into_owned()),
            Self::RefInvokeSpecial(value) => MethodHandle::RefInvokeSpecial(value.into_owned()),
            Self::RefNewInvokeSpecial(value) => {
                MethodHandle::RefNewInvokeSpecial(value.into_owned())
            }
            Self::RefInvokeInterface(value) => MethodHandle::RefInvokeInterface(value.into_owned()),
        }
    }

    /// Gets the reference kind of this method handle.
    #[must_use]
    pub const fn reference_kind(self) -> u8 {
        match self {
            Self::RefGetField(..) => 1,
            Self::RefGetStatic(..) => 2,
            Self::RefPutField(..) => 3,
            Self::RefPutStatic(..) => 4,
            Self::RefInvokeVirtual(..) => 5,
            Self::RefInvokeStatic(..) => 6,
            Self::RefInvokeSpecial(..) => 7,
            Self::RefNewInvokeSpecial(..) => 8,
            Self::RefInvokeInterface(..) => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodHandleData {
    GetField(FieldRefId),
    GetStatic(FieldRefId),
    PutField(FieldRefId),
    PutStatic(FieldRefId),
    InvokeVirtual(MethodRefId),
    InvokeStatic(MethodRefId),
    InvokeSpecial(MethodRefId),
    NewInvokeSpecial(MethodRefId),
    InvokeInterface(MethodRefId),
}

impl MethodHandleData {
    pub fn view(self, pool: &ConstantPool) -> MethodHandleView<'_> {
        match self {
            Self::GetField(id) => MethodHandleView::RefGetField(pool.field_ref(id)),
            Self::GetStatic(id) => MethodHandleView::RefGetStatic(pool.field_ref(id)),
            Self::PutField(id) => MethodHandleView::RefPutField(pool.field_ref(id)),
            Self::PutStatic(id) => MethodHandleView::RefPutStatic(pool.field_ref(id)),
            Self::InvokeVirtual(id) => MethodHandleView::RefInvokeVirtual(pool.method_ref(id)),
            Self::InvokeStatic(id) => MethodHandleView::RefInvokeStatic(pool.method_ref(id)),
            Self::InvokeSpecial(id) => MethodHandleView::RefInvokeSpecial(pool.method_ref(id)),
            Self::NewInvokeSpecial(id) => {
                MethodHandleView::RefNewInvokeSpecial(pool.method_ref(id))
            }
            Self::InvokeInterface(id) => MethodHandleView::RefInvokeInterface(pool.method_ref(id)),
        }
    }
}
