use super::*;
use crate::jvm::class_file::constant_pool::{RawConstantPool, RawEntry};
use crate::jvm::{Class, code::Instruction};

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
    assert_eq!(
        data.view(&class),
        Instruction::InvokeInterface(
            &crate::jvm::references::MethodRef {
                owner: crate::types::reference_type::ReferenceType::Class("a/b/I".parse().unwrap()),
                name: "m".into(),
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
    let mut pool = interface_method_pool(false);
    let error = InstructionData::from_raw_instruction(instruction, pc, &mut pool).unwrap_err();
    assert_eq!(error.kind(), crate::jvm::errors::ParseErrorKind::Malformed);
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
        let name = pool.insert_class_name("Test".parse().unwrap());
        let class = Class {
            name,
            constant_pool: pool,
            ..Class::default()
        };
        let method = method(
            instructions
                .iter()
                .map(|(pc, data)| (*pc, data.view(&class))),
            "()I",
            vec![],
        );
        let ir = MokaIRMethod::from_method(method.as_method()).unwrap();
        let definitions = operations(&ir).collect::<Vec<_>>();
        assert_matches!(definitions.as_slice(), [Operation::Definition { value, expr: Expression::Const(ConstantValue::Integer(actual)) }]
            if *actual == expected && matches!(&ir.block(ir.entry.block).unwrap().terminator, Terminator::Return { value: Some(returned), .. } if returned == value));
    }
}

fn borrowing_instruction_fixture() -> (Class, crate::jvm::code::MethodBodyData) {
    let raw = RawConstantPool::from_entries([
        RawEntry::Utf8(b"a/b/C"),
        RawEntry::Class {
            name_index: 1.into(),
        },
        RawEntry::Utf8(b"value"),
        RawEntry::Utf8(b"I"),
        RawEntry::NameAndType {
            name_index: 3.into(),
            descriptor_index: 4.into(),
        },
        RawEntry::FieldRef {
            class_index: 2.into(),
            name_and_type_index: 5.into(),
        },
        RawEntry::Utf8(b"m"),
        RawEntry::Utf8(b"()V"),
        RawEntry::NameAndType {
            name_index: 7.into(),
            descriptor_index: 8.into(),
        },
        RawEntry::MethodRef {
            class_index: 2.into(),
            name_and_type_index: 9.into(),
        },
        RawEntry::InvokeDynamic {
            bootstrap_method_attr_index: 3.into(),
            name_and_type_index: 9.into(),
        },
        RawEntry::Integer(42.into()),
    ]);
    let mut pool = ConstantPool::from_raw(raw).unwrap();
    let bytes = [
        0xb2, 0, 6, 0xb4, 0, 6, 0xb6, 0, 10, 0xb8, 0, 10, 0xba, 0, 11, 0, 0, 0x12, 12, 0x13, 0, 12,
        0xbb, 0, 2, 0xc0, 0, 2, 0xbd, 0, 2, 0xb1,
    ];
    let instructions = RawInstruction::iter_from_bytes(&bytes)
        .map(|entry| {
            let (pc, raw) = entry.unwrap();
            (
                pc,
                InstructionData::from_raw_instruction(raw, pc, &mut pool).unwrap(),
            )
        })
        .collect();
    let class = Class {
        name: pool.class_id(2).unwrap(),
        constant_pool: pool.finish(),
        ..Class::default()
    };
    let fixture = crate::tests::method(
        [(0, Instruction::Return)],
        "()V",
        vec![],
        crate::jvm::method::AccessFlags::PUBLIC,
    );
    let mut data = fixture.body.clone().unwrap();
    data.instructions = instructions;
    (class, data)
}

#[test]
fn instructions_borrow_pool_operands_and_preserve_lookup() {
    use crate::jvm::{ConstantValue, code::MethodBody};

    let (class, data) = borrowing_instruction_fixture();
    let body = MethodBody::new(&class, &data);
    let list = body.instructions();
    assert_eq!(list.len(), 11);
    assert!(!list.is_empty());
    assert!(body.instruction_at(1.into()).is_none());
    assert_eq!(
        list.entry_point(),
        body.instruction_at(0.into())
            .map(|instruction| (0.into(), instruction))
    );
    assert_eq!(list.next_pc_of(&1.into()), Some(3.into()));
    assert_eq!(list.prev_pc_of(&1.into()), Some(0.into()));
    let range = {
        use std::ops::Bound::Included;
        let start = ProgramCounter::from(3);
        let end = ProgramCounter::from(9);
        list.range((Included(&start), Included(&end)))
    };
    assert_eq!(range.len(), 3);
    assert_eq!(
        list.iter().next_back(),
        Some((31.into(), Instruction::Return))
    );
    let field = |pc| match body.instruction_at(pc).unwrap() {
        Instruction::GetStatic(field) | Instruction::GetField(field) => field,
        _ => panic!("expected a field access"),
    };
    assert!(std::ptr::eq(field(0.into()), field(3.into())));
    let method = |pc| match body.instruction_at(pc).unwrap() {
        Instruction::InvokeVirtual(method) | Instruction::InvokeStatic(method) => method,
        _ => panic!("expected a method invocation"),
    };
    assert!(std::ptr::eq(method(6.into()), method(9.into())));
    let constant = |pc| match body.instruction_at(pc).unwrap() {
        Instruction::Ldc(value) | Instruction::LdcW(value) => value,
        _ => panic!("expected a constant"),
    };
    assert_eq!(constant(17.into()), &ConstantValue::Integer(42));
    assert!(std::ptr::eq(constant(17.into()), constant(19.into())));
    let Instruction::InvokeDynamic {
        bootstrap_method_index,
        name,
        descriptor,
    } = body.instruction_at(12.into()).unwrap()
    else {
        panic!("expected invokedynamic")
    };
    assert_eq!(bootstrap_method_index, 3);
    assert_eq!(name, "m");
    assert_eq!(descriptor, &"()V".parse().unwrap());
    let Instruction::New(class_name) = body.instruction_at(22.into()).unwrap() else {
        panic!("expected new")
    };
    assert!(std::ptr::eq(class_name, class.name()));
    let Instruction::CheckCast(target) = body.instruction_at(25.into()).unwrap() else {
        panic!("expected checkcast")
    };
    let Instruction::ANewArray(element) = body.instruction_at(28.into()).unwrap() else {
        panic!("expected anewarray")
    };
    assert!(std::ptr::eq(target, element));
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
