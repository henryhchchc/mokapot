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
pub(crate) struct PoolStorage<S> {
    slots: S,
}

impl<T> PoolStorage<Vec<Slot<T>>> {
    pub fn with_capacity(count: u16) -> Self {
        let mut slots = Vec::with_capacity(usize::from(count.max(1)));
        slots.push(Slot::Padding);
        Self { slots }
    }

    pub fn into_slots(self) -> Vec<Slot<T>> {
        self.slots
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "constructors and insertion bound the slot count to u16"
    )]
    pub const fn count(&self) -> u16 {
        self.slots.len() as u16
    }

    pub fn push(&mut self, entry: T, padding: bool) -> Result<u16, T> {
        if self.slots.len() + 1 + usize::from(padding) > usize::from(u16::MAX) {
            return Err(entry);
        }
        let index = self.count();
        self.slots.push(Slot::Entry(entry));
        if padding {
            self.slots.push(Slot::Padding);
        }
        Ok(index)
    }
}

impl<T> PoolStorage<Box<[Slot<T>]>> {
    pub fn with_padding(count: u16) -> Self {
        Self {
            slots: (0..count.max(1)).map(|_| Slot::Padding).collect(),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "constructor bounds the slot count to u16"
    )]
    pub const fn count(&self) -> u16 {
        self.slots.len() as u16
    }
}

impl<T, S: std::ops::Deref<Target = [Slot<T>]>> PoolStorage<S> {
    pub fn as_slice(&self) -> &[Slot<T>] {
        &self.slots
    }

    pub fn get_entry(&self, index: u16) -> Option<&T> {
        self.slots.get(usize::from(index)).and_then(Slot::as_ref)
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "constructors and insertion bound the slot count to u16"
    )]
    pub fn find(&self, predicate: impl Fn(&T) -> bool) -> Option<(u16, &T)> {
        self.slots.iter().enumerate().find_map(|(index, slot)| {
            slot.as_ref()
                .filter(|entry| predicate(entry))
                .map(|entry| (index as u16, entry))
        })
    }
}

impl<T, S: std::ops::DerefMut<Target = [Slot<T>]>> PoolStorage<S> {
    pub fn as_mut_slice(&mut self) -> &mut [Slot<T>] {
        &mut self.slots
    }
}
