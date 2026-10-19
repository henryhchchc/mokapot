use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher};

/// Trait for hashing collections in an order-independent manner.
///
/// This trait can be implemented by collection types to provide
/// order-independent hashing capabilities, where the hash value
/// is the same regardless of the iteration order of elements.
pub(crate) trait HashUnordered {
    /// Hashes the collection's items in an order-independent manner.
    ///
    /// # Arguments
    ///
    /// * `state` - The hasher state to write the computed hash to
    fn hash_unordered<H>(self, state: &mut H)
    where
        H: Hasher;
}

impl<I> HashUnordered for I
where
    I: IntoIterator,
    I::Item: Hash,
{
    #[inline]
    fn hash_unordered<H>(self, state: &mut H)
    where
        H: Hasher,
    {
        hash_unordered_with_hasher(self, state, &BuildHasherDefault::<DefaultHasher>::default());
    }
}

/// Hashes an iterator's items in an order-independent manner using
/// symmetric polynomial-inspired combining with a custom [`BuildHasher`].
///
/// This produces a hash that is:
/// - Order-independent: `{a, b, c}` hashes the same regardless of iteration order
/// - Duplicate-sensitive: `{a, a, b}` differs from `{a, b}`
/// - Collision-resistant: Uses sum, product, and count to avoid simple collisions
#[inline]
fn hash_unordered_with_hasher<I, H, B>(iter: I, state: &mut H, build_hasher: &B)
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
        iter.hash_unordered(&mut hasher);
        hasher.finish()
    }
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn order_independent(
            elements in prop::collection::vec(any::<i32>(), 0..100),
            swaps in prop::collection::vec((0usize..100, 0usize..100), 0..100)
        ) {
            let hash1 = compute_unordered_hash(elements.iter());

            let mut permuted = elements;
            permuted.reverse();
            prop_assert_eq!(hash1, compute_unordered_hash(permuted.iter()));
            for (left, right) in swaps {
                let len = permuted.len();
                if len > 0 {
                    permuted.swap(left % len, right % len);
                }
            }
            prop_assert_eq!(hash1, compute_unordered_hash(permuted.iter()));
        }
    }
}
