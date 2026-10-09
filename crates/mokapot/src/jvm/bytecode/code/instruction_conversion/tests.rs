use super::*;
use crate::{
    jvm::{bytecode::constant_pool::RawEntry, references::MethodRef},
    types::reference_type::ReferenceType,
};

fn interface_method() -> MethodRef {
    MethodRef {
        owner: ReferenceType::Class("a/b/I".parse().unwrap()),
        name: "m".to_owned(),
        descriptor: "()V".parse().unwrap(),
    }
}

#[test]
fn tableswitch_default_can_target_backward() {
    let raw = Instruction::TableSwitch {
        default: ProgramCounter::from(5),
        low: 0,
        jump_targets: vec![ProgramCounter::from(10)],
    }
    .into_raw_instruction(ProgramCounter::from(10), &mut RawConstantPool::new())
    .unwrap();

    assert_eq!(
        raw,
        RawInstruction::TableSwitch {
            default: -5,
            low: 0,
            high: 0,
            jump_offsets: vec![0],
        }
    );
}

#[test]
fn tableswitch_uses_inclusive_high_bound() {
    let targets = vec![10.into(), 11.into()];
    let instruction = Instruction::TableSwitch {
        default: 12.into(),
        low: -1,
        jump_targets: targets.clone(),
    };
    let raw = instruction
        .into_raw_instruction(10.into(), &mut RawConstantPool::new())
        .unwrap();

    let raw_instruction = RawInstruction::TableSwitch {
        default: 2,
        low: -1,
        high: 0,
        jump_offsets: vec![0, 1],
    };
    assert_eq!(raw, raw_instruction);
    let expected = Instruction::TableSwitch {
        default: 12.into(),
        low: -1,
        jump_targets: targets,
    };
    let actual = Instruction::from_raw_instruction(raw, 10.into(), &ConstantPool::new()).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn tableswitch_rejects_invalid_target_counts() {
    for (low, jump_targets) in [(0, vec![]), (i32::MAX, vec![0.into(), 0.into()])] {
        let instruction = Instruction::TableSwitch {
            default: 0.into(),
            low,
            jump_targets,
        };
        let invalid = instruction.into_raw_instruction(0.into(), &mut RawConstantPool::new());
        assert!(invalid.is_err());
    }
}

#[test]
fn tableswitch_rejects_mismatched_raw_targets() {
    let raw = RawInstruction::TableSwitch {
        default: 0,
        low: 1,
        high: 2,
        jump_offsets: vec![0],
    };
    assert!(Instruction::from_raw_instruction(raw, 0.into(), &ConstantPool::new()).is_err());
}

#[test]
fn invokeinterface_uses_interface_method_refs() {
    let mut pool = RawConstantPool::new();
    let raw = Instruction::InvokeInterface(interface_method(), 1)
        .into_raw_instruction(ProgramCounter::default(), &mut pool)
        .unwrap();

    let RawInstruction::InvokeInterface { method_index, .. } = raw else {
        panic!("expected invokeinterface instruction");
    };
    assert!(matches!(
        pool.get_entry(method_index),
        Some(RawEntry::InterfaceMethodRef { .. })
    ));
}

#[test]
fn invokeinterface_rejects_method_refs() {
    let mut pool = RawConstantPool::new();
    let class_name_index = pool
        .put_entry(RawEntry::Utf8(b"a/b/I".as_slice().into()))
        .unwrap();
    let class_index = pool
        .put_entry(RawEntry::Class {
            name_index: class_name_index.into(),
        })
        .unwrap();
    let name_index = pool
        .put_entry(RawEntry::Utf8(b"m".as_slice().into()))
        .unwrap();
    let descriptor_index = pool
        .put_entry(RawEntry::Utf8(b"()V".as_slice().into()))
        .unwrap();
    let name_and_type_index = pool
        .put_entry(RawEntry::NameAndType {
            name_index: name_index.into(),
            descriptor_index: descriptor_index.into(),
        })
        .unwrap();
    let method_index = pool
        .put_entry(RawEntry::MethodRef {
            class_index: class_index.into(),
            name_and_type_index: name_and_type_index.into(),
        })
        .unwrap();

    let pool = ConstantPool::from_raw(pool).unwrap();
    assert!(
        Instruction::from_raw_instruction(
            RawInstruction::InvokeInterface {
                method_index,
                count: 1,
            },
            ProgramCounter::default(),
            &pool,
        )
        .is_err()
    );
}
