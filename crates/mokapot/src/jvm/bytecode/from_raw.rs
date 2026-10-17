use super::{ParseError, ParsingContext};

/// Resolves a raw JVM structure into its semantic model.
pub(super) trait FromRaw<'a>: Sized {
    type Raw;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError>;
}
