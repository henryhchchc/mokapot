use super::*;
use crate::jvm::{
    JavaString,
    references::{FieldRefData, MethodRef},
};
use crate::types::Descriptor;

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

    let [
        (pool, field, method),
        (other, other_field, other_method),
        (different, different_field, different_method),
    ] = [("field", "()V"), ("field", "()V"), ("other", "()I")].map(|(name, descriptor)| {
        let mut pool = ConstantPool::new();
        let owner = pool.insert_reference_type("pkg/Owner".parse().unwrap());
        let name = pool.insert_utf8(name.into());
        let ty = pool.insert_field_type("I".parse().unwrap());
        let index = pool
            .inner
            .push(Entry::NameAndType(NameAndType::Field(name, ty)));
        let name_and_type = FieldNameAndTypeId(NonZeroU16::new(index).unwrap());
        let index = pool.inner.push(Entry::FieldRef(FieldRefData {
            owner,
            name_and_type,
        }));
        let field = FieldRefId(NonZeroU16::new(index).unwrap());
        let method = pool.insert_method_ref(MethodRef {
            owner: "pkg/Owner".parse().unwrap(),
            name: "method".into(),
            descriptor: descriptor.parse().unwrap(),
        });
        (pool, field, method)
    });
    let field = pool.field_ref(field);
    let other_field = other.field_ref(other_field);
    assert_eq!(field, other_field);
    assert_eq!(hash(field), hash(other_field));

    let method = pool.method_ref(method);
    let other_method = other.method_ref(other_method);
    assert_eq!(method, other_method);
    assert_eq!(hash(method), hash(other_method));

    assert_ne!(field, different.field_ref(different_field));
    assert_ne!(method, different.method_ref(different_method));
}

#[test]
fn miri_utf8_ranges_preserve_boundaries() {
    let values = [
        b"".as_slice(),
        b"a\xc0\x80\xed\xa0\xbd\xed\xb8\x80\xed\xa0\x80",
        b"",
        b"tail",
        b"",
    ];
    let mut pool = ConstantPool::new();
    let ids: Vec<_> = values
        .iter()
        .map(|bytes| pool.insert_utf8(JavaString::from_cesu8(bytes.to_vec()).unwrap()))
        .collect();
    assert!(
        pool.insert_resolved_at(ids[0].0.get(), Entry::Utf8(1..2))
            .is_err()
    );
    let mut start = 0;
    for (id, bytes) in ids.into_iter().zip(values) {
        let end = start + bytes.len();
        let Entry::Utf8(range) = pool.get_entry(id.0.get()).unwrap() else {
            panic!()
        };
        assert_eq!(range, &(start..end));
        let value = pool.utf8(id);
        assert_eq!(value.as_bytes(), bytes);
        assert_eq!(
            value.as_bytes().as_ptr(),
            pool.utf8_bytes[start..end].as_ptr()
        );
        start = end;
    }
    assert_eq!(pool.utf8_bytes.len(), start);
}

#[test]
fn miri_utf8_ranges_survive_growth_and_clone() {
    let bytes = b"\xc0\x80\xed\xa0\x80";
    let mut pool = ConstantPool::new();
    let first = pool.insert_utf8(JavaString::from_cesu8(bytes.to_vec()).unwrap());
    let second = pool.insert_utf8(JavaString::new());
    let original = pool.clone();
    let capacity = pool.utf8_bytes.capacity();
    let value = JavaString::from("x".repeat(capacity + 1));
    let inserted = pool.insert_utf8(value.clone());
    assert!(pool.utf8_bytes.capacity() > capacity);
    let mut cloned = pool.clone();
    cloned.insert_utf8(JavaString::from("another"));
    drop(pool);
    for pool in [&original, &cloned] {
        assert_eq!(pool.utf8(first).as_bytes(), bytes);
        assert_eq!(pool.utf8(second).as_bytes(), []);
    }
    assert_eq!(cloned.utf8(inserted), &*value);
}

#[test]
fn miri_owned_method_ref_preserves_pool_name_after_drop() {
    let name = [0xed, 0xa0, 0x80];
    let mut pool = ConstantPool::new();
    let id = pool.insert_method_ref(MethodRef {
        owner: "pkg/Owner".parse().unwrap(),
        name: JavaString::from_cesu8(name.to_vec()).unwrap(),
        descriptor: "()V".parse().unwrap(),
    });
    let reference = pool.method_ref(id).into_owned();
    drop(pool);
    assert_eq!(reference.name.as_bytes(), name);
    assert_eq!(reference.descriptor.descriptor(), "()V");
}
