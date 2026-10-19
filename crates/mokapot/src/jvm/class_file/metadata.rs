//! Class metadata attribute resolution.

use crate::jvm::class_file::{ParseError, ResolutionContext, attribute::raw, resolve::Resolve};

use crate::intrinsics::ExactSizeIteratorExt;

impl Resolve for raw::Signature {
    type Output = super::constant_pool::Utf8Id;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool.utf8_id(self.signature_index.get())
    }
}

impl Resolve for raw::SourceFile {
    type Output = super::constant_pool::Utf8Id;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool.utf8_id(self.sourcefile_index.get())
    }
}

impl Resolve for raw::ModuleMainClass {
    type Output = super::constant_pool::ClassId;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool.class_id(self.main_class_index.get())
    }
}

impl Resolve for raw::NestHost {
    type Output = super::constant_pool::ClassId;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool.class_id(self.host_class_index.get())
    }
}

impl Resolve for &raw::ModulePackages {
    type Output = Box<[super::constant_pool::PackageId]>;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        self.package_index
            .iter()
            .map(|index| ctx.constant_pool.package_id(index.get()))
            .try_collect_exact()
    }
}

impl Resolve for &raw::NestMembers {
    type Output = Box<[super::constant_pool::ClassId]>;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_class_ids(&self.classes, ctx)
    }
}

impl Resolve for &raw::PermittedSubclasses {
    type Output = Box<[super::constant_pool::ClassId]>;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_class_ids(&self.classes, ctx)
    }
}

fn resolve_class_ids(
    indices: &[zerocopy::byteorder::big_endian::U16],
    ctx: &mut ResolutionContext,
) -> Result<Box<[super::constant_pool::ClassId]>, ParseError> {
    indices
        .iter()
        .map(|index| ctx.constant_pool.class_id(index.get()))
        .try_collect_exact()
}
