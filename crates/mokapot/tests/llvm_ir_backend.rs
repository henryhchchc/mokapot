#![cfg(feature = "unstable-llvm-ir")]

use inkwell::context::Context as LLVMContext;
use mokapot::{
    ir::{
        ControlFlowGraph, LocalValue, MokaIRMethod, MokaIRMethodExt, MokaInstruction,
        llvm_ir_backend::lower_method_to_module,
    },
    jvm::{Class, Method},
    jvm::{code::ProgramCounter, method},
    types::field_type::{FieldType, PrimitiveType},
};

fn get_test_class() -> Class {
    let mut bytes = if cfg!(integration_test) {
        include_bytes!(concat!(
            env!("OUT_DIR"),
            "/mokapot/java_classes/org/mokapot/test/TestAnalysis.class"
        ))
        .as_slice()
    } else {
        &[]
    };
    Class::from_reader(&mut bytes).unwrap()
}

fn get_test_method(name: &str) -> Method {
    get_test_class()
        .methods
        .into_iter()
        .find(|method| method.name == name)
        .unwrap()
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn lower_callme_to_llvm_ir() {
    let llvm = LLVMContext::create();
    let method = get_test_method("callMe");
    let ir_method = method.brew().unwrap();
    let module = lower_method_to_module(&llvm, "call_me", &ir_method).unwrap();
    let ir = module.print_to_string().to_string();

    assert!(ir.contains(
        "define i32 @org_mokapot_test_TestAnalysis_callMe__Ljava_lang_String_II_I(ptr %this, ptr %arg0, i32 %arg1, i32 %arg2)"
    ));
    assert!(ir.contains("ret i32 %"));
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn lower_lambda_to_llvm_ir() {
    let llvm = LLVMContext::create();
    let method = get_test_method("lambda$test$0");
    let ir_method = method.brew().unwrap();
    let module = lower_method_to_module(&llvm, "lambda", &ir_method).unwrap();
    let ir = module.print_to_string().to_string();

    assert!(ir.contains(
        "define i32 @org_mokapot_test_TestAnalysis_lambda_test_0__II_I(i32 %arg0, i32 %arg1)"
    ));
    assert_eq!(ir.matches("add i32").count(), 2);
    assert!(ir.contains("ret i32"));
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn lower_test_method_to_llvm_ir() {
    let llvm = LLVMContext::create();
    let method = get_test_method("test");
    let ir_method = method.brew().unwrap();
    let module = lower_method_to_module(&llvm, "test_method", &ir_method).unwrap();
    let ir = module.print_to_string().to_string();

    assert!(ir.contains(
        "define i32 @org_mokapot_test_TestAnalysis_test__II_I(ptr %this, i32 %arg0, i32 %arg1)"
    ));
    assert!(ir.contains("switch i32"));
    assert!(ir.contains("@mokapot_abi_phi_i32_2"));
    assert!(ir.contains("%mokapot_array_i32 = type { i32, ptr }"));
    assert!(ir.contains("array_length_ptr"));
    assert!(ir.contains("array_data_ptr_ptr"));
    assert!(ir.contains("array_element_ptr"));
    assert!(
        ir.contains("@mokapot_abi_field_get_static_ref_java_lang_System_out_Ljava_io_PrintStream_")
    );
    assert!(ir.contains("@mokapot_abi_const_string"));
    assert!(ir.contains("@org_mokapot_test_TestAnalysis_callMe__Ljava_lang_String_II_I"));
    assert!(ir.contains("ret i32 %"));
}

#[test]
fn lower_manual_long_array_to_llvm_ir() {
    let llvm = LLVMContext::create();
    let length = LocalValue::new(0);
    let array = LocalValue::new(1);
    let method = MokaIRMethod {
        access_flags: method::AccessFlags::STATIC,
        name: "makeLongArray".to_owned(),
        descriptor: "() [J".replace(' ', "").parse().unwrap(),
        owner: mokapot::jvm::references::ClassRef::new("org/mokapot/test/ArrayLowering"),
        instructions: [
            (
                ProgramCounter::ZERO,
                MokaInstruction::Definition {
                    value: length,
                    expr: mokapot::ir::expression::Expression::Const(
                        mokapot::jvm::ConstantValue::Integer(4),
                    ),
                },
            ),
            (
                ProgramCounter::from(1),
                MokaInstruction::Definition {
                    value: array,
                    expr: mokapot::ir::expression::Expression::Array(
                        mokapot::ir::expression::ArrayOperation::New {
                            element_type: FieldType::Base(PrimitiveType::Long),
                            length: length.as_operand(),
                        },
                    ),
                },
            ),
            (
                ProgramCounter::from(2),
                MokaInstruction::Return(Some(array.as_operand())),
            ),
        ]
        .into_iter()
        .collect(),
        exception_table: vec![],
        control_flow_graph: ControlFlowGraph::from_edges([
            (
                ProgramCounter::ZERO,
                ProgramCounter::from(1),
                mokapot::ir::control_flow::ControlTransfer::Unconditional,
            ),
            (
                ProgramCounter::from(1),
                ProgramCounter::from(2),
                mokapot::ir::control_flow::ControlTransfer::Unconditional,
            ),
        ]),
    };

    let module = lower_method_to_module(&llvm, "manual_array", &method).unwrap();
    let ir = module.print_to_string().to_string();

    assert!(ir.contains("%mokapot_array_i64 = type { i32, ptr }"));
    assert!(ir.contains("array_data"));
    assert!(ir.contains("ret ptr %"));
}
