use crate::elements::instruction::InvalidOffset;

#[derive(Debug, thiserror::Error)]
pub enum ClassFileParsingError {
    #[error("Failed to read from buffer: {0}")]
    ReadFail(#[from] std::io::Error),
    #[error("MalformedClassFile: {0}")]
    MalformedClassFile(&'static str),
    #[error("Mismatched constant pool entry, expected {expected}, but found {found}")]
    MismatchedConstantPoolEntryType {
        expected: &'static str,
        found: &'static str,
    },
    #[error("Cannot find entry #{0} in the constant pool")]
    BadConstantPoolIndex(u16),
    #[error("Unknown attribute: {0}")]
    UnknownAttribute(String),
    #[error("Invalid attribute lengeh, expected {expected} but was {actual}")]
    InvalidAttributeLength { expected: u32, actual: u32 },
    #[error("Unexpected attribute {0} in {1}")]
    UnexpectedAttribute(&'static str, &'static str),
    #[error("Unexpected data at the end of the file")]
    UnexpectedData,
    #[error("Invalid element tag {0}")]
    InvalidElementValueTag(char),
    #[error("Invalid target type {0}")]
    InvalidTargetType(u8),
    #[error("Invalid type path kind")]
    InvalidTypePathKind,
    #[error("Unknown stack map frame type {0}")]
    UnknownStackMapFrameType(u8),
    #[error("Invalid verification type info tag {0}")]
    InvalidVerificationTypeInfoTag(u8),
    #[error("Unexpected opcode {0:#x}")]
    UnexpectedOpCode(u8),
    #[error("Unknown access flag in {1}: {0:#x}")]
    UnknownFlags(u16, &'static str),
    #[error("Fail to parse descriptor: {0}")]
    InvalidDescriptor(#[from] InvalidDescriptor),
    #[error("Unexpected constant pool tag {0}")]
    UnexpectedConstantPoolTag(u8),
    #[error("The buffer does not contains a Java class file")]
    NotAClassFile,
    #[error("Invalid jump target: {0}")]
    InvalidJumpTarget(#[from] InvalidOffset),
    #[error("Invalid Cesu8 byte sequence")]
    BrokenCesu8,
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid descriptor: {0}")]
pub struct InvalidDescriptor(pub String);
