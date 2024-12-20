use inkwell::{builder::Builder, intrinsics::Intrinsic, module::Module};

/// Invokes [`llvm.donothing()`](https://llvm.org/docs/LangRef.html#llvm-donothing-intrinsic).
pub(super) fn invoke_donothing<'ctx>(module: &Module<'ctx>, builder: &Builder<'ctx>) {
    let intrinsic = Intrinsic::find("llvm.donothing").unwrap();
    let intrinsic_fn = intrinsic.get_declaration(module, &[]).unwrap();

    builder.build_call(intrinsic_fn, &[], "").unwrap();
}
