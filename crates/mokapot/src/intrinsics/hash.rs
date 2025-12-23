use std::{
    hash::{DefaultHasher, Hash, Hasher},
    iter::once,
};

pub(crate) fn hash_unordered<I, H>(iter: I, state: &mut H)
where
    I: Iterator,
    I::Item: Hash,
    H: Hasher,
{
    let aggregated: u128 = once(0x9e37_79b9_9e37_79b9_9e37_79b9)
        .chain(iter.map(|item| {
            let mut hasher = DefaultHasher::new();
            item.hash(&mut hasher);
            u128::from(hasher.finish())
        }))
        .sum();
    aggregated.hash(state);
}
