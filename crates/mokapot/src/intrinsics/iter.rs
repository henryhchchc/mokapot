/// Collection helpers for iterators with a known length.
pub(crate) trait ExactSizeIteratorExt: ExactSizeIterator {
    /// Collects fallible elements into a boxed slice with capacity reserved up front.
    fn try_collect_exact<T, E>(self) -> Result<Box<[T]>, E>
    where
        Self: Sized + Iterator<Item = Result<T, E>>,
    {
        let mut values = Vec::with_capacity(self.len());
        for item in self {
            values.push(item?);
        }
        Ok(values.into_boxed_slice())
    }
}

impl<I: ExactSizeIterator> ExactSizeIteratorExt for I {}
