use std::marker::PhantomData;

#[derive(Debug)]
pub(crate) struct IdAllocator<T: NumericalId> {
    next_id: Option<u32>,
    id_kind: PhantomData<T>,
}

pub(crate) trait NumericalId {
    fn from_raw(value: u32) -> Self;
}

impl<T: NumericalId> Default for IdAllocator<T> {
    fn default() -> Self {
        Self {
            next_id: Some(0),
            id_kind: PhantomData,
        }
    }
}

impl<T: NumericalId> IdAllocator<T> {
    pub(crate) fn new_id(&mut self) -> T {
        // Identities are keyed by class-file positions — code offsets, frame slots
        // bounded by `max_locals`/`max_stack`, handlers — so no method exhausts `u32`.
        let id = self.next_id.expect("the id space is exhausted");
        self.next_id = id.checked_add(1);
        T::from_raw(id)
    }
}
