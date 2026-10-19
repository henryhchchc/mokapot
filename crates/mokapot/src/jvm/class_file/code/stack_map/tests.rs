use super::raw;
use crate::jvm::class_file::Decode;

#[test]
fn raw_frames_preserve_extended_encoding_and_constant_pool_indices() {
    let bytes = [247, 0, 9, 7, 0x12, 0x34];
    let frame = raw::StackMapFrameInfo::decode(&mut bytes.as_slice()).unwrap();
    assert!(matches!(
        frame,
        raw::StackMapFrameInfo::SameLocals1StackItemFrameExtended {
            offset_delta: 9,
            stack: raw::VerificationTypeInfo::Object {
                cpool_index: 0x1234
            },
        }
    ));
}

#[test]
fn object_verification_types_validate_indices_and_accept_arrays() {
    use super::*;
    use crate::jvm::{
        class::Version,
        class_file::constant_pool::{ConstantPool, RawConstantPool, RawEntry},
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
        constant_pool: ConstantPool::from_raw(pool).unwrap(),
        class_version: Version::LATEST,
    };
    for cpool_index in [0, 1, 3, 4, 5] {
        let raw = raw::VerificationTypeInfo::Object { cpool_index };
        assert_eq!(
            raw.resolve(&mut ctx).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
    }
    let raw = raw::StackMapFrameInfo::FullFrame {
        offset_delta: 7,
        locals: Box::new([raw::VerificationTypeInfo::Object { cpool_index: 2 }]),
        stack: Box::new([raw::VerificationTypeInfo::Uninitialized { offset: 3 }]),
    };
    let StackMapFrameData::Full {
        offset_delta,
        locals,
        stack,
    } = raw.resolve(&mut ctx).unwrap()
    else {
        panic!("expected a full frame")
    };
    assert_eq!(offset_delta, 7);
    assert!(
        matches!(stack[0], VerificationTypeData::Uninitialized { offset } if offset == 3.into())
    );
    let VerificationTypeData::Object(id) = locals[0] else {
        panic!("expected an object type")
    };
    let pool = ctx.constant_pool.finish();
    assert_eq!(pool.reference_type(id).descriptor(), "[I");
}
