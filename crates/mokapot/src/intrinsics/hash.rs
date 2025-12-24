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
    iter.into_iter()
        .map(|ref it| build_hasher.hash_one(it))
        .fold(
            (0u64, 1u64, 0u64, 0u64),
            |(sum, product, xor, count), item_hash| {
                (
                    // Wrapping add is order-independent and duplicate-sensitive
                    sum.wrapping_add(item_hash),
                    // Use wrapping multiplication with an offset to avoid zero-product issues
                    product.wrapping_mul((item_hash << 1) | 1),
                    // XOR with rotation reduces pattern collisions
                    xor ^ item_hash.rotate_left((item_hash & 63) as u32),
                    count + 1,
                )
            },
        )
        .hash(state);
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
    use proptest::prelude::*;
    use rand::seq::SliceRandom as _;

    proptest! {
        #[test]
        fn order_independent(
            elements in prop::collection::vec(any::<i32>(), 0..100)
        ) {
            let hash1 = compute_unordered_hash(elements.iter());

            let mut vec2 = elements.clone();
            vec2.reverse();
            let hash2 = compute_unordered_hash(vec2.iter());

            let mut vec3 = elements.clone();
            if vec3.len() > 1 {
                vec3.rotate_left(1);
            }
            let hash3 = compute_unordered_hash(vec3.iter());

            let mut vec4 = elements.clone();
            vec4.shuffle(&mut rand::rng());
            let hash4 = compute_unordered_hash(vec4.iter());

            prop_assert_eq!(hash1, hash2);
            prop_assert_eq!(hash2, hash3);
            prop_assert_eq!(hash3, hash4);
        }
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

    proptest! {
        #[test]
        fn different_sizes_differ(
            elements in prop::collection::vec(any::<i32>(), 1..100)
        ) {
            let hash1 = compute_unordered_hash(elements.iter());

            let mut extended = elements.clone();
            extended.push(0);
            let hash2 = compute_unordered_hash(extended.iter());

            prop_assert_ne!(hash1, hash2);
        }
    }

    proptest! {
        #[test]
        fn same_elements_same_hash(element in any::<i32>(), count in 1usize..10) {
            let vec1: Vec<_> = std::iter::repeat_n(element, count).collect();
            let vec2: Vec<_> = std::iter::repeat_n(element, count).collect();

            let hash1 = compute_unordered_hash(vec1.iter());
            let hash2 = compute_unordered_hash(vec2.iter());

            prop_assert_eq!(hash1, hash2);
        }
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
