use std::iter;

use proptest::prelude::*;

use super::*;
use crate::jvm::bytecode::ParseErrorKind;

fn assert_malformed(bytes: Vec<u8>) {
    let error = InstructionList::<RawInstruction>::from_bytes(bytes).unwrap_err();
    assert_eq!(error.kind(), ParseErrorKind::Malformed);
}

fn valid_encoding(opcode: u8) -> Option<Vec<u8>> {
    let operands: &[u8] = match opcode {
        0x00..=0x0f
        | 0x1a..=0x35
        | 0x3b..=0x83
        | 0x85..=0x98
        | 0xac..=0xb1
        | 0xbe..=0xbf
        | 0xc2..=0xc3
        | 0xca
        | 0xfe..=0xff => &[],
        0x10 | 0x12 | 0x15..=0x19 | 0x36..=0x3a | 0xa9 | 0xbc => &[0],
        0x11
        | 0x13..=0x14
        | 0x84
        | 0x99..=0xa8
        | 0xb2..=0xb8
        | 0xbb
        | 0xbd
        | 0xc0..=0xc1
        | 0xc6..=0xc7 => &[0, 0],
        0xaa => &[0; 19],
        0xab => &[0; 11],
        0xb9..=0xba | 0xc8..=0xc9 => &[0, 0, 0, 0],
        0xc4 => &[0x15, 0, 0],
        0xc5 => &[0, 0, 1],
        _ => return None,
    };
    Some(iter::once(opcode).chain(operands.iter().copied()).collect())
}

#[test]
fn decodes_every_defined_opcode_and_rejects_undefined_opcodes() {
    for opcode in 0..=u8::MAX {
        let Some(bytes) = valid_encoding(opcode) else {
            assert_malformed(vec![opcode]);
            continue;
        };
        let instructions = InstructionList::<RawInstruction>::from_bytes(bytes)
            .unwrap_or_else(|error| panic!("opcode 0x{opcode:02x} failed to parse: {error}"));
        let decoded: Vec<_> = instructions.iter().collect();
        assert_eq!(decoded.len(), 1, "opcode 0x{opcode:02x}");
        assert_eq!(*decoded[0].0, ProgramCounter::default());
        assert_eq!(decoded[0].1.opcode(), opcode);
    }
}

#[test]
fn operand_bearing_opcodes_reject_truncated_encodings() {
    for opcode in 0..=u8::MAX {
        let Some(mut bytes) = valid_encoding(opcode) else {
            continue;
        };
        if bytes.len() == 1 {
            continue;
        }
        bytes.pop();
        let error = InstructionList::<RawInstruction>::from_bytes(bytes).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::IO, "opcode 0x{opcode:02x}");
    }
}

proptest! {
    #[test]
    fn invokedynamic_rejects_nonzero_reserved_bytes(
        dynamic_index in any::<u16>(),
        reserved in 1u16..=u16::MAX,
    ) {
        let mut bytes = vec![0xba];
        bytes.extend(dynamic_index.to_be_bytes());
        bytes.extend(reserved.to_be_bytes());
        assert_malformed(bytes);
    }

    #[test]
    fn switches_reject_nonzero_padding(
        opcode in prop_oneof![Just(0xaa_u8), Just(0xab_u8)],
        prefix_len in 0usize..3,
        padding_slot in any::<usize>(),
        nonzero in 1u8..=u8::MAX,
    ) {
        let mut bytes = vec![0; prefix_len];
        bytes.push(opcode);
        let padding_len = (4 - bytes.len() % 4) % 4;
        let padding_start = bytes.len();
        bytes.resize(padding_start + padding_len, 0);
        bytes[padding_start + padding_slot % padding_len] = nonzero;
        bytes.extend(if opcode == 0xaa {
            &[0; 16][..]
        } else {
            &[0; 8][..]
        });
        assert_malformed(bytes);
    }

    #[test]
    fn lookupswitch_rejects_negative_pair_count(
        default in any::<i32>(),
        pair_count in i32::MIN..0,
    ) {
        let mut bytes = vec![0xab, 0, 0, 0];
        bytes.extend(default.to_be_bytes());
        bytes.extend(pair_count.to_be_bytes());
        assert_malformed(bytes);
    }

    #[test]
    fn lookupswitch_rejects_keys_that_are_not_strictly_increasing(
        default in any::<i32>(),
        first_key in any::<i32>(),
        second_key in any::<i32>(),
        first_offset in any::<i32>(),
        second_offset in any::<i32>(),
    ) {
        prop_assume!(second_key <= first_key);
        let mut bytes = vec![0xab, 0, 0, 0];
        bytes.extend(default.to_be_bytes());
        bytes.extend(2_i32.to_be_bytes());
        bytes.extend(first_key.to_be_bytes());
        bytes.extend(first_offset.to_be_bytes());
        bytes.extend(second_key.to_be_bytes());
        bytes.extend(second_offset.to_be_bytes());
        assert_malformed(bytes);
    }

    #[test]
    fn tableswitch_rejects_inverted_range(
        default in any::<i32>(),
        low in any::<i32>(),
        high in any::<i32>(),
    ) {
        prop_assume!(low > high);
        let mut bytes = vec![0xaa, 0, 0, 0];
        bytes.extend(default.to_be_bytes());
        bytes.extend(low.to_be_bytes());
        bytes.extend(high.to_be_bytes());
        assert_malformed(bytes);
    }
}

#[test]
fn opcode_matches_encoding() {
    use RawInstruction::*;

    assert_eq!(Nop.opcode(), 0x00);
    assert_eq!(AConstNull.opcode(), 0x01);
    assert_eq!(IConstM1.opcode(), 0x02);
    assert_eq!(ILoad { index: 233 }.opcode(), 0x15);
}
