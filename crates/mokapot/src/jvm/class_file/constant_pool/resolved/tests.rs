use super::*;
use crate::jvm::{
    ConstantValue,
    class::MethodHandle,
    class_file::constant_pool::RawEntry,
    errors::ParseErrorKind,
    references::{FieldRef, MethodRef},
};
use crate::types::Descriptor;

fn member_pool() -> ConstantPool {
    let raw = RawConstantPool::from_entries([
        RawEntry::Utf8(b"[I"),
        RawEntry::Class {
            name_index: 1.into(),
        },
        RawEntry::Utf8(&[0xed, 0xa0, 0x80]),
        RawEntry::Utf8(b"I"),
        RawEntry::NameAndType {
            name_index: 3.into(),
            descriptor_index: 4.into(),
        },
        RawEntry::FieldRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        },
        RawEntry::FieldRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        },
        RawEntry::NameAndType {
            name_index: 3.into(),
            descriptor_index: 4.into(),
        },
        RawEntry::FieldRef {
            class_index: 2.into(),
            name_and_type_index: 8.into(),
        },
        RawEntry::Utf8(b"(I[Ljava/lang/Object;)V"),
        RawEntry::NameAndType {
            name_index: 3.into(),
            descriptor_index: 10.into(),
        },
        RawEntry::MethodRef {
            class_index: 2.into(),
            name_and_type_index: 11.into(),
        },
        RawEntry::MethodRef {
            class_index: 2.into(),
            name_and_type_index: 11.into(),
        },
        RawEntry::InterfaceMethodRef {
            class_index: 2.into(),
            name_and_type_index: 11.into(),
        },
        RawEntry::NameAndType {
            name_index: 3.into(),
            descriptor_index: 10.into(),
        },
        RawEntry::MethodRef {
            class_index: 2.into(),
            name_and_type_index: 15.into(),
        },
        RawEntry::Dynamic {
            bootstrap_method_attr_index: 1.into(),
            name_and_type_index: 5.into(),
        },
        RawEntry::InvokeDynamic {
            bootstrap_method_attr_index: 2.into(),
            name_and_type_index: 11.into(),
        },
        RawEntry::Long(0.into()),
    ]);
    ConstantPool::from_raw(raw).unwrap()
}

#[test]
fn member_views_share_owners_names_and_descriptors() {
    let pool = member_pool();
    let owner = pool.reference_type(pool.reference_type_id(2).unwrap());
    let name = pool.utf8(pool.utf8_id(3).unwrap());
    let field = pool.field_ref(pool.field_ref_id(6).unwrap());
    for index in [6, 7, 9] {
        let other = pool.field_ref(pool.field_ref_id(index).unwrap());
        assert!(std::ptr::eq(other.owner(), owner));
        assert!(std::ptr::eq(other.name(), name));
        assert!(std::ptr::eq(other.field_type(), field.field_type()));
    }
    let method = pool.method_ref(pool.method_ref_id(12).unwrap());
    for index in [12, 13, 14, 16] {
        let other = pool.method_ref(pool.method_or_interface_ref_id(index).unwrap());
        assert!(std::ptr::eq(other.owner(), owner));
        assert!(std::ptr::eq(other.name(), name));
        assert!(std::ptr::eq(other.descriptor(), method.descriptor()));
    }
    let (bootstrap, dynamic_name, descriptor) =
        pool.invoke_dynamic(pool.invoke_dynamic_id(18).unwrap());
    assert_eq!(bootstrap, 2);
    assert!(std::ptr::eq(dynamic_name, name));
    assert!(std::ptr::eq(descriptor, method.descriptor()));
    assert_eq!(field.name().as_bytes(), &[0xed, 0xa0, 0x80]);
    assert_eq!(FieldRef::from(field).name.as_bytes(), name.as_bytes());
    assert_eq!(MethodRef::from(method).name.as_bytes(), name.as_bytes());
    let ConstantValueView::Dynamic(bootstrap, dynamic_name, ty) =
        pool.constant_value(pool.constant_value_id(17).unwrap())
    else {
        panic!("expected dynamic constant");
    };
    assert_eq!(bootstrap, 1);
    assert!(std::ptr::eq(dynamic_name, name));
    assert!(std::ptr::eq(ty, field.field_type()));
}

#[test]
fn member_views_compare_and_hash_values_across_pools() {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    fn hash(value: impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
    let pool = member_pool();
    let other = member_pool();
    let field = pool.field_ref(pool.field_ref_id(6).unwrap());
    let other_field = other.field_ref(other.field_ref_id(9).unwrap());
    assert_eq!(field, other_field);
    assert_eq!(hash(field), hash(other_field));
    assert_eq!(format!("{field:?}"), format!("{other_field:?}"));
    assert_eq!(field.to_string(), FieldRef::from(field).to_string());
    let method = pool.method_ref(pool.method_ref_id(12).unwrap());
    let other_method = other.method_ref(other.interface_method_ref_id(14).unwrap());
    assert_eq!(method, other_method);
    assert_eq!(hash(method), hash(other_method));
    assert_eq!(format!("{method:?}"), format!("{other_method:?}"));
    assert_eq!(method.to_string(), MethodRef::from(method).to_string());
    let mut different = ConstantPool::new();
    let different_field = different.insert_field_ref(FieldRef {
        name: "different".into(),
        ..field.into()
    });
    let different_method = different.insert_method_ref(MethodRef {
        descriptor: "()I".parse().unwrap(),
        ..method.into()
    });
    assert_ne!(field, different.field_ref(different_field));
    assert_ne!(method, different.method_ref(different_method));
}

#[test]
fn member_ids_reject_invalid_slots_and_wrong_kinds() {
    let pool = member_pool();
    for index in [0, 1, 19, 20, 21, u16::MAX] {
        for result in [
            pool.field_ref_id(index).map(|_| ()),
            pool.method_ref_id(index).map(|_| ()),
            pool.interface_method_ref_id(index).map(|_| ()),
            pool.method_or_interface_ref_id(index).map(|_| ()),
            pool.field_name_and_type_id(index).map(|_| ()),
            pool.method_name_and_type_id(index).map(|_| ()),
            pool.invoke_dynamic_id(index).map(|_| ()),
        ] {
            assert_eq!(result.unwrap_err().kind(), ParseErrorKind::Malformed);
        }
    }
    for result in [
        pool.field_ref_id(12).map(|_| ()),
        pool.method_ref_id(14).map(|_| ()),
        pool.interface_method_ref_id(12).map(|_| ()),
        pool.method_or_interface_ref_id(6).map(|_| ()),
        pool.field_name_and_type_id(11).map(|_| ()),
        pool.method_name_and_type_id(5).map(|_| ()),
    ] {
        assert_eq!(result.unwrap_err().kind(), ParseErrorKind::Malformed);
    }
}

#[test]
fn member_resolution_rejects_invalid_owners_and_name_and_type_kinds() {
    for (owner, field_type, method_type) in [
        (0, 5, 7),
        (1, 5, 7),
        (99, 5, 7),
        (2, 0, 0),
        (2, 1, 1),
        (2, 7, 5),
        (2, 99, 99),
    ] {
        for entry in [
            RawEntry::FieldRef {
                class_index: owner.into(),
                name_and_type_index: field_type.into(),
            },
            RawEntry::MethodRef {
                class_index: owner.into(),
                name_and_type_index: method_type.into(),
            },
            RawEntry::InterfaceMethodRef {
                class_index: owner.into(),
                name_and_type_index: method_type.into(),
            },
        ] {
            let raw = RawConstantPool::from_entries([
                RawEntry::Utf8(b"a/b/C"),
                RawEntry::Class {
                    name_index: 1.into(),
                },
                RawEntry::Utf8(b"name"),
                RawEntry::Utf8(b"I"),
                RawEntry::NameAndType {
                    name_index: 3.into(),
                    descriptor_index: 4.into(),
                },
                RawEntry::Utf8(b"()V"),
                RawEntry::NameAndType {
                    name_index: 3.into(),
                    descriptor_index: 6.into(),
                },
                entry,
            ]);
            assert_eq!(
                ConstantPool::from_raw(raw).unwrap_err().kind(),
                ParseErrorKind::Malformed
            );
        }
    }
}

#[test]
fn checked_ids_preserve_values_and_share_parsed_descriptors() {
    let raw = RawConstantPool::from_entries([
        RawEntry::Utf8(b"()V"),
        RawEntry::Long(0.into()),
        RawEntry::Utf8(&[0xed, 0xa0, 0x80]),
        RawEntry::Utf8(b"I"),
        RawEntry::Utf8(b"V"),
    ]);
    let mut pool = ConstantPool::from_raw(raw).unwrap();
    for index in [0, 2, 3, 7] {
        assert_eq!(
            pool.utf8_id(index).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
        assert_eq!(
            pool.intern_method_descriptor(index).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
    }
    let name = pool.utf8_id(4).unwrap();
    let descriptor = pool.intern_method_descriptor(1).unwrap();
    assert_eq!(pool.intern_method_descriptor(1).unwrap(), descriptor);
    assert_eq!(pool.method_descriptors.values.len(), 1);
    let field_type = pool.intern_field_type(5).unwrap();
    assert_eq!(pool.intern_field_type(5).unwrap(), field_type);
    let return_type = pool.intern_return_type(6).unwrap();
    assert_eq!(pool.intern_return_type(6).unwrap(), return_type);
    let constant = pool.constant_value_id(2).unwrap();
    assert_eq!(pool.constant_value_id(2).unwrap(), constant);
    for index in [0, 3, 7] {
        for result in [
            pool.intern_field_type(index).map(|_| ()),
            pool.intern_return_type(index).map(|_| ()),
            pool.constant_value_id(index).map(|_| ()),
        ] {
            assert_eq!(result.unwrap_err().kind(), ParseErrorKind::Malformed);
        }
    }
    let pool = pool.finish();
    assert_eq!(pool.utf8(name).as_bytes(), &[0xed, 0xa0, 0x80]);
    assert_eq!(pool.method_descriptor(descriptor).descriptor(), "()V");
    assert_eq!(pool.field_type(field_type).descriptor(), "I");
    assert_eq!(pool.return_type(return_type), &ReturnType::Void);
    assert_eq!(pool.constant_value(constant), ConstantValueView::Long(0));
}

#[test]
fn class_and_package_names_parse_with_distinct_tags() {
    // Utf8 "a/b/C", Class #1, Package #1.
    let bytes = [1, 0, 5, b'a', b'/', b'b', b'/', b'C', 7, 0, 1, 20, 0, 1];
    let pool = ConstantPool::from_bytes(bytes.as_slice(), 4).unwrap();
    let class_name: ClassName = "a/b/C".parse().unwrap();
    let package_name: PackageName = "a/b/C".parse().unwrap();
    assert_eq!(pool.count(), 4);
    assert_eq!(pool.get_class_name(2).unwrap(), class_name);
    assert_eq!(
        pool.package_name(pool.package_id(3).unwrap()).clone(),
        package_name
    );
    assert_eq!(
        pool.get_class_name(3).unwrap_err().kind(),
        ParseErrorKind::Malformed,
    );
    assert_eq!(
        pool.package_id(2).unwrap_err().kind(),
        ParseErrorKind::Malformed,
    );
}

#[test]
fn class_name_accessor_rejects_descriptors() {
    for descriptor in ["[I", "[La/b/C;", "La/b/C;"] {
        let reference_type: ReferenceType = descriptor.parse().unwrap();
        let pool = RawConstantPool::from_entries([
            RawEntry::Utf8(descriptor.as_bytes()),
            RawEntry::Class {
                name_index: 1.into(),
            },
        ]);
        let index = 2;

        let pool = ConstantPool::from_raw(pool).unwrap();
        assert_eq!(pool.get_type_ref(index).unwrap(), reference_type);
        assert_eq!(
            pool.get_class_name(index).unwrap_err().kind(),
            ParseErrorKind::Malformed,
        );
    }
}

fn method_handle_pool(kind: u8, tag: u8) -> Result<ConstantPool, ParseError> {
    let member = match tag {
        9 => RawEntry::FieldRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        },
        10 => RawEntry::MethodRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        },
        11 => RawEntry::InterfaceMethodRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        },
        _ => unreachable!(),
    };
    ConstantPool::from_raw(RawConstantPool::from_entries([
        RawEntry::Utf8(b"a/b/I"),
        RawEntry::Class {
            name_index: 1.into(),
        },
        RawEntry::Utf8(b"m"),
        RawEntry::Utf8(if tag == 9 { b"I" } else { b"()V" }),
        RawEntry::NameAndType {
            name_index: 3.into(),
            descriptor_index: 4.into(),
        },
        member,
        RawEntry::MethodHandle {
            reference_kind: kind,
            reference_index: 6.into(),
        },
    ]))
}

#[test]
fn method_handles_validate_reference_kinds_and_tags() {
    for kind in 0..=10 {
        for tag in [9, 10, 11] {
            let valid = match kind {
                1..=4 => tag == 9,
                5 | 8 => tag == 10,
                6 | 7 => matches!(tag, 10 | 11),
                9 => tag == 11,
                _ => false,
            };
            match method_handle_pool(kind, tag) {
                Ok(pool) => {
                    assert!(valid, "kind {kind}, tag {tag}");
                    let handle = pool.method_handle(pool.method_handle_id(7).unwrap());
                    assert_eq!(handle.reference_kind(), kind);
                    let owned = MethodHandle::from(handle);
                    assert_eq!(owned.reference_kind(), kind);
                    assert_eq!(
                        ConstantValue::from(pool.get_constant_value(7).unwrap()),
                        ConstantValue::Handle(owned)
                    );
                }
                Err(error) => {
                    assert!(!valid, "kind {kind}, tag {tag}");
                    assert_eq!(error.kind(), ParseErrorKind::Malformed);
                }
            }
        }
    }
}

#[test]
fn method_handle_views_share_references_and_compare_values() {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    fn hash(value: impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
    let pool = method_handle_pool(9, 11).unwrap();
    let other = method_handle_pool(9, 11).unwrap();
    let handle = pool.method_handle(pool.method_handle_id(7).unwrap());
    let other_handle = other.method_handle(other.method_handle_id(7).unwrap());
    let MethodHandleView::RefInvokeInterface(method) = handle else {
        panic!()
    };
    let member = pool.method_ref(pool.interface_method_ref_id(6).unwrap());
    assert!(std::ptr::eq(method.owner(), member.owner()));
    assert!(std::ptr::eq(method.name(), member.name()));
    assert!(std::ptr::eq(method.descriptor(), member.descriptor()));
    assert_eq!(handle, other_handle);
    assert_eq!(hash(handle), hash(other_handle));
    assert_ne!(handle, MethodHandleView::RefInvokeStatic(method));
}

#[test]
fn forward_references_resolve_typed_values_and_keep_indices() {
    let raw = RawConstantPool::from_entries([
        RawEntry::MethodHandle {
            reference_kind: 5,
            reference_index: 2.into(),
        },
        RawEntry::MethodRef {
            class_index: 3.into(),
            name_and_type_index: 4.into(),
        },
        RawEntry::Class {
            name_index: 5.into(),
        },
        RawEntry::NameAndType {
            name_index: 6.into(),
            descriptor_index: 7.into(),
        },
        RawEntry::Utf8(b"a/b/C".as_slice()),
        RawEntry::Utf8(b"m".as_slice()),
        RawEntry::Utf8(b"(I)V".as_slice()),
        RawEntry::Long(42.into()),
        RawEntry::Dynamic {
            bootstrap_method_attr_index: 3.into(),
            name_and_type_index: 12.into(),
        },
        RawEntry::InvokeDynamic {
            bootstrap_method_attr_index: 4.into(),
            name_and_type_index: 4.into(),
        },
        RawEntry::NameAndType {
            name_index: 6.into(),
            descriptor_index: 13.into(),
        },
        RawEntry::Utf8(b"I".as_slice()),
    ]);
    let count = raw.count();
    let pool = ConstantPool::from_raw(raw).unwrap();
    assert_eq!(pool.count(), count);
    assert_eq!(pool.get_entry(0), None);
    assert_eq!(pool.get_entry(9), None);
    let method = MethodRef::from(pool.method_ref(pool.method_ref_id(2).unwrap()));
    assert_eq!(method.owner, "a/b/C".parse::<ReferenceType>().unwrap());
    assert_eq!(method.descriptor, "(I)V".parse().unwrap());
    assert_eq!(
        MethodHandle::from(pool.method_handle(pool.method_handle_id(1).unwrap())),
        MethodHandle::RefInvokeVirtual(method)
    );
    assert!(matches!(
        pool.get_entry(10),
        Some(Entry::Dynamic {
            bootstrap_method_attr_index: 3,
            ..
        })
    ));
    assert!(matches!(
        pool.get_entry(11),
        Some(Entry::InvokeDynamic {
            bootstrap_method_attr_index: 4,
            ..
        })
    ));
}

#[test]
fn resolution_rejects_invalid_dependencies_and_descriptors() {
    for entry in [
        RawEntry::Class {
            name_index: 0.into(),
        },
        RawEntry::Class {
            name_index: 3.into(),
        },
        RawEntry::Class {
            name_index: 2.into(),
        },
        RawEntry::Class {
            name_index: u16::MAX.into(),
        },
        RawEntry::MethodType {
            descriptor_index: 1.into(),
        },
        RawEntry::MethodHandle {
            reference_kind: 0,
            reference_index: 1.into(),
        },
    ] {
        let raw = RawConstantPool::from_entries([
            RawEntry::Utf8(b"I".as_slice()),
            RawEntry::Long(42.into()),
            entry,
        ]);
        assert_eq!(
            ConstantPool::from_raw(raw).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
    }
}

#[test]
fn string_entries_preserve_jvm_bytes() {
    let bytes = b"a\xc0\x80\xed\xa0\xbd\xed\xb8\x80\xed\xa0\x80";
    let expected = JavaString::from_cesu8(bytes.to_vec()).unwrap();
    let raw = RawConstantPool::from_entries([
        RawEntry::Utf8(bytes),
        RawEntry::String {
            string_index: 1.into(),
        },
    ]);
    let pool = ConstantPool::from_raw(raw).unwrap();
    assert_eq!(pool.get_utf8(1).unwrap().as_bytes(), bytes);
    let constant = pool.constant_value(pool.constant_value_id(2).unwrap());
    let ConstantValueView::String(value) = constant else {
        panic!("expected string constant");
    };
    assert!(std::ptr::eq(value, pool.utf8(pool.utf8_id(1).unwrap())));
    assert_eq!(
        ConstantValue::from(constant),
        ConstantValue::String(expected)
    );
}

#[test]
fn malformed_utf8_entries_are_rejected_without_references() {
    let raw = RawConstantPool::from_entries([RawEntry::Utf8(b"\xff")]);
    assert_eq!(
        ConstantPool::from_raw(raw).unwrap_err().kind(),
        ParseErrorKind::Malformed
    );
}
