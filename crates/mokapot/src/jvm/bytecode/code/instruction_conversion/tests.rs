use super::*;
use crate::jvm::bytecode::constant_pool::{RawConstantPool, RawEntry};

#[test]
fn tableswitch_default_can_target_backward() {
    let raw = RawInstruction::TableSwitch {
        default: -5,
        low: 0,
        high: 0,
        jump_offsets: &[0.into()],
    };
    let actual = Instruction::from_raw_instruction(raw, 10.into(), &ConstantPool::new()).unwrap();
    assert_eq!(
        actual,
        Instruction::TableSwitch {
            default: 5.into(),
            low: 0,
            jump_targets: vec![10.into()],
        }
    );
}

#[test]
fn tableswitch_uses_inclusive_high_bound() {
    let raw = RawInstruction::TableSwitch {
        default: 2,
        low: -1,
        high: 0,
        jump_offsets: &[0.into(), 1.into()],
    };
    let actual = Instruction::from_raw_instruction(raw, 10.into(), &ConstantPool::new()).unwrap();
    assert_eq!(
        actual,
        Instruction::TableSwitch {
            default: 12.into(),
            low: -1,
            jump_targets: vec![10.into(), 11.into()],
        }
    );
}

#[test]
fn tableswitch_rejects_mismatched_raw_targets() {
    let raw = RawInstruction::TableSwitch {
        default: 0,
        low: 1,
        high: 2,
        jump_offsets: &[0.into()],
    };
    let error = Instruction::from_raw_instruction(raw, 0.into(), &ConstantPool::new()).unwrap_err();
    assert_eq!(error.kind(), crate::jvm::errors::ParseErrorKind::Malformed);
}

fn interface_method_pool(is_interface: bool) -> ConstantPool {
    let mut raw = RawConstantPool::new();
    let class_name_index = raw.put_entry(RawEntry::Utf8(b"a/b/I".as_slice())).unwrap();
    let class_index = raw
        .put_entry(RawEntry::Class {
            name_index: class_name_index.into(),
        })
        .unwrap();
    let name_index = raw.put_entry(RawEntry::Utf8(b"m".as_slice())).unwrap();
    let descriptor_index = raw.put_entry(RawEntry::Utf8(b"()V".as_slice())).unwrap();
    let name_and_type_index = raw
        .put_entry(RawEntry::NameAndType {
            name_index: name_index.into(),
            descriptor_index: descriptor_index.into(),
        })
        .unwrap();
    let entry = if is_interface {
        RawEntry::InterfaceMethodRef {
            class_index: class_index.into(),
            name_and_type_index: name_and_type_index.into(),
        }
    } else {
        RawEntry::MethodRef {
            class_index: class_index.into(),
            name_and_type_index: name_and_type_index.into(),
        }
    };
    raw.put_entry(entry).unwrap();
    ConstantPool::from_raw(raw).unwrap()
}

#[test]
fn invokeinterface_parses_interface_method_refs() {
    let raw = InstructionList::<RawInstruction>::from_bytes(&[0xb9, 0, 6, 1, 0]).unwrap();
    let (pc, instruction) = raw.iter().next().unwrap();
    assert_eq!(
        Instruction::from_raw_instruction(*instruction, pc, &interface_method_pool(true)).unwrap(),
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
    let raw = InstructionList::<RawInstruction>::from_bytes(&[0xb9, 0, 6, 1, 0]).unwrap();
    let (pc, instruction) = raw.iter().next().unwrap();
    let pool = interface_method_pool(false);
    let error = Instruction::from_raw_instruction(*instruction, pc, &pool).unwrap_err();
    assert_eq!(error.kind(), crate::jvm::errors::ParseErrorKind::Malformed);
}
