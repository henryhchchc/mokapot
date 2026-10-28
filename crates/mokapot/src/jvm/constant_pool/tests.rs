use super::*;
use crate::jvm::{JavaString, references::MethodReference};
use crate::types::Descriptor;

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
    for (id, bytes) in ids.into_iter().zip(values) {
        assert_eq!(pool.utf8(id).as_bytes(), bytes);
    }
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
    cloned.insert_utf8("another".to_owned().into());
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
    let id = pool.insert_method_ref(MethodReference {
        owner: "pkg/Owner".parse().unwrap(),
        name: JavaString::from_cesu8(name.to_vec()).unwrap(),
        descriptor: "()V".parse().unwrap(),
    });
    let reference = pool.method_ref(id).into_owned();
    drop(pool);
    assert_eq!(reference.name.as_bytes(), name);
    assert_eq!(reference.descriptor.descriptor(), "()V");
}
