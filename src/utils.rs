use std::{borrow::Borrow, collections::HashMap, hash::Hash, mem::transmute, sync::RwLock};

#[derive(Debug)]
pub(crate) struct Cache<K, V> {
    inner: RwLock<HashMap<K, Box<V>>>,
}

impl<K, V> Cache<K, V> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_or_try_put<'c, G, E, Q>(&'c self, key: &Q, generator: G) -> Result<&'c V, E>
    where
        Q: ?Sized + Eq + Hash + ToOwned<Owned = K>,
        K: Eq + Hash + Borrow<Q>,
        G: FnOnce(&K) -> Result<V, E>,
    {
        let cache = match self.inner.read() {
            Ok(it) => it,
            Err(poison_err) => {
                // The operation on `self.cache` should not panic.
                // When the other thread holding the lock get panic, the panic should happen before
                // modifying the cache.
                // Therefore, it is safe to take the lock even if it is poisoned.
                self.inner.clear_poison();
                poison_err.into_inner()
            }
        };
        let item = if let Some(b) = cache.get(key) {
            // SAFETY: We never remove elements from the cache so the `Box` is not dropped until
            // `self.cache` gets dropped, which is when `self` gets dropped.
            // Therefore, it is ok to extend the lifetime of the reference to the lifetime of `self`.
            unsafe { transmute::<&V, &'c V>(b.as_ref()) }
        } else {
            drop(cache); // Release the 
            let mut cache = match self.inner.write() {
                Ok(it) => it,
                Err(poison_err) => {
                    self.inner.clear_poison();
                    poison_err.into_inner()
                },
            };
            // It is possible that the item is added to the cache before we get the write lock.
            // Therefore, we need to check the cache again.
            let item_box = if let Some(b) = cache.get(key) {
                b
            } else {
                let owned_key = key.to_owned();
                let item = generator(&owned_key)?;
                cache
                    .entry(owned_key)
                    .and_modify(|_| panic!("The item is already in the cache"))
                    .or_insert(Box::new(item))
            };
            // SAFETY: We never remove elements from the cache so the `Box` is not dropped until
            // `self.cache` gets dropped, which is when `self` gets dropped.
            // Therefore, it is ok to extend the lifetime of the reference to the lifetime of `self`.
            unsafe { transmute::<&V, &'c V>(item_box.as_ref()) }
        };
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{self, AtomicUsize};

    use super::*;
    use proptest::prelude::*;
    use rayon::prelude::*;

    proptest! {
        #[test]
        fn get_or_try_put_generate_once(key in any::<u32>(), value in any::<u32>()) {
            let cache = Cache::new();
            let counter = AtomicUsize::new(0);
            (0..10).into_par_iter().for_each(|_|{
                let result = cache.get_or_try_put(&key, |_| {
                    counter.fetch_add(1, atomic::Ordering::Relaxed);
                    Ok::<_, ()>(value)
                });
                assert_eq!(&value, result.unwrap());
            });
            assert_eq!(1, counter.load(atomic::Ordering::Relaxed));
        }
    }
}
