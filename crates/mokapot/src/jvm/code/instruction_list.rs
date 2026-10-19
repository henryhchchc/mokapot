use std::{fmt, ops::RangeBounds, slice};

use super::{Instruction, InstructionData, InstructionStorage, ProgramCounter};
use crate::jvm::Class;

/// Instructions ordered by program counter, borrowing their class.
#[derive(Clone, Copy)]
pub struct InstructionList<'class> {
    class: &'class Class,
    data: &'class InstructionStorage<InstructionData>,
}

impl<'class> InstructionList<'class> {
    pub(crate) const fn new(
        class: &'class Class,
        data: &'class InstructionStorage<InstructionData>,
    ) -> Self {
        Self { class, data }
    }

    /// Iterates over the instructions.
    #[must_use]
    pub fn iter(self) -> InstructionIter<'class> {
        InstructionIter {
            class: self.class,
            inner: self.data.as_slice().iter(),
        }
    }

    /// Iterates over the instructions whose program counters are in `range`.
    ///
    /// # Panics
    /// Panics if the start bound exceeds the end bound, or both bounds exclude the same counter.
    #[must_use]
    pub fn range(self, range: impl RangeBounds<ProgramCounter>) -> InstructionIter<'class> {
        InstructionIter {
            class: self.class,
            inner: self.data.range_slice(range).iter(),
        }
    }

    /// Returns the instruction at the given program counter.
    #[must_use]
    pub fn get(self, pc: &ProgramCounter) -> Option<Instruction<'class>> {
        self.data.get(*pc).map(|data| data.view(self.class))
    }

    /// Returns the first instruction.
    #[must_use]
    pub fn entry_point(self) -> Option<(ProgramCounter, Instruction<'class>)> {
        self.data
            .entry_point()
            .map(|(pc, data)| (pc, data.view(self.class)))
    }

    /// Returns the program counter of the next instruction after the given one.
    #[must_use]
    pub fn next_pc_of(self, pc: &ProgramCounter) -> Option<ProgramCounter> {
        self.data.next_pc_of(*pc)
    }

    /// Returns the program counter of the previous instruction before the given one.
    #[must_use]
    pub fn prev_pc_of(self, pc: &ProgramCounter) -> Option<ProgramCounter> {
        self.data.prev_pc_of(*pc)
    }

    /// Returns the number of instructions.
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

impl fmt::Debug for InstructionList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl fmt::Display for InstructionList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.iter();
        if let Some((pc, instruction)) = iter.next() {
            write!(f, "{pc}: {instruction}")?;
        }
        for (pc, instruction) in iter {
            writeln!(f)?;
            write!(f, "{pc}: {instruction}")?;
        }
        Ok(())
    }
}

impl<'class> IntoIterator for InstructionList<'class> {
    type Item = (ProgramCounter, Instruction<'class>);
    type IntoIter = InstructionIter<'class>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'class> IntoIterator for &InstructionList<'class> {
    type Item = (ProgramCounter, Instruction<'class>);
    type IntoIter = InstructionIter<'class>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An iterator over a method's instructions.
#[derive(Clone)]
pub struct InstructionIter<'class> {
    class: &'class Class,
    inner: slice::Iter<'class, (ProgramCounter, InstructionData)>,
}

impl fmt::Debug for InstructionIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'class> Iterator for InstructionIter<'class> {
    type Item = (ProgramCounter, Instruction<'class>);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(pc, data)| (*pc, data.view(self.class)))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl DoubleEndedIterator for InstructionIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|(pc, data)| (*pc, data.view(self.class)))
    }
}
impl ExactSizeIterator for InstructionIter<'_> {}
impl std::iter::FusedIterator for InstructionIter<'_> {}
