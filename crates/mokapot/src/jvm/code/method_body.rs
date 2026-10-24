use crate::types::class_name::ClassName;

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
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
        InstructionList::new(self.class, &self.data.instructions)
    }

    /// Returns the exception handlers.
    pub fn exception_table(
        self,
    ) -> impl ExactSizeIterator<Item = ExceptionTableEntry<'class>> + DoubleEndedIterator {
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
    ) -> Option<impl ExactSizeIterator<Item = StackMapFrame<'class>> + DoubleEndedIterator> {
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
    ) -> impl ExactSizeIterator<Item = Attribute<'class>> + DoubleEndedIterator {
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
    use Instruction::*;

    use super::*;
    #[test]
    fn exception_table_entry_covers_half_open_range() {
        let data = ExceptionTableEntryData {
            covered_pc: 1.into()..4.into(),
            handler_pc: 5.into(),
            catch_type: None,
        };
        let class = Class::default();
        let entry = ExceptionTableEntry {
            class: &class,
            data: &data,
        };
        assert!(entry.covers(1.into()));
        assert!(entry.covers(3.into()));
        assert!(!entry.covers(4.into()));
    }

    #[test]
    fn exception_and_stack_map_types_resolve_pool_ids() {
        use crate::types::Descriptor;

        let mut pool = crate::jvm::ConstantPool::new();
        let catch_type = pool.insert_class_name("java/lang/Throwable".parse().unwrap());
        let array_type = pool.insert_reference_type("[I".parse().unwrap());
        let class = Class {
            name: catch_type,
            constant_pool: pool,
            ..Class::default()
        };
        let entry_data = ExceptionTableEntryData {
            covered_pc: 0.into()..1.into(),
            handler_pc: 1.into(),
            catch_type: Some(catch_type),
        };
        let entry = ExceptionTableEntry {
            class: &class,
            data: &entry_data,
        };
        let caught = entry.catch_type().unwrap();
        assert!(std::ptr::eq(
            caught,
            class.constant_pool.class_name(catch_type)
        ));
        assert!(entry.catches_all());
        let array = VerificationType::new(&class, VerificationTypeData::Object(array_type));
        let VerificationType::ObjectVariable(array) = array else {
            panic!("expected an object type");
        };
        assert_eq!(array.descriptor(), "[I");
        assert!(std::ptr::eq(
            array,
            class.constant_pool.reference_type(array_type)
        ));
    }

    #[test]
    fn instruction_at() {
        let data = MethodBodyData {
            instructions: InstructionStorage::from([
                (0.into(), InstructionData::Nop),
                (1.into(), InstructionData::IConst0),
                (2.into(), InstructionData::IConst1),
            ]),
            max_stack: 0,
            max_locals: 0,
            exception_table: Box::new([]),
            line_number_table: None,
            local_variable_table: None,
            stack_map_table: None,
            type_annotations: AnnotationsData::default(),
            other_attributes: Box::new([]),
        };
        let class = Class::default();
        let body = MethodBody::new(&class, &data);
        assert_eq!(Some(IConst0), body.instruction_at(1.into()));
    }

    #[test]
    fn previous_pc() {
        let instruction_list =
            InstructionStorage::from([(0.into(), 10), (1.into(), 20), (2.into(), 30)]);
        assert_eq!(Some(0.into()), instruction_list.prev_pc_of(1.into()));
        assert_eq!(None, instruction_list.prev_pc_of(0.into()));
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
    {
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

/// The verification types of local variables or operand stack entries.
#[derive(Clone, Copy)]
pub struct VerificationTypes<'class> {
    class: &'class Class,
    data: &'class [VerificationTypeData],
}

impl<'class> VerificationTypes<'class> {
    /// Iterates over the verification types.
    pub fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = VerificationType<'class>> + DoubleEndedIterator {
        self.data
            .iter()
            .map(move |data| VerificationType::new(self.class, *data))
    }

    /// Returns the verification type at the given index.
    #[must_use]
    pub fn get(self, index: usize) -> Option<VerificationType<'class>> {
        self.data
            .get(index)
            .map(|data| VerificationType::new(self.class, *data))
    }

    /// Returns the number of verification types.
    #[must_use]
    pub const fn len(self) -> usize {
        self.data.len()
    }

    /// Returns whether the sequence is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }
}

impl fmt::Debug for VerificationTypes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
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
                locals: VerificationTypes {
                    class,
                    data: locals,
                },
            },
            StackMapFrameData::Full {
                offset_delta,
                locals,
                stack,
            } => Self::FullFrame {
                offset_delta: *offset_delta,
                locals: VerificationTypes {
                    class,
                    data: locals,
                },
                stack: VerificationTypes { class, data: stack },
            },
        }
    }
}
