#[test]
fn object_verification_types_validate_indices_and_resolve_arrays() {
    use super::*;
    use crate::ir::test::prelude::assert_matches;
    use crate::jvm::{
        class::Version,
        class_file::constant_pool::{RawConstantPool, RawEntry, resolve},
        errors::ParseErrorKind,
    };
    use crate::types::Descriptor;

    let pool = RawConstantPool::from_entries([
        RawEntry::Utf8(b"[I"),
        RawEntry::Class {
            name_index: 1.into(),
        },
        RawEntry::Long(0.into()),
    ]);
    let mut ctx = ResolutionContext {
        constant_pool: resolve(pool).unwrap(),
        class_version: Version::LATEST,
    };
    for cpool_index in [0, 1, 3, 4, 5] {
        let raw = raw::VerificationTypeInfo::Object { cpool_index };
        let kind = raw.resolve(&mut ctx).unwrap_err().kind();
        if matches!(cpool_index, 1 | 3) {
            assert_matches!(
                kind,
                ParseErrorKind::UnexpectedConstantPoolEntry { index, .. } if index == cpool_index
            );
        } else {
            assert_matches!(
                kind,
                ParseErrorKind::InvalidConstantPoolIndex { index } if index == cpool_index
            );
        }
    }
    let VerificationTypeData::Object(id) = raw::VerificationTypeInfo::Object { cpool_index: 2 }
        .resolve(&mut ctx)
        .unwrap()
    else {
        panic!("expected an object verification type")
    };
    let pool = ctx.constant_pool.finish();
    assert_eq!(pool.reference_type(id).descriptor(), "[I");
}
