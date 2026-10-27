use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        constant_pool::{ConstantPool, FieldRefId, MethodRefId},
        references::{FieldRef, FieldReference, MethodRef, MethodReference},
    },
};

/// A method handle.
#[doc = see_jvm_spec!(4, 4, 8)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MethodHandle {
    /// Get an instance field.
    RefGetField(FieldReference) = 1,
    /// Get a static field.
    RefGetStatic(FieldReference) = 2,
    /// Writes to an instance field.
    RefPutField(FieldReference) = 3,
    /// Writes to a static field.
    RefPutStatic(FieldReference) = 4,
    /// Invoke an instance method.
    RefInvokeVirtual(MethodReference) = 5,
    /// Invoke a static method.
    RefInvokeStatic(MethodReference) = 6,
    /// Invoke a special method (e.g., a constructor).
    RefInvokeSpecial(MethodReference) = 7,
    /// The new version a special method (e.g., a constructor).
    RefNewInvokeSpecial(MethodReference) = 8,
    /// Invoke an interface method.
    RefInvokeInterface(MethodReference) = 9,
}

impl MethodHandle {
    /// Borrows this method handle.
    #[must_use]
    pub const fn as_borrowed(&self) -> MethodHndl<'_> {
        match self {
            Self::RefGetField(value) => MethodHndl::RefGetField(value.as_borrowed()),
            Self::RefGetStatic(value) => MethodHndl::RefGetStatic(value.as_borrowed()),
            Self::RefPutField(value) => MethodHndl::RefPutField(value.as_borrowed()),
            Self::RefPutStatic(value) => MethodHndl::RefPutStatic(value.as_borrowed()),
            Self::RefInvokeVirtual(value) => MethodHndl::RefInvokeVirtual(value.as_borrowed()),
            Self::RefInvokeStatic(value) => MethodHndl::RefInvokeStatic(value.as_borrowed()),
            Self::RefInvokeSpecial(value) => MethodHndl::RefInvokeSpecial(value.as_borrowed()),
            Self::RefNewInvokeSpecial(value) => {
                MethodHndl::RefNewInvokeSpecial(value.as_borrowed())
            }
            Self::RefInvokeInterface(value) => MethodHndl::RefInvokeInterface(value.as_borrowed()),
        }
    }

    /// Gets the reference kind of this method handle.
    #[must_use]
    pub const fn reference_kind(&self) -> u8 {
        self.as_borrowed().reference_kind()
    }
}

/// A borrowed method handle.
#[doc = see_jvm_spec!(4, 4, 8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodHndl<'pool> {
    /// Get an instance field.
    RefGetField(FieldRef<'pool>),
    /// Get a static field.
    RefGetStatic(FieldRef<'pool>),
    /// Write to an instance field.
    RefPutField(FieldRef<'pool>),
    /// Write to a static field.
    RefPutStatic(FieldRef<'pool>),
    /// Invoke an instance method.
    RefInvokeVirtual(MethodRef<'pool>),
    /// Invoke a static method.
    RefInvokeStatic(MethodRef<'pool>),
    /// Invoke a special method.
    RefInvokeSpecial(MethodRef<'pool>),
    /// Invoke a constructor.
    RefNewInvokeSpecial(MethodRef<'pool>),
    /// Invoke an interface method.
    RefInvokeInterface(MethodRef<'pool>),
}

impl MethodHndl<'_> {
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
    pub fn as_borrowed(self, pool: &ConstantPool) -> MethodHndl<'_> {
        match self {
            Self::GetField(id) => MethodHndl::RefGetField(pool.field_ref(id)),
            Self::GetStatic(id) => MethodHndl::RefGetStatic(pool.field_ref(id)),
            Self::PutField(id) => MethodHndl::RefPutField(pool.field_ref(id)),
            Self::PutStatic(id) => MethodHndl::RefPutStatic(pool.field_ref(id)),
            Self::InvokeVirtual(id) => MethodHndl::RefInvokeVirtual(pool.method_ref(id)),
            Self::InvokeStatic(id) => MethodHndl::RefInvokeStatic(pool.method_ref(id)),
            Self::InvokeSpecial(id) => MethodHndl::RefInvokeSpecial(pool.method_ref(id)),
            Self::NewInvokeSpecial(id) => MethodHndl::RefNewInvokeSpecial(pool.method_ref(id)),
            Self::InvokeInterface(id) => MethodHndl::RefInvokeInterface(pool.method_ref(id)),
        }
    }
}
