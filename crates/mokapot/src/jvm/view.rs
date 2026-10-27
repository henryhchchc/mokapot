//! Shared implementation for indexed borrowed views.

macro_rules! define_view_list {
    (
        $(#[$docs:meta])*
        pub struct $list:ident<$class:lifetime>;
        iterator $iter:ident;
        $data:ty => $view:ty;
        |$owner:ident, $item:ident| $project:expr
    ) => {
        $(#[$docs])*
        #[derive(Clone, Copy)]
        pub struct $list<$class> {
            class: &$class $crate::jvm::Class,
            data: &$class [$data],
        }

        impl<$class> $list<$class> {
            const fn new(class: &$class $crate::jvm::Class, data: &$class [$data]) -> Self {
                Self { class, data }
            }

            /// Iterates over the values.
            #[must_use]
            pub fn iter(self) -> $iter<$class> {
                $iter {
                    class: self.class,
                    inner: self.data.iter(),
                }
            }

            /// Returns the value at the given index.
            #[must_use]
            pub fn get(self, index: usize) -> Option<$view> {
                self.data.get(index).map(|data| $iter::project(self.class, data))
            }

            /// Returns the number of values.
            #[must_use]
            pub const fn len(self) -> usize {
                self.data.len()
            }

            /// Returns whether the list is empty.
            #[must_use]
            pub const fn is_empty(self) -> bool {
                self.data.is_empty()
            }
        }

        impl std::fmt::Debug for $list<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.iter()).finish()
            }
        }

        impl<$class> IntoIterator for $list<$class> {
            type Item = $view;
            type IntoIter = $iter<$class>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<$class> IntoIterator for &$list<$class> {
            type Item = $view;
            type IntoIter = $iter<$class>;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        #[doc = concat!("An iterator over [`", stringify!($list), "`].")]
        #[derive(Clone)]
        pub struct $iter<$class> {
            class: &$class $crate::jvm::Class,
            inner: std::slice::Iter<$class, $data>,
        }

        impl<$class> $iter<$class> {
            fn project($owner: &$class $crate::jvm::Class, $item: &$class $data) -> $view {
                $project
            }
        }

        impl std::fmt::Debug for $iter<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.clone()).finish()
            }
        }

        impl<$class> Iterator for $iter<$class> {
            type Item = $view;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next().map(|data| Self::project(self.class, data))
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
        }

        impl DoubleEndedIterator for $iter<'_> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.inner.next_back().map(|data| Self::project(self.class, data))
            }
        }

        impl ExactSizeIterator for $iter<'_> {}
        impl std::iter::FusedIterator for $iter<'_> {}
    };
}

pub(super) use define_view_list;
