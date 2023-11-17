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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_success() {
        let lhs = Some(5);
        let rhs = Some(10);
        let result = merge(lhs, rhs, |a, b| a + b);
        assert_eq!(result, Some(15));
    }

    #[test]
    fn merge_with_none() {
        let lhs = Some(5);
        let rhs = None;
        let result = merge(lhs, rhs, |a, b| a + b);
        assert_eq!(result, Some(5));
    }

    #[test]
    fn try_merge_success() {
        let lhs = Some(5);
        let rhs = Some(10);
        let result = try_merge(lhs, rhs, |a, b| Ok::<_, ()>(a + b));
        assert_eq!(result, Ok(Some(15)));
    }

    #[test]
    fn try_merge_with_none() {
        let lhs = Some(5);
        let rhs = None;
        let result = try_merge(lhs, rhs, |a, b| Ok::<_, ()>(a + b));
        assert_eq!(result, Ok(Some(5)));
    }

    #[test]
    fn try_merge_with_error() {
        let lhs = Some(5);
        let rhs = Some(10);
        let result = try_merge(lhs, rhs, |_, _| Err("Error"));
        assert_eq!(result, Err("Error"));
    }
}
