//! Resolution of decoded class-file structures.

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

/// Borrows a fixed-width table before resolving its entries.
pub(super) fn resolve_table<'input, R>(
    bytes: &mut &'input [u8],
    count: usize,
    ctx: &ResolutionContext,
) -> Result<Vec<R::Output>, ParseError>
where
    R: super::fixed_layout::FixedLayout + zerocopy::KnownLayout + Copy + Resolve + 'input,
{
    super::decoder::borrow_slice::<R>(bytes, count)?
        .iter()
        .copied()
        .map(|raw| raw.resolve(ctx))
        .collect()
}
