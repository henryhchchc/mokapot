//! JVM fields and constant values.

use super::{references::FieldRef, Field};

impl Field {
    /// Creates a [`FieldRef`] referring to the field.
    #[must_use]
    pub fn as_ref(&self) -> FieldRef {
        FieldRef {
            owner: self.owner.clone(),
            name: self.name.clone(),
            field_type: self.field_type.clone(),
        }
    }
}

/// A generic type signature for a field, a formal parameter, a local variable, or a record component.
pub type Signature = String;

use bitflags::bitflags;

bitflags! {
    /// The access flags of a field.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC = 0x0001;
        /// Declared `private`; accessible only within the defining class and other classes belonging to the same nest.
        const PRIVATE = 0x0002;
        /// Declared `protected`; may be accessed within subclasses.
        const PROTECTED = 0x0004;
        /// Declared `static`.
        const STATIC = 0x0008;
        /// Declared `final`; never directly assigned to after object construction.
        const FINAL = 0x0010;
        /// Declared `volatile`; cannot be cached.
        const VOLATILE = 0x0040;
        /// Declared `transient`; not written or read by a persistent object manager.
        const TRANSIENT = 0x0080;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
        /// Declared as an element of an `enum` class.
        const ENUM = 0x4000;
    }
}

#[cfg(test)]
mod test {

    use proptest::prelude::*;

    use super::AccessFlags;

    fn arb_access_flag() -> impl Strategy<Value = AccessFlags> {
        prop_oneof![
            Just(AccessFlags::PUBLIC),
            Just(AccessFlags::PRIVATE),
            Just(AccessFlags::PROTECTED),
            Just(AccessFlags::STATIC),
            Just(AccessFlags::FINAL),
            Just(AccessFlags::VOLATILE),
            Just(AccessFlags::TRANSIENT),
            Just(AccessFlags::SYNTHETIC),
            Just(AccessFlags::ENUM),
        ]
    }

    proptest! {

        #[test]
        fn access_flags_bit_no_overlap(
            lhs in arb_access_flag(),
            rhs in arb_access_flag()
        ){
            prop_assume!(lhs != rhs);
            assert_eq!(lhs.bits() & rhs.bits(), 0);
        }
    }
}
