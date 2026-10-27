use crate::ir::test::prelude::assert_matches;
use proptest::prelude::*;

use super::*;
use crate::jvm::errors::ParseErrorKind;

#[test]
fn invalid_opcodes_report_instruction_offset() {
    for (bytes, opcode) in [(&[0, 0xcb][..], 0xcb), (&[0, 0xc4, 0xff][..], 0xff)] {
        let error = RawInstruction::iter_from_bytes(bytes)
            .nth(1)
            .unwrap()
            .unwrap_err();
        assert_eq!(
            error.kind(),
            ParseErrorKind::InvalidOpcode { opcode, pc: 1 }
        );
    }
}

fn assert_malformed(bytes: &[u8]) {
    let mut reader = bytes;
    let error = RawInstruction::read_one(&mut reader, bytes.len()).unwrap_err();
    assert_matches!(
        error.kind(),
        ParseErrorKind::Malformed | ParseErrorKind::Truncated { .. }
    );
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
    let decoded = collect_instructions(&bytes)
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>();

    assert_eq!(
        &decoded[..9],
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
        ]
    );
    let (table_pc, TableSwitch(table)) = decoded[9] else {
        panic!("expected tableswitch");
    };
    assert_eq!(table_pc, 27.into());
    assert_eq!(table.default.get(), -12);
    assert_eq!(table.low.get(), -1);
    assert_eq!(table.high.get(), 1);
    assert_eq!(
        &table.jump_offsets,
        [I32::new(10), I32::new(-20), I32::new(30)]
    );

    let (lookup_pc, LookupSwitch(lookup)) = decoded[10] else {
        panic!("expected lookupswitch");
    };
    assert_eq!(lookup_pc, 52.into());
    assert_eq!(lookup.default.get(), 40);
    let pairs: Vec<_> = lookup
        .match_offsets
        .iter()
        .map(|pair| (pair.match_value.get(), pair.offset.get()))
        .collect();
    assert_eq!(pairs, [(-3, -30), (2, 20), (9, 90)]);
    assert_eq!(&decoded[11..], [(88.into(), GotoW { offset: -123_456 })]);
}

#[test]
fn switches_decode_at_every_instruction_alignment() {
    for prefix_len in 0..4 {
        for opcode in [0xaa, 0xab] {
            let mut bytes = vec![0; prefix_len];
            bytes.push(opcode);
            while !bytes.len().is_multiple_of(4) {
                bytes.push(0xff);
            }
            let header: &[i32] = if opcode == 0xaa { &[0, 0, 1] } else { &[0, 2] };
            bytes.extend(header.iter().flat_map(|value| value.to_be_bytes()));
            let values: &[i32] = if opcode == 0xaa {
                &[0, 0]
            } else {
                &[-1, 0, 1, 0]
            };
            bytes.extend(values.iter().flat_map(|value| value.to_be_bytes()));

            let instructions = collect_instructions(&bytes).unwrap();
            let pc = ProgramCounter::from(u16::try_from(prefix_len).unwrap());
            let instruction = instructions
                .iter()
                .find_map(|(at, instruction)| (*at == pc).then_some(instruction))
                .unwrap();
            match instruction {
                RawInstruction::TableSwitch(info) => {
                    assert_eq!(info.jump_offsets.len(), 2);
                }
                RawInstruction::LookupSwitch(info) => {
                    assert_eq!(info.match_offsets.len(), 2);
                    assert_eq!(info.match_offsets[0].match_value.get(), -1);
                }
                _ => panic!("expected a switch"),
            }
        }
    }
}

#[test]
fn switches_reject_truncated_padding() {
    for opcode in [0xaa, 0xab] {
        for padding_len in 0..3 {
            let mut bytes = vec![opcode];
            bytes.extend(vec![0xff; padding_len]);
            assert_malformed(&bytes);
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
) -> Result<Vec<(ProgramCounter, RawInstruction<'_>)>, ParseError> {
    RawInstruction::iter_from_bytes(bytes).collect()
}
