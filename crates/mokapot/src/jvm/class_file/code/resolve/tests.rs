use super::*;
use crate::jvm::class_file::constant_pool::{RawConstantPool, RawEntry};

#[test]
fn tableswitch_resolves_backward_default_and_inclusive_range() {
    let bytes = [
        0xaa, 0, 0, 0, 0xff, 0xff, 0xff, 0xfb, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 1,
    ];
    let (_, raw) = RawInstruction::iter_from_bytes(&bytes)
        .next()
        .unwrap()
        .unwrap();
    let actual = Instruction::from_raw_instruction(raw, 10.into(), &ConstantPool::new()).unwrap();
    assert_eq!(
        actual,
        Instruction::TableSwitch {
            default: 5.into(),
            low: -1,
            jump_targets: Box::new([10.into(), 11.into()]),
        }
    );
}

fn interface_method_pool(is_interface: bool) -> ConstantPool {
    let entry = if is_interface {
        RawEntry::InterfaceMethodRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        }
    } else {
        RawEntry::MethodRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        }
    };
    let raw = RawConstantPool::from_entries([
        RawEntry::Utf8(b"a/b/I"),
        RawEntry::Class {
            name_index: 1.into(),
        },
        RawEntry::Utf8(b"m"),
        RawEntry::Utf8(b"()V"),
        RawEntry::NameAndType {
            name_index: 3.into(),
            descriptor_index: 4.into(),
        },
        entry,
    ]);
    ConstantPool::from_raw(raw).unwrap()
}

#[test]
fn invokeinterface_parses_interface_method_refs() {
    let (pc, instruction) = RawInstruction::iter_from_bytes(&[0xb9, 0, 6, 1, 0])
        .next()
        .unwrap()
        .unwrap();
    assert_eq!(
        Instruction::from_raw_instruction(instruction, pc, &interface_method_pool(true)).unwrap(),
        Instruction::InvokeInterface(
            crate::jvm::references::MethodRef {
                owner: crate::types::reference_type::ReferenceType::Class("a/b/I".parse().unwrap()),
                name: "m".to_owned(),
                descriptor: "()V".parse().unwrap(),
            },
            1,
        ),
    );
}

#[test]
fn invokeinterface_rejects_method_refs() {
    let (pc, instruction) = RawInstruction::iter_from_bytes(&[0xb9, 0, 6, 1, 0])
        .next()
        .unwrap()
        .unwrap();
    let pool = interface_method_pool(false);
    let error = Instruction::from_raw_instruction(instruction, pc, &pool).unwrap_err();
    assert_eq!(error.kind(), crate::jvm::errors::ParseErrorKind::Malformed);
}

#[test]
fn miri_push_immediates_preserve_signed_values_through_ir() {
    use crate::ir::test::prelude::*;
    use crate::jvm::ConstantValue;

    let cases = [
        (vec![0x10, 0x80, 0xac], -128),
        (vec![0x10, 0x7f, 0xac], 127),
        (vec![0x11, 0x80, 0x00, 0xac], -32768),
        (vec![0x11, 0x7f, 0xff, 0xac], 32767),
    ];
    for (bytes, expected) in cases {
        let pool = ConstantPool::new();
        let instructions = RawInstruction::iter_from_bytes(&bytes)
            .map(|item| {
                let (pc, raw) = item?;
                Instruction::from_raw_instruction(raw, pc, &pool)
                    .map(|instruction| (pc, instruction))
            })
            .collect::<Result<Vec<_>, ParseError>>()
            .unwrap();
        let method = method(instructions, "()I", vec![]);
        let ir = MokaIRMethod::from_method(&method).unwrap();
        let definitions = operations(&ir).collect::<Vec<_>>();
        assert_matches!(definitions.as_slice(), [Operation::Definition { value, expr: Expression::Const(ConstantValue::Integer(actual)) }]
            if *actual == expected && matches!(&ir.block(ir.entry.block).unwrap().terminator, Terminator::Return { value: Some(returned), .. } if returned == value));
    }
}
