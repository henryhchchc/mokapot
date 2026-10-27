use super::*;
use crate::ir::test::prelude::assert_matches;
use crate::jvm::class_file::constant_pool::{
    ConstantPoolBuilder, RawConstantPool, RawEntry, resolve,
};
use crate::jvm::{Class, code::Instruction, references::MethodReference};

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
    let actual = InstructionData::from_raw_instruction(raw, 10.into(), &mut empty_pool()).unwrap();
    assert_eq!(
        actual.view(&Class::default().constant_pool),
        Instruction::TableSwitch {
            default: 5.into(),
            low: -1,
            jump_targets: &[10.into(), 11.into()],
        }
    );
}

fn empty_pool() -> ConstantPoolBuilder {
    resolve(RawConstantPool::from_entries([])).unwrap()
}

fn interface_method_pool(is_interface: bool) -> ConstantPoolBuilder {
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
    resolve(raw).unwrap()
}

#[test]
fn invokeinterface_parses_interface_method_refs() {
    let (pc, instruction) = RawInstruction::iter_from_bytes(&[0xb9, 0, 6, 1, 0])
        .next()
        .unwrap()
        .unwrap();
    let mut pool = interface_method_pool(true);
    let data = InstructionData::from_raw_instruction(instruction, pc, &mut pool).unwrap();
    let name = pool.class_id(2).unwrap();
    let class = Class {
        name,
        constant_pool: pool.finish(),
        ..Class::default()
    };
    let Instruction::InvokeInterface(method, count) = data.view(&class.constant_pool) else {
        panic!("expected invokeinterface")
    };
    assert_eq!(count, 1);
    assert_eq!(
        method.into_owned(),
        MethodReference {
            owner: "a/b/I".parse().unwrap(),
            name: "m".to_owned().into(),
            descriptor: "()V".parse().unwrap(),
        }
    );
}

#[test]
fn invocation_opcodes_preserve_pool_kind_validation() {
    for interface in [false, true] {
        let mut pool = interface_method_pool(interface);
        for opcode in [0xb6, 0xb7, 0xb8, 0xb9] {
            let bytes = [opcode, 0, 6, 1, 0];
            let (pc, raw) = RawInstruction::iter_from_bytes(&bytes)
                .next()
                .unwrap()
                .unwrap();
            let result = InstructionData::from_raw_instruction(raw, pc, &mut pool);
            let valid = match opcode {
                0xb6 => !interface,
                0xb9 => interface,
                _ => true,
            };
            if valid {
                assert!(result.is_ok());
            } else {
                assert_matches!(
                    result.unwrap_err().kind(),
                    crate::jvm::errors::ParseErrorKind::UnexpectedConstantPoolEntry {
                        index: 6,
                        ..
                    }
                );
            }
        }
    }
}
