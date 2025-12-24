use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher};

/// Hashes an iterator's items in an order-independent manner using
/// symmetric polynomial-inspired combining.
///
/// This produces a hash that is:
/// - Order-independent: `{a, b, c}` hashes the same regardless of iteration order
/// - Duplicate-sensitive: `{a, a, b}` differs from `{a, b}`
/// - Collision-resistant: Uses sum, product, and count to avoid simple collisions
///
/// Uses the default hasher ([`DefaultHasher`]) for computing individual item hashes.
/// For custom hasher support, use [`hash_unordered_with_hasher`].
#[inline]
pub(crate) fn hash_unordered<I, H>(iter: I, state: &mut H)
where
    I: IntoIterator,
    I::Item: Hash,
    H: Hasher,
{
    hash_unordered_with_hasher(iter, state, &BuildHasherDefault::<DefaultHasher>::default());
}

/// Hashes an iterator's items in an order-independent manner using
/// symmetric polynomial-inspired combining with a custom [`BuildHasher`].
///
/// This produces a hash that is:
/// - Order-independent: `{a, b, c}` hashes the same regardless of iteration order
/// - Duplicate-sensitive: `{a, a, b}` differs from `{a, b}`
/// - Collision-resistant: Uses sum, product, and count to avoid simple collisions
#[inline]
pub(crate) fn hash_unordered_with_hasher<I, H, B>(iter: I, state: &mut H, build_hasher: &B)
where
    I: IntoIterator,
    I::Item: Hash,
    H: Hasher,
    B: BuildHasher,
{
    let mut sum: u64 = 0;
    let mut product: u128 = 1;
    let mut xor: u64 = 0;
    let mut count: u64 = 0;

    for ref item in iter {
        let item_hash = build_hasher.hash_one(item);
        // Wrapping add is order-independent and duplicate-sensitive
        sum = sum.wrapping_add(item_hash);
        // Use wrapping multiplication with an offset to avoid zero-product issues
        product = product.wrapping_mul(u128::from(item_hash) | 1);
        // XOR with rotation reduces pattern collisions
        xor ^= item_hash.rotate_left((item_hash & 63) as u32);
        count += 1;
    }

    // Combine all components into the final hash state
    // The count ensures collections of different sizes are distinguishable
    sum.hash(state);
    product.hash(state);
    xor.hash(state);
    count.hash(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compute_unordered_hash<I>(iter: I) -> u64
    where
        I: Iterator,
        I::Item: Hash,
    {
        let mut hasher = DefaultHasher::new();
        hash_unordered(iter, &mut hasher);
        hasher.finish()
    }

    #[test]
    fn order_independent() {
        let hash1 = compute_unordered_hash([1, 2, 3].iter());
        let hash2 = compute_unordered_hash([3, 1, 2].iter());
        let hash3 = compute_unordered_hash([2, 3, 1].iter());

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn duplicate_sensitive() {
        let hash1 = compute_unordered_hash([1, 2, 3].iter());
        let hash2 = compute_unordered_hash([1, 2, 3, 3].iter());
        let hash3 = compute_unordered_hash([1, 1, 2, 3].iter());

        assert_ne!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash2, hash3);
    }

    #[test]
    fn empty_collection() {
        let hash_empty = compute_unordered_hash(std::iter::empty::<i32>());
        let hash_one = compute_unordered_hash([0].iter());

        assert_ne!(hash_empty, hash_one);
    }

    #[test]
    fn different_sizes_differ() {
        let hash1 = compute_unordered_hash([1, 2].iter());
        let hash2 = compute_unordered_hash([1, 2, 3].iter());

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn same_elements_same_hash() {
        let hash1 = compute_unordered_hash([42, 42, 42].iter());
        let hash2 = compute_unordered_hash([42, 42, 42].iter());

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn works_with_strings() {
        let hash1 = compute_unordered_hash(["apple", "banana", "cherry"].iter());
        let hash2 = compute_unordered_hash(["cherry", "apple", "banana"].iter());
        let hash3 = compute_unordered_hash(["apple", "banana"].iter());

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
