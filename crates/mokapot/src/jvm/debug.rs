use std::fmt;

pub(crate) struct DebugList<F>(F);

impl<F> DebugList<F> {
    pub(crate) const fn new(factory: F) -> Self {
        Self(factory)
    }
}

impl<F, I> fmt::Debug for DebugList<F>
where
    F: Fn() -> I,
    I: IntoIterator,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((self.0)()).finish()
    }
}
