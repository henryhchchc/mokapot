pub(crate) fn merge<T, M>(lhs: Option<T>, rhs: Option<T>, merge: M) -> Option<T>
where
    M: FnOnce(T, T) -> T,
{
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => Some(merge(lhs, rhs)),
        (lhs, rhs) => lhs.or(rhs),
    }
}

pub(crate) fn try_merge<T, M, E>(lhs: Option<T>, rhs: Option<T>, merge: M) -> Result<Option<T>, E>
where
    M: FnOnce(T, T) -> Result<T, E>,
{
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => merge(lhs, rhs).map(Some),
        (lhs, rhs) => Ok(lhs.or(rhs)),
    }
}
