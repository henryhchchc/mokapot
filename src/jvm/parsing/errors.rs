use crate::jvm::{
    code::InvalidOffset, constant_pool::BadConstantPoolIndex, method::InvalidDescriptor,
};

/// An error that occurs when parsing a Java class file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error that occurs when reading from a buffer.
    #[error("Failed to read from buffer: {0}")]
    ReadFail(#[from] std::io::Error),
    /// The format of the class file is invalid.
    #[error("MalformedClassFile: {0}")]
    MalformedClassFile(&'static str),
    /// The constant pool index does not point to a desired entry.
    #[error("Mismatched constant pool entry, expected {expected}, but found {found}")]
    MismatchedConstantPoolEntryType {
        /// The type of the constant pool entry that was expected.
        expected: &'static str,
        /// The type of the constant pool entry that was found.
        found: &'static str,
    },
    /// The constant pool index does not point to an entry.
    #[error("Error when accessing constant pool: {0}")]
    BadConstantPoolIndex(#[from] BadConstantPoolIndex),
    /// The attribute table contains an attribute whose name cannot be recognized.
    #[error("Unknown attribute: {0}")]
    UnknownAttribute(String),
    /// An known attribute is found in an unexpected location.
    #[error("Unexpected attribute {0} in {1}")]
    UnexpectedAttribute(&'static str, &'static str),
    /// There are unread data when the parsing is finished.
    #[error("Unexpected data at the end of the file")]
    UnexpectedData,
    /// The value of an element in an annotation is invalid.
    #[error("Invalid element tag {0}")]
    InvalidElementValueTag(char),
    /// The target type of an annotation is invalid.
    #[error("Invalid target type {0}")]
    InvalidTargetType(u8),
    /// The target type of an annotation is invalid.
    #[error("Invalid type path kind")]
    InvalidTypePathKind,
    /// The stack map frame type is invalid.
    #[error("Unknown stack map frame type {0}")]
    UnknownStackMapFrameType(u8),
    /// The verification type info tag is invalid.
    #[error("Invalid verification type info tag {0}")]
    InvalidVerificationTypeInfoTag(u8),
    /// The opcode cannot be recognized when parsing the code attribute.
    #[error("Unexpected opcode {0:#x}")]
    UnexpectedOpCode(u8),
    /// The flags cannot be recognized.
    #[error("Unknown access flag in {1}: {0:#x}")]
    UnknownFlags(u16, &'static str),
    /// The descriptor is invalid.
    #[error("Fail to parse descriptor: {0}")]
    InvalidDescriptor(#[from] InvalidDescriptor),
    /// The constant pool tag is invalid.
    #[error("Unexpected constant pool tag {0}")]
    UnexpectedConstantPoolTag(u8),
    /// The buffer does not contain a Java class file.
    #[error("The buffer does not contains a Java class file")]
    NotAClassFile,
    /// The jump target is invalid.
    #[error("Invalid jump target: {0}")]
    InvalidJumpTarget(#[from] InvalidOffset),
    /// Tries to reads a string for constructing JVM components (e.g., class name) but got an invalid UTF-8 string.
    #[error("Invalid UTF-8 string")]
    BrokenUTF8,
    /// The instruction list is too long.
    #[error("The instruction list is too long, it should be at most 65536 bytes")]
    TooLongInstructionList,
}
