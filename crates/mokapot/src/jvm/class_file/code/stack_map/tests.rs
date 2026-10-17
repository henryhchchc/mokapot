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
