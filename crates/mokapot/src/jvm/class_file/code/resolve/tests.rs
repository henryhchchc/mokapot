use super::*;
use crate::jvm::class_file::constant_pool::{RawConstantPool, RawEntry};
use crate::jvm::{
    Class,
    code::Instruction,
    references::{FieldRef, MethodRef},
};
use crate::types::{class_name::ClassName, reference_type::ReferenceType};

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
    let actual =
        InstructionData::from_raw_instruction(raw, 10.into(), &mut ConstantPool::new()).unwrap();
    assert_eq!(
        actual.view(&Class::default()),
        Instruction::TableSwitch {
            default: 5.into(),
            low: -1,
            jump_targets: &[10.into(), 11.into()],
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
    let mut pool = interface_method_pool(true);
    let data = InstructionData::from_raw_instruction(instruction, pc, &mut pool).unwrap();
    let name = pool.insert_class_name("Test".parse().unwrap());
    let class = Class {
        name,
        constant_pool: pool,
        ..Class::default()
    };
    let Instruction::InvokeInterface(method, count) = data.view(&class) else {
        panic!("expected invokeinterface")
    };
    assert_eq!(count, 1);
    assert_eq!(
        MethodRef::from(method),
        MethodRef {
            owner: "a/b/I".parse().unwrap(),
            name: "m".into(),
            descriptor: "()V".parse().unwrap(),
        }
    );
}

#[test]
fn push_immediates_preserve_signed_values_through_ir() {
    use crate::ir::test::prelude::*;
    use crate::jvm::ConstantValue;

    let cases = [
        (vec![0x10, 0x80, 0xac], -128),
        (vec![0x10, 0x7f, 0xac], 127),
        (vec![0x11, 0x80, 0x00, 0xac], -32768),
        (vec![0x11, 0x7f, 0xff, 0xac], 32767),
    ];
    for (bytes, expected) in cases {
        let mut pool = ConstantPool::new();
        let instructions = RawInstruction::iter_from_bytes(&bytes)
            .map(|item| {
                let (pc, raw) = item?;
                InstructionData::from_raw_instruction(raw, pc, &mut pool)
                    .map(|instruction| (pc, instruction))
            })
            .collect::<Result<Vec<_>, ParseError>>()
            .unwrap();
        let method = method_with_pool(pool, instructions, "()I", vec![]);
        let ir = MokaIRMethod::from_method(method.as_method()).unwrap();
        let definitions = operations(&ir).collect::<Vec<_>>();
        assert_matches!(definitions.as_slice(), [Operation::Definition { value, expr: Expression::Const(ConstantValue::Integer(actual)) }]
            if *actual == expected && matches!(&ir.block(ir.entry.block).unwrap().terminator, Terminator::Return { value: Some(returned), .. } if returned == value));
    }
}

#[test]
fn instructions_map_pool_operands_to_borrowed_views() {
    let mut pool = ConstantPool::new();
    let name: ClassName = "a/b/C".parse().unwrap();
    let class_id = pool.insert_class_name(name.clone());
    let field_id = pool.insert_field_ref(FieldRef {
        owner: ReferenceType::Class(name.clone()),
        name: "value".into(),
        field_type: "I".parse().unwrap(),
    });
    let method_id = pool.insert_method_ref(MethodRef {
        owner: ReferenceType::Class(name.clone()),
        name: "m".into(),
        descriptor: "()V".parse().unwrap(),
    });
    let class = Class {
        name: class_id,
        constant_pool: pool.finish(),
        ..Class::default()
    };
    let instructions = [
        InstructionData::GetStatic(field_id),
        InstructionData::GetField(field_id),
        InstructionData::InvokeVirtual(method_id),
        InstructionData::InvokeStatic(method_id),
        InstructionData::New(class_id),
    ];
    let [
        Instruction::GetStatic(static_field),
        Instruction::GetField(instance_field),
        Instruction::InvokeVirtual(virtual_method),
        Instruction::InvokeStatic(static_method),
        Instruction::New(new_class),
    ] = instructions
        .each_ref()
        .map(|instruction| instruction.view(&class))
    else {
        panic!("instruction views preserve their variants")
    };
    assert!(std::ptr::eq(static_field.name(), instance_field.name()));
    assert!(std::ptr::eq(
        virtual_method.descriptor(),
        static_method.descriptor()
    ));
    assert!(std::ptr::eq(new_class, class.name()));
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
                assert_eq!(
                    result.unwrap_err().kind(),
                    crate::jvm::errors::ParseErrorKind::Malformed
                );
            }
        }
    }
}
