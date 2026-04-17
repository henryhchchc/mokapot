#![cfg(feature = "unstable-llvm-ir")]

use inkwell::context::Context as LLVMContext;
use mokapot::{
    ir::{MokaIRMethodExt, llvm_ir_backend::lower_method_to_module},
    jvm::{Class, Method},
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
    assert!(ir.contains("@mokapot_runtime_new_array_I"));
    assert!(ir.contains("@mokapot_runtime_get_static_java_lang_System_out_Ljava_io_PrintStream_"));
    assert!(ir.contains("@org_mokapot_test_TestAnalysis_callMe__Ljava_lang_String_II_I"));
    assert!(ir.contains("ret i32 %"));
}
