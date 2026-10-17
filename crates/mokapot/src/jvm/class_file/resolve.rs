//! Resolution of decoded class-file structures.

use crate::intrinsics::ExactSizeIteratorExt;

use super::{Decode, ParseError, ResolutionContext};

/// Resolves constant-pool references and validates a decoded structure.
pub(super) trait Resolve: Sized {
    type Output;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError>;
}

/// Decodes and resolves one structure within a bounded payload.
pub(super) fn decode_resolved<'input, R>(
    bytes: &mut &'input [u8],
    ctx: &ResolutionContext,
) -> Result<R::Output, ParseError>
where
    R: Decode<'input> + Resolve,
{
    R::decode(bytes)?.resolve(ctx)
}

/// Resolves fixed-width raw entries into a boxed slice.
pub(super) fn resolve_table<R: Copy + Resolve>(
    entries: &[R],
    ctx: &ResolutionContext,
) -> Result<Box<[R::Output]>, ParseError> {
    entries
        .iter()
        .copied()
        .map(|raw| raw.resolve(ctx))
        .try_collect_exact()
}
