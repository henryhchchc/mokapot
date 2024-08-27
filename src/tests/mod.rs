use proptest::prelude::*;

use crate::{
    jvm::{class, references::ClassRef, Class},
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

impl Default for Class {
    fn default() -> Self {
        Self {
            version: class::Version::Jdk22(false),
            access_flags: class::AccessFlags::empty(),
            binary_name: String::default(),
            super_class: None,
            interfaces: Vec::default(),
            fields: Vec::default(),
            methods: Vec::default(),
            source_file: None,
            inner_classes: Vec::default(),
            enclosing_method: None,
            source_debug_extension: None,
            runtime_visible_annotations: Vec::default(),
            runtime_invisible_annotations: Vec::default(),
            runtime_visible_type_annotations: Vec::default(),
            runtime_invisible_type_annotations: Vec::default(),
            bootstrap_methods: Vec::default(),
            module: None,
            module_packages: Vec::default(),
            module_main_class: None,
            nest_host: None,
            nest_members: Vec::default(),
            permitted_subclasses: Vec::default(),
            is_synthetic: false,
            is_deprecated: false,
            signature: None,
            record: None,
            free_attributes: Vec::default(),
        }
    }
}

pub(crate) fn arb_identifier() -> impl Strategy<Value = String> {
    let arb_ident = prop::string::string_regex(r"[a-zA-Z][\w\$_]*").expect("The regex is invalid");
    prop::collection::vec(arb_ident, 1..10).prop_map(|v| v.join("/"))
}

pub(crate) fn arb_non_array_field_type() -> impl Strategy<Value = FieldType> {
    prop_oneof![
        any::<PrimitiveType>().prop_map(FieldType::Base),
        arb_identifier()
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
