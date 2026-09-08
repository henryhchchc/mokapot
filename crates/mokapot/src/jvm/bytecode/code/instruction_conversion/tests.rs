use crate::{
    jvm::{
        JavaString,
        class::{ConstantPool, constant_pool::Entry},
        code::{Instruction, ProgramCounter, RawInstruction},
        references::MethodRef,
    },
    types::reference_type::ReferenceType,
};

fn interface_method() -> MethodRef {
    MethodRef {
        owner: ReferenceType::Class("example/Interface".parse().unwrap()),
        name: "method".to_owned(),
        descriptor: "()V".parse().unwrap(),
    }
}

#[test]
fn tableswitch_default_can_target_backward() {
    let raw = Instruction::TableSwitch {
        default: ProgramCounter::from(5),
        range: 0..=0,
        jump_targets: vec![ProgramCounter::from(10)],
    }
    .into_raw_instruction(ProgramCounter::from(10), &mut ConstantPool::new())
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
fn invokeinterface_uses_interface_method_refs() {
    let mut pool = ConstantPool::new();
    let raw = Instruction::InvokeInterface(interface_method(), 1)
        .into_raw_instruction(ProgramCounter::default(), &mut pool)
        .unwrap();

    let RawInstruction::InvokeInterface { method_index, .. } = raw else {
        panic!("expected invokeinterface instruction");
    };
    assert!(matches!(
        pool.get_entry(method_index),
        Some(Entry::InterfaceMethodRef { .. })
    ));
}

#[test]
fn invokeinterface_rejects_method_refs() {
    let mut pool = ConstantPool::new();
    let class_name_index = pool
        .put_entry(Entry::Utf8(JavaString::Utf8(
            "example/Interface".to_owned(),
        )))
        .unwrap();
    let class_index = pool
        .put_entry(Entry::Class {
            name_index: class_name_index,
        })
        .unwrap();
    let name_index = pool
        .put_entry(Entry::Utf8(JavaString::Utf8("method".to_owned())))
        .unwrap();
    let descriptor_index = pool
        .put_entry(Entry::Utf8(JavaString::Utf8("()V".to_owned())))
        .unwrap();
    let name_and_type_index = pool
        .put_entry(Entry::NameAndType {
            name_index,
            descriptor_index,
        })
        .unwrap();
    let method_index = pool
        .put_entry(Entry::MethodRef {
            class_index,
            name_and_type_index,
        })
        .unwrap();

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
