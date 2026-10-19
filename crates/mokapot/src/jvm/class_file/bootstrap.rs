use crate::{
    intrinsics::ExactSizeIteratorExt,
    jvm::{
        class::bootstrap::BootstrapMethodData,
        class_file::{ParseError, ResolutionContext, class::raw, resolve::Resolve},
    },
};

impl Resolve for &raw::BootstrapMethod {
    type Output = BootstrapMethodData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let method = ctx
            .constant_pool
            .method_handle_id(self.bootstrap_method_ref.get())?;
        let arguments = self
            .bootstrap_arguments
            .iter()
            .map(|index| ctx.constant_pool.intern_constant(index.get()))
            .try_collect_exact()?;

        Ok(BootstrapMethodData { method, arguments })
    }
}
