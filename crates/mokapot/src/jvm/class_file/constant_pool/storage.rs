//! Shared indexed storage for raw and resolved constant pools.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Slot<T> {
    Entry(T),
    Padding,
}

impl<T> Slot<T> {
    pub const fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Entry(entry) => Some(entry),
            Self::Padding => None,
        }
    }

    pub fn take_if(&mut self, predicate: impl FnOnce(&T) -> bool) -> Option<T> {
        if let Self::Entry(entry) = self
            && predicate(entry)
            && let Self::Entry(entry) = std::mem::replace(self, Self::Padding)
        {
            Some(entry)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PoolStorage<T> {
    slots: Box<[Slot<T>]>,
}

impl<T> PoolStorage<T> {
    pub const fn from_slots(slots: Box<[Slot<T>]>) -> Self {
        Self { slots }
    }

    pub fn into_slots(self) -> Box<[Slot<T>]> {
        self.slots
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "parsed pool counts and test fixtures are bounded by u16"
    )]
    pub const fn count(&self) -> u16 {
        self.slots.len() as u16
    }

    pub fn with_padding(count: u16) -> Self {
        Self {
            slots: (0..count).map(|_| Slot::Padding).collect(),
        }
    }

    pub fn get_entry(&self, index: u16) -> Option<&T> {
        self.slots.get(usize::from(index)).and_then(Slot::as_ref)
    }
    pub fn as_mut_slice(&mut self) -> &mut [Slot<T>] {
        &mut self.slots
    }
}
