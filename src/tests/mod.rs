use proptest::prelude::*;

use crate::{
    jvm::references::ClassRef,
    types::field_type::{FieldType, PrimitiveType},
};

#[rustfmt::skip]
#[must_use]
pub const fn empty_class_with_version(major: u16, minor: u16) -> [u8;40] {
    [
        0xCA, 0xFE, 0xBA, 0xBE, // Magic
        minor.to_be_bytes()[0], minor.to_be_bytes()[1], // Minor version
        major.to_be_bytes()[0], major.to_be_bytes()[1], // Major version
        // Constant pool
        0x00, 0x03, // Constant pool count 2+1
        0x07, // Tag: Class
        0x00, 0x02, // Name index: 2
        0x01, // Tag: Utf8
        0x00, 0x0A, // Length of string: 10
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x57, 0x6F, 0x72, 0x6C, 0x64, // "Helloworld"
        0x00, 0x01, // Access flags: public 
        0x00, 0x01, // This class index
        0x00, 0x01, // Super class index
        0x00, 0x00, // Interfaces count
        0x00, 0x00, // Fields count
        0x00, 0x00, // Methods count
        0x00, 0x00, // Attributes count
    ]
}

pub(crate) fn arb_class_name() -> impl Strategy<Value = String> {
    let arb_ident = prop::string::string_regex(r"[a-zA-Z][\w\$_]*").expect("The regex is invalid");
    prop::collection::vec(arb_ident, 0..10).prop_map(|v| v.join("/"))
}

pub(crate) fn arb_non_array_field_type() -> impl Strategy<Value = FieldType> {
    prop_oneof![
        any::<PrimitiveType>().prop_map(FieldType::Base),
        arb_class_name()
            .prop_map(ClassRef::new)
            .prop_map(FieldType::Object),
    ]
}

prop_compose! {
    fn arb_array_field_type()(
        t in arb_non_array_field_type(),
        dim in 1..=u8::MAX
    ) -> FieldType {
        FieldType::array_of(t, dim)
    }
}

pub(crate) fn arb_field_type() -> impl Strategy<Value = FieldType> {
    prop_oneof![arb_non_array_field_type(), arb_array_field_type()]
}
