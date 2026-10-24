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
            inner_class_info_index,
            outer_class_info_index,
            inner_name_index,
            inner_class_access_flags,
        } = self;

        let inner_class = ctx.constant_pool.class_id(inner_class_info_index.get())?;
        let outer_class = optional_id(outer_class_info_index.get(), |index| {
            ctx.constant_pool.class_id(index)
        })?;
        let inner_name = optional_id(inner_name_index.get(), |index| {
            ctx.constant_pool.utf8_id(index)
        })?;
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
        let Self {
            class_index,
            method_index,
        } = self;

        let class = ctx.constant_pool.class_id(class_index.get())?;
        let method = if method_index.get() == 0 {
            None
        } else {
            Some(
                ctx.constant_pool
                    .method_name_and_type_id(method_index.get())?,
            )
        };

        Ok(EnclosingMethodData { class, method })
    }
}

fn optional_id<T>(
    index: u16,
    resolve: impl FnOnce(u16) -> Result<T, ParseError>,
) -> Result<Option<T>, ParseError> {
    if index == 0 {
        Ok(None)
    } else {
        resolve(index).map(Some)
    }
}
