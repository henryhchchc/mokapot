//! Nested and enclosing class attribute resolution.

use crate::jvm::{
    class::{
        NestedClassAccessFlags,
        nesting::{EnclosingMethodData, InnerClassData},
    },
    class_file::{ParseError, ResolutionContext, resolve::Resolve},
};

use super::class::raw as class_raw;
use super::flags::parse_flags;

impl Resolve for class_raw::InnerClass {
    type Output = InnerClassData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let Self {
            inner_class_info,
            outer_class_info,
            inner_name,
            inner_class_access_flags,
        } = self;

        let inner_class = ctx.constant_pool.class_id(inner_class_info)?;
        let outer_class = outer_class_info
            .get()
            .map(|index| ctx.constant_pool.class_id(index))
            .transpose()?;
        let inner_name = inner_name
            .get()
            .map(|index| ctx.constant_pool.utf8_id(index))
            .transpose()?;
        let access_flags = parse_flags::<NestedClassAccessFlags>(
            inner_class_access_flags.get(),
            "Invalid nested class access flags",
        )?;

        Ok(InnerClassData {
            inner_class,
            outer_class,
            inner_name,
            access_flags,
        })
    }
}

impl Resolve for class_raw::EnclosingMethod {
    type Output = EnclosingMethodData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let Self { class, method } = self;

        let class = ctx.constant_pool.class_id(class)?;
        let method = method
            .map(|index| ctx.constant_pool.method_name_and_type_id(index))
            .transpose()?;

        Ok(EnclosingMethodData { class, method })
    }
}
