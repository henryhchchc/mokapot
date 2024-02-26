use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    ops::{Bound, Range, RangeInclusive},
};

use crate::{
    jvm::{
        annotation::TypeAnnotation, class::ClassRef, constant_pool::ConstantPool, parsing::Error,
    },
    macros::{malform, see_jvm_spec},
    types::field_type::FieldType,
};

use super::{Instruction, ProgramCounter, RawInstruction};

/// The body of a method.
#[doc = see_jvm_spec!(4, 7, 3)]
#[derive(Debug, Clone)]
pub struct MethodBody {
    /// The maximum number of values on the operand stack of the method.
    pub max_stack: u16,
    /// The maximum number of local variables in the method.
    pub max_locals: u16,
    /// The executable instructions.
    pub instructions: InstructionList<Instruction>,
    /// The exception handlers table.
    pub exception_table: Vec<ExceptionTableEntry>,
    /// The line number table.
    pub line_number_table: Option<Vec<LineNumberTableEntry>>,
    /// The local variable table.
    pub local_variable_table: Option<LocalVariableTable>,
    /// The stack map table.
    pub stack_map_table: Option<Vec<StackMapFrame>>,
    /// The runtime visible type annotations.
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    /// The runtime invisible type annotations.
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    /// Unrecognized JVM attributes.
    pub free_attributes: Vec<(String, Vec<u8>)>,
}

impl MethodBody {
    /// Returns the instruction at the given program counter.
    #[must_use]
    pub fn instruction_at(&self, pc: ProgramCounter) -> Option<&Instruction> {
        self.instructions.get(&pc)
    }
}

/// A list of instructions.
#[derive(Debug, Clone)]
pub struct InstructionList<I>(BTreeMap<ProgramCounter, I>);

impl<I> From<BTreeMap<ProgramCounter, I>> for InstructionList<I> {
    fn from(map: BTreeMap<ProgramCounter, I>) -> Self {
        Self(map)
    }
}

impl<I, const N: usize> From<[(ProgramCounter, I); N]> for InstructionList<I> {
    fn from(value: [(ProgramCounter, I); N]) -> Self {
        Self::from(BTreeMap::from(value))
    }
}

impl<I> IntoIterator for InstructionList<I> {
    type Item = (ProgramCounter, I);

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = <BTreeMap<ProgramCounter, I> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'i, I> IntoIterator for &'i InstructionList<I> {
    type Item = (&'i ProgramCounter, &'i I);

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = <&'i BTreeMap<ProgramCounter, I> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<I> InstructionList<I> {
    /// Creates an iterator over the instructions.
    pub fn iter(&self) -> impl Iterator<Item = (&ProgramCounter, &I)> {
        self.into_iter()
    }
}

impl<I> Display for InstructionList<I>
where
    I: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl<I> InstructionList<I> {
    /// Returns the instruction at the given program counter.
    #[must_use]
    pub fn get(&self, pc: &ProgramCounter) -> Option<&I> {
        self.0.get(pc)
    }

    /// Returns the first instruction in the list.
    #[must_use]
    pub fn entry_point(&self) -> Option<(&ProgramCounter, &I)> {
        self.0.first_key_value()
    }

    /// Returns the program counter of the next instruction after the given one.
    #[must_use]
    pub fn next_pc_of(&self, pc: &ProgramCounter) -> Option<ProgramCounter> {
        self.0
            .range((Bound::Excluded(pc), Bound::Unbounded))
            .next()
            .map(|(k, _)| *k)
    }

    /// Returns the number of instructions in the list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl InstructionList<RawInstruction> {
    /// Lifts an [`InstructionList<RawInstruction>`] to an [`InstructionList<Instruction>`] given the constant pool.
    /// # Errors
    /// See [`Error`] for possible errors.
    pub fn lift(self, constant_pool: &ConstantPool) -> Result<InstructionList<Instruction>, Error> {
        let mut instructions = BTreeMap::new();
        for (pc, raw_instruction) in self {
            let instruction =
                Instruction::from_raw_instruction(raw_instruction, pc, constant_pool)?;
            instructions.insert(pc, instruction);
        }
        Ok(InstructionList(instructions))
    }
}

#[cfg(test)]
mod test {
    use crate::jvm::code::{Instruction, InstructionList};

    use super::MethodBody;
    use Instruction::*;

    #[test]
    fn instruction_at() {
        let body = MethodBody {
            instructions: InstructionList::from([
                (0.into(), Nop),
                (1.into(), IConst0),
                (2.into(), IConst1),
            ]),
            max_stack: 0,
            max_locals: 0,
            exception_table: vec![],
            line_number_table: None,
            local_variable_table: None,
            stack_map_table: None,
            runtime_visible_type_annotations: vec![],
            runtime_invisible_type_annotations: vec![],
            free_attributes: vec![],
        };
        assert_eq!(Some(&IConst0), body.instruction_at(1.into()));
    }
}

/// An entry in the exception table.
#[derive(Debug, Clone)]
pub struct ExceptionTableEntry {
    /// The locations where the exception handler is active.
    pub covered_pc: RangeInclusive<ProgramCounter>,
    /// The location of the exception handler.
    pub handler_pc: ProgramCounter,
    /// The type of the exception to be handled.
    pub catch_type: Option<ClassRef>,
}

impl ExceptionTableEntry {
    /// Checks whether the given program counter is covered by this exception handler.
    #[must_use]
    pub fn covers(&self, pc: ProgramCounter) -> bool {
        self.covered_pc.contains(&pc)
    }
}

/// An entry in the line number table.
#[derive(Debug, Clone)]
pub struct LineNumberTableEntry {
    /// The program counter of the first instruction in the line.
    pub start_pc: ProgramCounter,
    /// The corresponding line number in the source file.
    pub line_number: u16,
}

/// A local variable table.
#[derive(Debug, Clone, Default)]
pub struct LocalVariableTable {
    entries: HashMap<LocalVariableId, LocalVariableTableEntry>,
}

impl LocalVariableTable {
    pub(crate) fn merge_type(
        &mut self,
        key: LocalVariableId,
        name: String,
        field_type: FieldType,
    ) -> Result<(), Error> {
        let entry = self.entries.entry(key).or_default();
        if let Some(existing_name) = entry.name.as_ref() {
            if existing_name != &name {
                malform!("Name of local variable does not match");
            }
        }
        entry.name = Some(name);
        entry.var_type = Some(field_type);
        Ok(())
    }

    pub(crate) fn merge_signature(
        &mut self,
        key: LocalVariableId,
        name: String,
        signature: String,
    ) -> Result<(), Error> {
        let entry = self.entries.entry(key).or_default();
        if let Some(existing_name) = entry.name.as_ref() {
            if existing_name != &name {
                malform!("Name of local variable does not match");
            }
        }
        entry.name = Some(name);
        entry.signature = Some(signature);
        Ok(())
    }
}

/// The identifier of a local variable.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct LocalVariableId {
    /// The location where the variable is valid.
    pub effective_range: Range<ProgramCounter>,
    /// The index in the local variable.
    pub index: u16,
}

/// An entry in the local variable table.
#[derive(Debug, Clone, Default)]
pub struct LocalVariableTableEntry {
    /// The name of the variable.
    pub name: Option<String>,
    /// The type of the variable.
    pub var_type: Option<FieldType>,
    /// The generic signature of the variable.
    pub signature: Option<String>,
}

/// The type of a value in the stack map table for verification.
#[doc = see_jvm_spec!(4, 7, 4)]
#[derive(Debug, Clone)]
pub enum VerificationTypeInfo {
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
    ObjectVariable(ClassRef),
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
#[derive(Debug, Clone)]
pub enum StackMapFrame {
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
        stack: VerificationTypeInfo,
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
        locals: Vec<VerificationTypeInfo>,
    },
    /// Indicates a new frame.
    /// Corresponds to `full_frame`.
    FullFrame {
        /// The offset where the frame applies.
        offset_delta: u16,
        /// The verification information of the local variables.
        locals: Vec<VerificationTypeInfo>,
        /// The verification information of the operand stack.
        stack: Vec<VerificationTypeInfo>,
    },
}
