use crate::types::class_name::ClassName;

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    iter::FusedIterator,
    ops::{Bound, Range, RangeBounds},
};

use super::{Instruction, InstructionData, InstructionList, ProgramCounter};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        Class, DebugList, JavaStr,
        annotation::{AnnotationsData, TypeAnnotationData, TypeAnnotations},
        attribute::{Attribute, AttributeData},
        constant_pool::{ClassId, FieldTypeId, ReferenceTypeId, Utf8Id},
        view::define_view_list,
    },
    types::{field_type::FieldType, reference_type::ReferenceType},
};

#[derive(Debug, Clone)]
pub(crate) struct MethodBodyData {
    pub max_stack: u16,
    pub max_locals: u16,
    pub instructions: InstructionStorage<InstructionData>,
    pub exception_table: Box<[ExceptionTableEntryData]>,
    pub line_number_table: Option<Box<[LineNumberTableEntry]>>,
    pub local_variable_table: Option<LocalVariableTableData>,
    pub stack_map_table: Option<Box<[StackMapFrameData]>>,
    pub type_annotations: AnnotationsData<TypeAnnotationData>,
    pub other_attributes: Box<[AttributeData]>,
}

/// The body of a method.
#[doc = see_jvm_spec!(4, 7, 3)]
#[derive(Clone, Copy)]
pub struct MethodBody<'class> {
    class: &'class Class,
    data: &'class MethodBodyData,
}

impl<'class> MethodBody<'class> {
    pub(crate) const fn new(class: &'class Class, data: &'class MethodBodyData) -> Self {
        Self { class, data }
    }

    /// Returns the maximum operand stack depth.
    #[must_use]
    pub const fn max_stack(self) -> u16 {
        self.data.max_stack
    }

    /// Returns the maximum number of local variables.
    #[must_use]
    pub const fn max_locals(self) -> u16 {
        self.data.max_locals
    }

    /// Returns the executable instructions.
    #[must_use]
    pub const fn instructions(self) -> InstructionList<'class> {
        InstructionList::new(&self.class.constant_pool, &self.data.instructions)
    }

    /// Returns the exception handlers.
    pub fn exception_table(
        self,
    ) -> impl ExactSizeIterator<Item = ExceptionTableEntry<'class>>
    + DoubleEndedIterator
    + FusedIterator
    + Clone {
        self.data
            .exception_table
            .iter()
            .map(move |data| ExceptionTableEntry {
                class: self.class,
                data,
            })
    }

    /// Returns the line number table.
    #[must_use]
    pub fn line_number_table(self) -> Option<&'class [LineNumberTableEntry]> {
        self.data.line_number_table.as_deref()
    }

    /// Returns the local variable table.
    #[must_use]
    pub fn local_variable_table(self) -> Option<LocalVariableTable<'class>> {
        self.data
            .local_variable_table
            .as_ref()
            .map(|data| LocalVariableTable {
                class: self.class,
                data,
            })
    }

    /// Returns the stack map table.
    #[must_use]
    pub fn stack_map_table(
        self,
    ) -> Option<
        impl ExactSizeIterator<Item = StackMapFrame<'class>>
        + DoubleEndedIterator
        + FusedIterator
        + Clone,
    > {
        self.data.stack_map_table.as_ref().map(|table| {
            table
                .iter()
                .map(move |data| StackMapFrame::new(self.class, data))
        })
    }

    /// Returns the type annotations.
    #[must_use]
    pub const fn type_annotations(self) -> TypeAnnotations<'class> {
        TypeAnnotations::new(self.class, &self.data.type_annotations)
    }

    /// Returns the unrecognized attributes.
    pub fn other_attributes(
        self,
    ) -> impl ExactSizeIterator<Item = Attribute<'class>> + DoubleEndedIterator + FusedIterator + Clone
    {
        self.data
            .other_attributes
            .iter()
            .map(move |data| Attribute::new(self.class, data))
    }

    /// Returns the instruction at the given program counter.
    #[must_use]
    pub fn instruction_at(self, pc: ProgramCounter) -> Option<Instruction<'class>> {
        self.instructions().get(&pc)
    }
}

impl fmt::Debug for MethodBody<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MethodBody")
            .field("max_stack", &self.max_stack())
            .field("max_locals", &self.max_locals())
            .field("instructions", &self.instructions())
            .field(
                "exception_table",
                &DebugList::new(|| self.exception_table()),
            )
            .field("line_number_table", &self.line_number_table())
            .field("local_variable_table", &self.local_variable_table())
            .field(
                "stack_map_table",
                &self
                    .stack_map_table()
                    .map(|_| DebugList::new(|| self.stack_map_table().into_iter().flatten())),
            )
            .field("type_annotations", &self.type_annotations())
            .field(
                "other_attributes",
                &DebugList::new(|| self.other_attributes()),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InstructionStorage<I>(Vec<(ProgramCounter, I)>);

impl<I, T> From<T> for InstructionStorage<I>
where
    T: Into<BTreeMap<ProgramCounter, I>>,
{
    fn from(value: T) -> Self {
        Self(value.into().into_iter().collect())
    }
}

impl<I> IntoIterator for InstructionStorage<I> {
    type Item = (ProgramCounter, I);

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = std::vec::IntoIter<(ProgramCounter, I)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<I> InstructionStorage<I> {
    pub(crate) fn as_slice(&self) -> &[(ProgramCounter, I)] {
        &self.0
    }
    pub(crate) fn from_ordered(instructions: Vec<(ProgramCounter, I)>) -> Self {
        debug_assert!(instructions.windows(2).all(|pair| pair[0].0 < pair[1].0));
        Self(instructions)
    }

    pub(crate) fn range_slice(
        &self,
        range: impl RangeBounds<ProgramCounter>,
    ) -> &[(ProgramCounter, I)] {
        match (range.start_bound(), range.end_bound()) {
            (Bound::Excluded(start), Bound::Excluded(end)) => {
                assert!(
                    start < end,
                    "Excluded range bounds must be strictly increasing"
                );
            }
            (
                Bound::Included(start) | Bound::Excluded(start),
                Bound::Included(end) | Bound::Excluded(end),
            ) => assert!(start <= end, "Range bounds must be increasing"),
            _ => {}
        }
        let start = match range.start_bound() {
            Bound::Included(pc) => self.0.partition_point(|(key, _)| key < pc),
            Bound::Excluded(pc) => self.0.partition_point(|(key, _)| key <= pc),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(pc) => self.0.partition_point(|(key, _)| key <= pc),
            Bound::Excluded(pc) => self.0.partition_point(|(key, _)| key < pc),
            Bound::Unbounded => self.0.len(),
        };
        &self.0[start..end]
    }
}

impl<I> FromIterator<(ProgramCounter, I)> for InstructionStorage<I> {
    fn from_iter<T: IntoIterator<Item = (ProgramCounter, I)>>(iter: T) -> Self {
        let mut instructions: Vec<_> = iter.into_iter().collect();
        instructions.sort_by_key(|(pc, _)| *pc);
        instructions.dedup_by(|next, previous| {
            if next.0 == previous.0 {
                std::mem::swap(&mut next.1, &mut previous.1);
                true
            } else {
                false
            }
        });
        Self(instructions)
    }
}

impl<I> InstructionStorage<I> {
    /// Returns the instruction at the given program counter.
    #[must_use]
    pub fn get(&self, pc: ProgramCounter) -> Option<&I> {
        self.0
            .binary_search_by_key(&pc, |(pc, _)| *pc)
            .ok()
            .map(|index| &self.0[index].1)
    }

    /// Returns the first instruction in the list.
    #[must_use]
    pub fn entry_point(&self) -> Option<(ProgramCounter, &I)> {
        self.0.first().map(|(pc, insn)| (*pc, insn))
    }

    /// Returns the program counter of the next instruction after the given one.
    #[must_use]
    pub fn next_pc_of(&self, pc: ProgramCounter) -> Option<ProgramCounter> {
        let index = self.0.partition_point(|(key, _)| *key <= pc);
        self.0.get(index).map(|(pc, _)| *pc)
    }

    /// Returns the program counter of the previous instruction before the given one.
    #[must_use]
    pub fn prev_pc_of(&self, pc: ProgramCounter) -> Option<ProgramCounter> {
        let index = self.0.partition_point(|(key, _)| *key < pc);
        index.checked_sub(1).map(|index| self.0[index].0)
    }

    /// Returns the number of instructions in the list.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the list is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn catch_all_handlers_include_implicit_and_throwable_types() {
        let mut class = Class::default();
        let throwable = class
            .constant_pool
            .insert_class_name("java/lang/Throwable".parse().unwrap());
        let exception = class
            .constant_pool
            .insert_class_name("java/lang/Exception".parse().unwrap());
        for (catch_type, catches_all) in [
            (None, true),
            (Some(throwable), true),
            (Some(exception), false),
        ] {
            let data = ExceptionTableEntryData {
                covered_pc: 0.into()..1.into(),
                handler_pc: 1.into(),
                catch_type,
            };
            let entry = ExceptionTableEntry {
                class: &class,
                data: &data,
            };
            assert_eq!(entry.catches_all(), catches_all);
        }
    }

    #[test]
    fn instruction_storage_orders_sparse_offsets_and_finds_neighbors() {
        let instructions = InstructionStorage::from_iter([
            (5.into(), 50),
            (0.into(), 0),
            (2.into(), 20),
            (2.into(), 21),
        ]);
        assert_eq!(instructions.get(2.into()), Some(&21));
        assert_eq!(instructions.get(3.into()), None);
        assert_eq!(instructions.prev_pc_of(3.into()), Some(2.into()));
        assert_eq!(instructions.next_pc_of(3.into()), Some(5.into()));
        assert_eq!(instructions.prev_pc_of(0.into()), None);
        assert_eq!(instructions.next_pc_of(5.into()), None);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExceptionTableEntryData {
    pub covered_pc: Range<ProgramCounter>,
    pub handler_pc: ProgramCounter,
    pub catch_type: Option<ClassId>,
}

/// An entry in the exception table.
#[doc = see_jvm_spec!(4, 7, 3)]
#[derive(Clone, Copy)]
pub struct ExceptionTableEntry<'class> {
    class: &'class Class,
    data: &'class ExceptionTableEntryData,
}

impl<'class> ExceptionTableEntry<'class> {
    /// The half-open range where the exception handler is active.
    #[must_use]
    pub const fn covered_pc(self) -> &'class Range<ProgramCounter> {
        &self.data.covered_pc
    }

    /// The location of the exception handler.
    #[must_use]
    pub const fn handler_pc(self) -> ProgramCounter {
        self.data.handler_pc
    }

    /// The type of the exception to be handled.
    #[must_use]
    pub fn catch_type(self) -> Option<&'class ClassName> {
        self.data
            .catch_type
            .map(|id| self.class.constant_pool.class_name(id))
    }

    /// Checks whether the given program counter is covered by this exception handler.
    #[must_use]
    pub fn covers(self, pc: ProgramCounter) -> bool {
        self.covered_pc().contains(&pc)
    }

    /// Returns whether this exception handler catches all exceptions.
    #[must_use]
    pub fn catches_all(self) -> bool {
        self.catch_type()
            .is_none_or(|caught| caught.as_java_str() == "java/lang/Throwable")
    }
}

impl fmt::Debug for ExceptionTableEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExceptionTableEntry")
            .field("covered_pc", &self.covered_pc())
            .field("handler_pc", &self.handler_pc())
            .field("catch_type", &self.catch_type())
            .finish()
    }
}

/// An entry in the line number table.
#[doc = see_jvm_spec!(4, 7, 12)]
#[derive(Debug, Clone)]
pub struct LineNumberTableEntry {
    /// The program counter of the first instruction in the line.
    pub start_pc: ProgramCounter,
    /// The corresponding line number in the source file.
    pub line_number: u16,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LocalVariableTableData {
    entries: HashMap<LocalVariableId, LocalVariableTableEntryData>,
}

impl LocalVariableTableData {
    pub(crate) fn entry_data_mut(
        &mut self,
        key: LocalVariableId,
    ) -> &mut LocalVariableTableEntryData {
        self.entries.entry(key).or_default()
    }
}

/// A local variable table.
#[doc = see_jvm_spec!(4, 7, 13)]
#[derive(Clone, Copy)]
pub struct LocalVariableTable<'class> {
    class: &'class Class,
    data: &'class LocalVariableTableData,
}

impl<'class> LocalVariableTable<'class> {
    /// Iterates over the local variable identifiers and their information.
    pub fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = (&'class LocalVariableId, LocalVariableTableEntry<'class>)>
    + FusedIterator
    + Clone {
        self.data.entries.iter().map(move |(id, data)| {
            (
                id,
                LocalVariableTableEntry {
                    class: self.class,
                    data,
                },
            )
        })
    }

    /// Returns the information for a local variable.
    #[must_use]
    pub fn get(self, id: &LocalVariableId) -> Option<LocalVariableTableEntry<'class>> {
        self.data
            .entries
            .get(id)
            .map(|data| LocalVariableTableEntry {
                class: self.class,
                data,
            })
    }

    /// Returns the number of local variables.
    #[must_use]
    pub fn len(self) -> usize {
        self.data.entries.len()
    }

    /// Returns whether the table is empty.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.data.entries.is_empty()
    }
}

impl fmt::Debug for LocalVariableTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// The identifier of a local variable.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct LocalVariableId {
    /// The location where the variable is valid.
    pub range: Range<ProgramCounter>,
    /// The index in the local variable.
    pub index: u16,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LocalVariableTableEntryData {
    pub name: Option<Utf8Id>,
    pub var_type: Option<FieldTypeId>,
    pub signature: Option<Utf8Id>,
}

/// An entry in the local variable table.
#[doc = see_jvm_spec!(4, 7, 13)]
#[derive(Clone, Copy)]
pub struct LocalVariableTableEntry<'class> {
    class: &'class Class,
    data: &'class LocalVariableTableEntryData,
}

impl<'class> LocalVariableTableEntry<'class> {
    /// The name of the variable.
    #[must_use]
    pub fn name(self) -> Option<&'class JavaStr> {
        self.data.name.map(|id| self.class.constant_pool.utf8(id))
    }

    /// The type of the variable.
    #[must_use]
    pub fn var_type(self) -> Option<&'class FieldType> {
        self.data
            .var_type
            .map(|id| self.class.constant_pool.field_type(id))
    }

    /// The generic signature of the variable.
    #[must_use]
    pub fn signature(self) -> Option<&'class JavaStr> {
        self.data
            .signature
            .map(|id| self.class.constant_pool.utf8(id))
    }
}

impl fmt::Debug for LocalVariableTableEntry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalVariableTableEntry")
            .field("name", &self.name())
            .field("var_type", &self.var_type())
            .field("signature", &self.signature())
            .finish()
    }
}

/// The type of a value in the stack map table for verification.
#[doc = see_jvm_spec!(4, 7, 4)]
#[derive(Debug, Clone, Copy)]
pub enum VerificationType<'class> {
    /// Indicates that the local variable has the verification type `top`.
    TopVariable,
    /// Indicates that the local variable has the verification type `int`.
    IntegerVariable,
    /// Indicates that the local variable has the verification type `float`.
    FloatVariable,
    /// Indicates that the local variable has the verification type `null`.
    NullVariable,
    /// Indicates that the local variable has the verification type `uninitializedThis`.
    UninitializedThisVariable,
    /// Indicates that the local variable has the verification type `object` with the given type
    ObjectVariable(&'class ReferenceType),
    /// Indicates that the local variable has the verification type `uninitialized` with the given offset.
    UninitializedVariable {
        /// The location of the [`Instruction::New`] that created the object.
        offset: ProgramCounter,
    },
    /// Indicates that the local variable has the verification type `long`.
    LongVariable,
    /// Indicates that the local variable has the verification type `double`.
    DoubleVariable,
}

/// A stack map frame for verification.
#[doc = see_jvm_spec!(4, 7, 4)]
#[derive(Debug, Clone, Copy)]
pub enum StackMapFrame<'class> {
    /// Indicates that the frame has exactly the same locals as the previous frame and that the operand stack is empty.
    /// Corresponds to the `same_frame` and `same_frame_extended`.
    SameFrame {
        /// The offset where the frame applies.
        offset_delta: u16,
    },
    /// Indicates that the frame has exactly the same locals as the previous frame and that the operand stack has one entry.
    /// Corresponds to the `same_locals_1_stack_item_frame` and `same_locals_1_stack_item_frame_extended`.
    SameLocals1StackItemFrame {
        /// The offset where the frame applies.
        offset_delta: u16,
        /// The type of the one entry in the operand stack.
        stack: VerificationType<'class>,
    },
    /// Indicates that the frame has the same local variables as the previous frame except that the last few local
    /// variables are absent, and that the operand stack is empty.
    /// Corresponds to `chop_frame`.
    ChopFrame {
        /// The offset where the frame applies.
        offset_delta: u16,
        /// The number of local variables that are absent.
        chop_count: u8,
    },
    /// Indicates that the frame has the same locals as the previous frame except that k additional locals are defined,
    /// and that the operand stack is empty.
    /// Corresponds to `append_frame`.
    AppendFrame {
        /// The offset where the frame applies.
        offset_delta: u16,
        /// The verification information of additional local variables.
        locals: VerificationTypes<'class>,
    },
    /// Indicates a new frame.
    /// Corresponds to `full_frame`.
    FullFrame {
        /// The offset where the frame applies.
        offset_delta: u16,
        /// The verification information of the local variables.
        locals: VerificationTypes<'class>,
        /// The verification information of the operand stack.
        stack: VerificationTypes<'class>,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum VerificationTypeData {
    Top,
    Integer,
    Float,
    Null,
    UninitializedThis,
    Object(ReferenceTypeId),
    Uninitialized { offset: ProgramCounter },
    Long,
    Double,
}

impl<'class> VerificationType<'class> {
    fn new(class: &'class Class, data: VerificationTypeData) -> Self {
        match data {
            VerificationTypeData::Top => Self::TopVariable,
            VerificationTypeData::Integer => Self::IntegerVariable,
            VerificationTypeData::Float => Self::FloatVariable,
            VerificationTypeData::Null => Self::NullVariable,
            VerificationTypeData::UninitializedThis => Self::UninitializedThisVariable,
            VerificationTypeData::Object(id) => {
                Self::ObjectVariable(class.constant_pool.reference_type(id))
            }
            VerificationTypeData::Uninitialized { offset } => {
                Self::UninitializedVariable { offset }
            }
            VerificationTypeData::Long => Self::LongVariable,
            VerificationTypeData::Double => Self::DoubleVariable,
        }
    }
}

define_view_list! {
    /// The verification types of local variables or operand stack entries.
    pub struct VerificationTypes<'class>;
    iterator VerificationTypeIter;
    VerificationTypeData => VerificationType<'class>;
    |class, data| VerificationType::new(class, *data)
}

#[derive(Debug, Clone)]
pub(crate) enum StackMapFrameData {
    Same {
        offset_delta: u16,
    },
    SameLocals1StackItem {
        offset_delta: u16,
        stack: VerificationTypeData,
    },
    Chop {
        offset_delta: u16,
        chop_count: u8,
    },
    Append {
        offset_delta: u16,
        locals: Box<[VerificationTypeData]>,
    },
    Full {
        offset_delta: u16,
        locals: Box<[VerificationTypeData]>,
        stack: Box<[VerificationTypeData]>,
    },
}

impl<'class> StackMapFrame<'class> {
    fn new(class: &'class Class, data: &'class StackMapFrameData) -> Self {
        match data {
            StackMapFrameData::Same { offset_delta } => Self::SameFrame {
                offset_delta: *offset_delta,
            },
            StackMapFrameData::SameLocals1StackItem {
                offset_delta,
                stack,
            } => Self::SameLocals1StackItemFrame {
                offset_delta: *offset_delta,
                stack: VerificationType::new(class, *stack),
            },
            StackMapFrameData::Chop {
                offset_delta,
                chop_count,
            } => Self::ChopFrame {
                offset_delta: *offset_delta,
                chop_count: *chop_count,
            },
            StackMapFrameData::Append {
                offset_delta,
                locals,
            } => Self::AppendFrame {
                offset_delta: *offset_delta,
                locals: VerificationTypes::new(class, locals),
            },
            StackMapFrameData::Full {
                offset_delta,
                locals,
                stack,
            } => Self::FullFrame {
                offset_delta: *offset_delta,
                locals: VerificationTypes::new(class, locals),
                stack: VerificationTypes::new(class, stack),
            },
        }
    }
}
