use crate::jvm::constant_pool::pool_index;

use super::*;
use crate::ir::test::prelude::assert_matches;
use crate::jvm::{
    ConstantVal, ConstantValue, JavaString, class::MethodHandle,
    class_file::constant_pool::RawEntry, errors::ParseErrorKind,
};
use crate::types::Descriptor;
use crate::types::class_name::ClassName;

#[test]
fn resolution_context_preserves_structured_error() {
    use std::error::Error;

    let raw = RawConstantPool::from_entries([RawEntry::Class {
        name: pool_index(2),
    }]);
    let error = resolve(raw).unwrap_err();
    assert_eq!(
        error.kind(),
        ParseErrorKind::InvalidConstantPoolIndex { index: 2 }
    );
    let context = error.source().unwrap();
    assert!(context.source().is_none());
}

#[test]
fn raw_pool_rejects_zero_references() {
    let count = 2_u16.try_into().unwrap();
    for mut bytes in [
        &[7, 0, 0][..],
        &[9, 0, 0, 0, 1],
        &[15, 1, 0, 0],
        &[17, 0, 0, 0, 0],
    ] {
        let error = RawConstantPool::parse_prefix(&mut bytes, count).unwrap_err();
        assert_eq!(
            error.kind(),
            ParseErrorKind::InvalidConstantPoolIndex { index: 0 }
        );
    }
}

#[test]
fn resolves_and_validates_module_names() {
    for name in [b"java.base".as_slice(), br"a\:b", b"a:b", b"a\xc0\x80b"] {
        let raw = RawConstantPool::from_entries([
            RawEntry::Utf8(name),
            RawEntry::Module {
                name: pool_index(1),
            },
        ]);
        let result = resolve(raw);
        if matches!(name, b"a:b" | b"a\xc0\x80b") {
            assert_eq!(result.unwrap_err().kind(), ParseErrorKind::Malformed);
        } else {
            let pool = result.unwrap();
            let id = pool.module_id(pool_index(2)).unwrap();
            assert_eq!(pool.module_name(id).as_java_str().as_bytes(), name);
        }
    }
}

#[test]
fn member_views_resolve_owners_names_and_descriptors() {
    let raw = RawConstantPool::from_entries([
        RawEntry::Utf8(b"[I"),
        RawEntry::Class {
            name: pool_index(1),
        },
        RawEntry::Utf8(&[0xed, 0xa0, 0x80]),
        RawEntry::Utf8(b"I"),
        RawEntry::NameAndType {
            name: pool_index(3),
            descriptor: pool_index(4),
        },
        RawEntry::FieldRef {
            class: pool_index(2),
            name_and_type: pool_index(5),
        },
        RawEntry::Utf8(b"(I[Ljava/lang/Object;)V"),
        RawEntry::NameAndType {
            name: pool_index(3),
            descriptor: pool_index(7),
        },
        RawEntry::MethodRef {
            class: pool_index(2),
            name_and_type: pool_index(8),
        },
        RawEntry::InterfaceMethodRef {
            class: pool_index(2),
            name_and_type: pool_index(8),
        },
        RawEntry::Dynamic {
            bootstrap_method_attr_index: 1.into(),
            name_and_type: pool_index(5),
        },
        RawEntry::InvokeDynamic {
            bootstrap_method_attr_index: 2.into(),
            name_and_type: pool_index(8),
        },
    ]);
    let pool = resolve(raw).unwrap().finish();
    let name = pool.utf8(pool.utf8_id(pool_index(3)).unwrap());
    let field = pool.field_ref(pool.field_ref_id(pool_index(6)).unwrap());
    assert_eq!(field.owner().descriptor(), "[I");
    assert_eq!(field.name().as_bytes(), &[0xed, 0xa0, 0x80]);
    assert_eq!(field.field_type().descriptor(), "I");
    assert_eq!(field.into_owned().name.as_bytes(), name.as_bytes());

    for index in [9, 10] {
        let method = pool.method_ref(pool.method_or_interface_ref_id(pool_index(index)).unwrap());
        assert_eq!(method.owner().descriptor(), "[I");
        assert_eq!(method.name(), name);
        assert_eq!(method.descriptor().descriptor(), "(I[Ljava/lang/Object;)V");
        assert_eq!(method.into_owned().name.as_bytes(), name.as_bytes());
    }
    let (bootstrap, dynamic_name, descriptor) =
        pool.invoke_dynamic(pool.invoke_dynamic_id(pool_index(12)).unwrap());
    assert_eq!(bootstrap, 2);
    assert_eq!(dynamic_name, name);
    assert_eq!(descriptor.descriptor(), "(I[Ljava/lang/Object;)V");
    let ConstantVal::Dynamic(bootstrap, dynamic_name, ty) =
        pool.constant_value(pool.constant_value_id(pool_index(11)).unwrap())
    else {
        panic!("expected dynamic constant");
    };
    assert_eq!(bootstrap, 1);
    assert_eq!(dynamic_name, name);
    assert_eq!(ty.descriptor(), "I");
}

#[test]
fn member_resolution_rejects_invalid_owners_and_name_and_type_kinds() {
    for (owner, field_type, method_type) in [(1, 5, 7), (99, 5, 7), (2, 7, 5)] {
        for entry in [
            RawEntry::FieldRef {
                class: pool_index(owner),
                name_and_type: pool_index(field_type),
            },
            RawEntry::MethodRef {
                class: pool_index(owner),
                name_and_type: pool_index(method_type),
            },
            RawEntry::InterfaceMethodRef {
                class: pool_index(owner),
                name_and_type: pool_index(method_type),
            },
        ] {
            let raw = RawConstantPool::from_entries([
                RawEntry::Utf8(b"a/b/C"),
                RawEntry::Class {
                    name: pool_index(1),
                },
                RawEntry::Utf8(b"name"),
                RawEntry::Utf8(b"I"),
                RawEntry::NameAndType {
                    name: pool_index(3),
                    descriptor: pool_index(4),
                },
                RawEntry::Utf8(b"()V"),
                RawEntry::NameAndType {
                    name: pool_index(3),
                    descriptor: pool_index(6),
                },
                entry,
            ]);
            assert_matches!(
                resolve(raw).unwrap_err().kind(),
                ParseErrorKind::InvalidConstantPoolIndex { index: 99 }
                    | ParseErrorKind::UnexpectedConstantPoolEntry {
                        index: 1 | 5 | 7,
                        ..
                    }
            );
        }
    }
}

#[test]
fn checked_ids_preserve_values_and_reject_invalid_indices() {
    let raw = RawConstantPool::from_entries([
        RawEntry::Utf8(b"()V"),
        RawEntry::Long(0.into()),
        RawEntry::Utf8(&[0xed, 0xa0, 0x80]),
        RawEntry::Utf8(b"I"),
        RawEntry::Utf8(b"V"),
    ]);
    let mut pool = resolve(raw).unwrap();
    for index in [2, 3, 7] {
        let expected = if index == 2 {
            ParseErrorKind::UnexpectedConstantPoolEntry {
                index,
                expected: "Utf8",
                actual: "CONSTANT_Long",
            }
        } else {
            ParseErrorKind::InvalidConstantPoolIndex { index }
        };
        let index = pool_index(index);
        assert_eq!(pool.utf8_id(index).unwrap_err().kind(), expected);
        assert_eq!(
            pool.intern_method_descriptor(index).unwrap_err().kind(),
            expected
        );
    }
    let name = pool.utf8_id(pool_index(4)).unwrap();
    let descriptor = pool.intern_method_descriptor(pool_index(1)).unwrap();
    let field_type = pool.intern_field_type(pool_index(5)).unwrap();
    let return_type = pool.intern_return_type(pool_index(6)).unwrap();
    let constant = pool.constant_value_id(pool_index(2)).unwrap();
    for raw_index in [3, 7] {
        let index = pool_index(raw_index);
        for result in [
            pool.intern_field_type(index).map(|_| ()),
            pool.intern_return_type(index).map(|_| ()),
            pool.constant_value_id(index).map(|_| ()),
        ] {
            assert_matches!(
                result.unwrap_err().kind(),
                ParseErrorKind::InvalidConstantPoolIndex { index: actual } if actual == raw_index
            );
        }
    }
    let pool = pool.finish();
    assert_eq!(pool.utf8(name).as_bytes(), &[0xed, 0xa0, 0x80]);
    assert_eq!(pool.method_descriptor(descriptor).descriptor(), "()V");
    assert_eq!(pool.field_type(field_type).descriptor(), "I");
    assert_eq!(pool.return_type(return_type), &ReturnType::Void);
    assert_eq!(pool.constant_value(constant), ConstantVal::Long(0));
}

#[test]
fn class_and_package_names_parse_with_distinct_tags() {
    // Utf8 "a/b/C", Class #1, Package #1.
    let bytes = [1, 0, 5, b'a', b'/', b'b', b'/', b'C', 7, 0, 1, 20, 0, 1];
    let count = 4_u16.try_into().unwrap();
    let pool = resolve(RawConstantPool::parse_prefix(&mut bytes.as_slice(), count).unwrap())
        .unwrap()
        .finish();
    let class_name: ClassName = "a/b/C".parse().unwrap();
    let package_name: PackageName = "a/b/C".parse().unwrap();
    assert_eq!(pool.count(), 4);
    assert_eq!(
        pool.class_name(pool.class_id(pool_index(2)).unwrap()),
        &class_name
    );
    assert_eq!(
        pool.package_name(pool.package_id(pool_index(3)).unwrap())
            .clone(),
        package_name
    );
    assert_matches!(
        pool.class_id(pool_index(3)).unwrap_err().kind(),
        ParseErrorKind::UnexpectedConstantPoolEntry { index: 3, .. }
    );
    assert_matches!(
        pool.package_id(pool_index(2)).unwrap_err().kind(),
        ParseErrorKind::UnexpectedConstantPoolEntry { index: 2, .. }
    );
}

#[test]
fn class_name_accessor_rejects_descriptors() {
    for descriptor in ["[I", "[La/b/C;", "La/b/C;"] {
        let reference_type: ReferenceType = descriptor.parse().unwrap();
        let pool = RawConstantPool::from_entries([
            RawEntry::Utf8(descriptor.as_bytes()),
            RawEntry::Class {
                name: pool_index(1),
            },
        ]);
        let index = 2;

        let pool = resolve(pool).unwrap().finish();
        assert_eq!(
            pool.reference_type(pool.reference_type_id(pool_index(index)).unwrap()),
            &reference_type
        );
        assert_eq!(
            pool.class_id(pool_index(index)).unwrap_err().kind(),
            ParseErrorKind::UnexpectedConstantPoolEntry {
                index: 2,
                expected: "ClassName",
                actual: "CONSTANT_Class"
            },
        );
    }
}

fn method_handle_pool(kind: u8, tag: u8) -> Result<ConstantPool, ParseError> {
    let member = match tag {
        9 => RawEntry::FieldRef {
            class: pool_index(2),
            name_and_type: pool_index(5),
        },
        10 => RawEntry::MethodRef {
            class: pool_index(2),
            name_and_type: pool_index(5),
        },
        11 => RawEntry::InterfaceMethodRef {
            class: pool_index(2),
            name_and_type: pool_index(5),
        },
        _ => unreachable!(),
    };
    resolve(RawConstantPool::from_entries([
        RawEntry::Utf8(b"a/b/I"),
        RawEntry::Class {
            name: pool_index(1),
        },
        RawEntry::Utf8(b"m"),
        RawEntry::Utf8(if tag == 9 { b"I" } else { b"()V" }),
        RawEntry::NameAndType {
            name: pool_index(3),
            descriptor: pool_index(4),
        },
        member,
        RawEntry::MethodHandle {
            reference_kind: kind,
            reference: pool_index(6),
        },
    ]))
    .map(ConstantPoolBuilder::finish)
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
                    let handle = pool.method_handle(pool.method_handle_id(pool_index(7)).unwrap());
                    assert_eq!(handle.reference_kind(), kind);
                    let owned = handle.into_owned();
                    assert_eq!(owned.reference_kind(), kind);
                    assert_eq!(
                        pool.constant_value(pool.constant_value_id(pool_index(7)).unwrap())
                            .into_owned(),
                        ConstantValue::Handle(owned)
                    );
                }
                Err(error) => {
                    assert!(!valid, "kind {kind}, tag {tag}");
                    assert_matches!(
                        error.kind(),
                        ParseErrorKind::Malformed
                            | ParseErrorKind::UnexpectedConstantPoolEntry { .. }
                    );
                }
            }
        }
    }
}

#[test]
fn forward_references_resolve_typed_values_and_keep_indices() {
    let raw = RawConstantPool::from_entries([
        RawEntry::MethodHandle {
            reference_kind: 5,
            reference: pool_index(2),
        },
        RawEntry::MethodRef {
            class: pool_index(3),
            name_and_type: pool_index(4),
        },
        RawEntry::Class {
            name: pool_index(5),
        },
        RawEntry::NameAndType {
            name: pool_index(6),
            descriptor: pool_index(7),
        },
        RawEntry::Utf8(b"a/b/C".as_slice()),
        RawEntry::Utf8(b"m".as_slice()),
        RawEntry::Utf8(b"(I)V".as_slice()),
        RawEntry::Long(42.into()),
        RawEntry::Dynamic {
            bootstrap_method_attr_index: 3.into(),
            name_and_type: pool_index(12),
        },
        RawEntry::InvokeDynamic {
            bootstrap_method_attr_index: 4.into(),
            name_and_type: pool_index(4),
        },
        RawEntry::NameAndType {
            name: pool_index(6),
            descriptor: pool_index(13),
        },
        RawEntry::Utf8(b"I".as_slice()),
    ]);
    let count = raw.count();
    let pool = resolve(raw).unwrap().finish();
    assert_eq!(pool.count(), count);
    assert!(pool.constant_value_id(pool_index(9)).is_err());
    let method = pool
        .method_ref(pool.method_ref_id(pool_index(2)).unwrap())
        .into_owned();
    assert_eq!(method.owner, "a/b/C".parse::<ReferenceType>().unwrap());
    assert_eq!(method.descriptor, "(I)V".parse().unwrap());
    assert_eq!(
        pool.method_handle(pool.method_handle_id(pool_index(1)).unwrap())
            .into_owned(),
        MethodHandle::RefInvokeVirtual(method)
    );
    assert!(matches!(
        pool.constant_value(pool.constant_value_id(pool_index(10)).unwrap()),
        ConstantVal::Dynamic(3, _, _)
    ));
    assert_eq!(
        pool.invoke_dynamic(pool.invoke_dynamic_id(pool_index(11)).unwrap())
            .0,
        4
    );
}

#[test]
fn resolution_rejects_invalid_dependencies_and_descriptors() {
    for entry in [
        RawEntry::Class {
            name: pool_index(3),
        },
        RawEntry::Class {
            name: pool_index(2),
        },
        RawEntry::Class {
            name: pool_index(u16::MAX),
        },
        RawEntry::MethodType {
            descriptor: pool_index(1),
        },
        RawEntry::MethodHandle {
            reference_kind: 0,
            reference: pool_index(1),
        },
    ] {
        let raw = RawConstantPool::from_entries([
            RawEntry::Utf8(b"I".as_slice()),
            RawEntry::Long(42.into()),
            entry,
        ]);
        assert_matches!(
            resolve(raw).unwrap_err().kind(),
            ParseErrorKind::InvalidConstantPoolIndex {
                index: 3 | u16::MAX
            } | ParseErrorKind::UnexpectedConstantPoolEntry { index: 2, .. }
                | ParseErrorKind::Malformed
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
            string: pool_index(1),
        },
    ]);
    let pool = resolve(raw).unwrap().finish();
    assert_eq!(pool.get_utf8(pool_index(1)).unwrap().as_bytes(), bytes);
    let constant = pool.constant_value(pool.constant_value_id(pool_index(2)).unwrap());
    let ConstantVal::String(value) = constant else {
        panic!("expected string constant");
    };
    assert_eq!(value, pool.utf8(pool.utf8_id(pool_index(1)).unwrap()));
    assert_eq!(constant.into_owned(), ConstantValue::String(expected));
}

#[test]
fn malformed_utf8_entries_are_rejected_without_references() {
    let raw = RawConstantPool::from_entries([RawEntry::Utf8(b"\xff")]);
    assert_eq!(resolve(raw).unwrap_err().kind(), ParseErrorKind::Malformed);
}
