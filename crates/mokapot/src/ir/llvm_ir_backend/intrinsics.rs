use inkwell::intrinsics::Intrinsic;

use crate::ir::llvm_ir_backend::Context;

/// Invokes [`llvm.donothing()`](https://llvm.org/docs/LangRef.html#llvm-donothing-intrinsic).
pub(super) fn invoke_donothing(ctx: &Context<'_, '_>) {
    let intrinsic = Intrinsic::find("llvm.donothing").unwrap();
    let intrinsic_fn = intrinsic.get_declaration(&ctx.module, &[]).unwrap();

    ctx.builder.build_call(intrinsic_fn, &[], "").unwrap();
}
