use std::iter;

use proptest::prelude::*;

use super::*;
use crate::jvm::errors::ParseErrorKind;

fn assert_malformed(bytes: &[u8]) {
    assert_malformed_at(bytes, 0);
}

fn assert_malformed_at(bytes: &[u8], prefix_len: usize) {
    let mut reader = bytes;
    for _ in 0..prefix_len {
        RawInstruction::read_one(&mut reader, bytes.len())
            .expect("prefix Nop should parse")
            .expect("prefix Nop should be present");
    }
    let error = RawInstruction::read_one(&mut reader, bytes.len()).unwrap_err();
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
            assert_malformed(&[opcode]);
            continue;
        };
        let mut reader = bytes.as_slice();
        let decoded = RawInstruction::read_one(&mut reader, bytes.len())
            .unwrap_or_else(|error| panic!("opcode 0x{opcode:02x} failed to parse: {error}"))
            .unwrap();
        assert_eq!(
            bytes.len() - reader.len(),
            bytes.len(),
            "opcode 0x{opcode:02x}"
        );
        assert_eq!(decoded.0, ProgramCounter::default());
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
        let error = RawInstruction::read_one(&mut bytes.as_slice(), bytes.len()).unwrap_err();
        assert_eq!(
            error.kind(),
            ParseErrorKind::Malformed,
            "opcode 0x{opcode:02x}"
        );
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
        assert_malformed(&bytes);
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
        assert_malformed_at(&bytes, prefix_len);
    }

    #[test]
    fn lookupswitch_rejects_negative_pair_count(
        default in any::<i32>(),
        pair_count in i32::MIN..0,
    ) {
        let mut bytes = vec![0xab, 0, 0, 0];
        bytes.extend(default.to_be_bytes());
        bytes.extend(pair_count.to_be_bytes());
        assert_malformed(&bytes);
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
        assert_malformed(&bytes);
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
        assert_malformed(&bytes);
    }
}

#[test]
fn parses_instruction_operands_and_switch_alignment() {
    use RawInstruction::*;

    let bytes = [
        0x03, // iconst_0
        0x10, 0xfe, // bipush
        0x11, 0xfe, 0xdc, // sipush
        0x12, 0xfe, // ldc
        0x13, 0xfe, 0xdc, // ldc_w
        0x15, 0xfe, // iload
        0x84, 0xfe, 0xfe, // iinc
        0xc4, 0x84, 0xfe, 0xdc, 0xfb, 0x2e, // wide iinc
        0xb9, 0xfe, 0xdc, 7, 0,    // invokeinterface
        0xaa, // tableswitch at pc 27, no padding
        0xff, 0xff, 0xff, 0xf4, // default -12
        0xff, 0xff, 0xff, 0xff, // low -1
        0, 0, 0, 1, // high 1
        0, 0, 0, 10, 0xff, 0xff, 0xff, 0xec, 0, 0, 0, 30, // offsets
        0xab, 0, 0, 0, // lookupswitch at pc 52, three padding bytes
        0, 0, 0, 40, // default
        0, 0, 0, 3, // pair count
        0xff, 0xff, 0xff, 0xfd, 0xff, 0xff, 0xff, 0xe2, // -3: -30
        0, 0, 0, 2, 0, 0, 0, 20, // 2: 20
        0, 0, 0, 9, 0, 0, 0, 90, // 9: 90
        0xc8, 0xff, 0xfe, 0x1d, 0xc0, // goto_w
    ];
    let decoded = collect_instructions(&bytes).unwrap();

    assert_eq!(
        decoded.into_iter().collect::<Vec<_>>(),
        vec![
            (0.into(), IConst0),
            (1.into(), BiPush { value: -2 }),
            (3.into(), SiPush { value: -292 }),
            (6.into(), Ldc { const_index: 0xfe }),
            (
                8.into(),
                LdcW {
                    const_index: 0xfedc
                }
            ),
            (11.into(), ILoad { index: 0xfe }),
            (
                13.into(),
                IInc {
                    index: 0xfe,
                    constant: -2
                }
            ),
            (
                16.into(),
                Wide(RawWideInstruction::IInc {
                    index: 0xfedc,
                    increment: -1234
                })
            ),
            (
                22.into(),
                InvokeInterface {
                    method_index: 0xfedc,
                    count: 7
                }
            ),
            (
                27.into(),
                TableSwitch {
                    default: -12,
                    low: -1,
                    high: 1,
                    jump_offsets: &[10.into(), (-20).into(), 30.into()],
                },
            ),
            (
                52.into(),
                LookupSwitch {
                    default: 40,
                    match_offsets: &[
                        LookupSwitchPair {
                            match_value: (-3).into(),
                            offset: (-30).into()
                        },
                        LookupSwitchPair {
                            match_value: 2.into(),
                            offset: 20.into()
                        },
                        LookupSwitchPair {
                            match_value: 9.into(),
                            offset: 90.into()
                        },
                    ],
                },
            ),
            (88.into(), GotoW { offset: -123_456 }),
        ]
    );
}

#[test]
fn switch_tables_borrow_unaligned_input_at_every_instruction_alignment() {
    for prefix_len in 0..4 {
        for opcode in [0xaa, 0xab] {
            let mut bytes = vec![0xff];
            bytes.resize(1 + prefix_len, 0);
            bytes.push(opcode);
            while !(bytes.len() - 1).is_multiple_of(4) {
                bytes.push(0);
            }
            let header: &[i32] = if opcode == 0xaa { &[0, 0, 1] } else { &[0, 2] };
            bytes.extend(header.iter().flat_map(|value| value.to_be_bytes()));
            let table_start = bytes.len();
            let values: &[i32] = if opcode == 0xaa {
                &[0, 0]
            } else {
                &[-1, 0, 1, 0]
            };
            bytes.extend(values.iter().flat_map(|value| value.to_be_bytes()));

            let instructions = collect_instructions(&bytes[1..]).unwrap();
            let pc = ProgramCounter::from(u16::try_from(prefix_len).unwrap());
            let pointer = match instructions.get(&pc).unwrap() {
                RawInstruction::TableSwitch { jump_offsets, .. } => {
                    assert_eq!(jump_offsets.len(), 2);
                    jump_offsets.as_ptr().cast::<u8>()
                }
                RawInstruction::LookupSwitch { match_offsets, .. } => {
                    assert_eq!(match_offsets.len(), 2);
                    assert_eq!(match_offsets[0].match_value.get(), -1);
                    match_offsets.as_ptr().cast::<u8>()
                }
                _ => panic!("expected a switch"),
            };
            assert!(std::ptr::eq(pointer, bytes[table_start..].as_ptr()));
        }
    }
}

#[test]
fn switches_reject_unavailable_large_tables() {
    for (opcode, header) in [
        (0xaa, vec![0_i32, i32::MIN, i32::MAX]),
        (0xab, vec![0_i32, i32::MAX]),
    ] {
        let mut bytes = vec![opcode, 0, 0, 0];
        bytes.extend(header.into_iter().flat_map(i32::to_be_bytes));
        assert_malformed(&bytes);
    }
}

fn collect_instructions(
    bytes: &[u8],
) -> Result<crate::jvm::code::InstructionList<RawInstruction<'_>>, ParseError> {
    RawInstruction::iter_from_bytes(bytes).collect()
}
