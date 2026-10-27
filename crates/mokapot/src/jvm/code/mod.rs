//! Module for the APIs for the executable code in JVM.
mod instruction;
mod instruction_list;
pub use instruction_list::{InstructionIter, InstructionList};
mod method_body;
mod pc;

pub(crate) use instruction::InstructionData;
pub use instruction::{Instruction, WideInstruction};
pub use method_body::{
    ExceptionTableEntry, LineNumberTableEntry, LocalVariableId, LocalVariableTable,
    LocalVariableTableEntry, MethodBody, StackMapFrame, VerificationType, VerificationTypeIter,
    VerificationTypes,
};
pub(crate) use method_body::{
    ExceptionTableEntryData, InstructionStorage, LocalVariableTableData,
    LocalVariableTableEntryData, MethodBodyData, StackMapFrameData, VerificationTypeData,
};
pub use pc::{InvalidOffset, ProgramCounter};
