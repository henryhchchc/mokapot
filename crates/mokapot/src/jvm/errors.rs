//! Error handling for JVM bytecode parsing and generation.
//!
//! This module defines error types and utilities for handling errors that
//! can occur during parsing JVM class files and generating JVM bytecode.
//!
//! The main error types are:
//! - [`ParseError`] - Errors that occur during parsing of class files
//! - [`GenerationError`] - Errors that occur during bytecode generation
//!
//! Both are type aliases for [`BytecodeError`] parameterized by their
//! respective [`ParseErrorKind`] or [`GenerationErrorKind`].
//!
//! Additionally, this module provides the [`ParsingErrorContext`] trait, which
//! allows for more context to be added to errors during parsing.

use std::{
    error::Error,
    fmt::{self, Display},
    io,
    num::TryFromIntError,
};

#[cfg(debug_assertions)]
use std::backtrace::Backtrace;

use crate::jvm::{class::constant_pool, code::InvalidOffset};

/// A generic JVM bytecode error parameterized by its error kind.
///
/// Both [`ParseError`] and [`GenerationError`] are type aliases for this
/// struct with different kind enums.
#[derive(Debug)]
pub struct BytecodeError<K> {
    cause: Box<dyn Error + Send + Sync>,
    kind: K,
    #[cfg(debug_assertions)]
    backtrace: Backtrace,
}

impl<K> BytecodeError<K> {
    /// Creates a new `BytecodeError` with the given cause and kind.
    #[must_use]
    pub fn new(cause: Box<dyn Error + Send + Sync>, kind: K) -> Self {
        Self {
            cause,
            kind,
            #[cfg(debug_assertions)]
            backtrace: Backtrace::capture(),
        }
    }

    /// Returns the kind of error.
    #[must_use]
    pub const fn kind(&self) -> K
    where
        K: Copy,
    {
        self.kind
    }
}

impl<K: Display> Display for BytecodeError<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{kind}: {cause}", kind = self.kind, cause = self.cause)?;
        #[cfg(debug_assertions)]
        {
            write!(f, "\nBacktrace: \n{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl<K: fmt::Debug + Display + Send + Sync + 'static> Error for BytecodeError<K> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.cause.as_ref())
    }
}

// ---------------------------------------------------------------------------
// ParseError
// ---------------------------------------------------------------------------

/// An error that occurs during parsing of a class file.
pub type ParseError = BytecodeError<ParseErrorKind>;

impl BytecodeError<ParseErrorKind> {
    /// Creates a new `ParseError` with the given message, indicating a malformed class file.
    pub(crate) fn malform(message: impl fmt::Display) -> Self {
        Self::new(format!("{message}").into(), ParseErrorKind::Malformed)
    }

    /// Creates a new `ParseError` with the kind `IO` and the given `std::io::Error` as its cause.
    pub(crate) fn io(error: io::Error) -> Self {
        Self::new(error.into(), ParseErrorKind::IO)
    }
}

impl From<std::io::Error> for BytecodeError<ParseErrorKind> {
    fn from(error: std::io::Error) -> Self {
        Self::io(error)
    }
}

/// The kind of [`ParseError`].
///
/// This enum represents the different categories of errors that can occur
/// during parsing of a class file.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// An error occurred while reading from the underlying input source.
    IO,
    /// The class file is malformed and does not conform to the JVM specification.
    Malformed,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO => write!(f, "IO Error"),
            Self::Malformed => write!(f, "Malformed class file"),
        }
    }
}

// ---------------------------------------------------------------------------
// ParsingErrorContext
// ---------------------------------------------------------------------------

/// A trait for providing context to errors during parsing.
///
/// This trait allows adding contextual information to errors that occur during
/// parsing, making it easier to understand what went wrong. It is implemented for
/// `Result` and `Option` types to provide a convenient way to handle errors.
pub(crate) trait ParsingErrorContext {
    /// The success type.
    type Output;

    /// The error type.
    type Error;

    /// Adds static context to an error.
    ///
    /// This method adds a static message to an error. The message is attached
    /// to the error regardless of the error's value.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to add to the error.
    fn context<Message>(self, message: Message) -> Result<Self::Output, ParseError>
    where
        Message: fmt::Display;

    /// Adds dynamic context to an error.
    ///
    /// This method adds a message to an error that is generated based on the error value.
    /// This allows for more specific error messages that depend on the error value.
    ///
    /// # Arguments
    ///
    /// * `message_fn` - A function that takes the error value and returns a message.
    fn with_context<F, Message>(self, message_fn: F) -> Result<Self::Output, ParseError>
    where
        F: FnOnce(Self::Error) -> Message,
        Message: fmt::Display;
}

impl<T, E> ParsingErrorContext for Result<T, E>
where
    E: fmt::Display,
{
    type Output = T;
    type Error = E;

    fn context<Message>(self, message: Message) -> Result<Self::Output, ParseError>
    where
        Message: fmt::Display,
    {
        self.with_context(|err| format!("{message}: {err}"))
    }

    fn with_context<F, Message>(self, message_fn: F) -> Result<Self::Output, ParseError>
    where
        F: FnOnce(Self::Error) -> Message,
        Message: fmt::Display,
    {
        self.map_err(|err| ParseError::malform(message_fn(err)))
    }
}

impl<T> ParsingErrorContext for Option<T> {
    type Output = T;
    type Error = ();

    fn context<Message>(self, message: Message) -> Result<Self::Output, ParseError>
    where
        Message: fmt::Display,
    {
        self.with_context(|()| message)
    }

    fn with_context<F, Message>(self, message_fn: F) -> Result<Self::Output, ParseError>
    where
        F: FnOnce(Self::Error) -> Message,
        Message: fmt::Display,
    {
        self.ok_or_else(|| {
            let message = message_fn(());
            ParseError::malform(message)
        })
    }
}

// ---------------------------------------------------------------------------
// GenerationError
// ---------------------------------------------------------------------------

/// An error that occurs during bytecode generation.
#[instability::unstable(feature = "bytecode-generation")]
pub type GenerationError = BytecodeError<GenerationErrorKind>;

impl BytecodeError<GenerationErrorKind> {
    /// Creates a new `GenerationError` with the given message and kind `Other`.
    #[must_use]
    pub fn other<Message>(message: Message) -> Self
    where
        Message: Display,
    {
        Self::new(format!("{message}").into(), GenerationErrorKind::Other)
    }

    /// Creates a new `GenerationError` with the kind `IO` and the given `std::io::Error` as its cause.
    #[must_use]
    pub fn io(error: io::Error) -> Self {
        Self::new(error.into(), GenerationErrorKind::IO)
    }
}

impl From<io::Error> for BytecodeError<GenerationErrorKind> {
    fn from(error: io::Error) -> Self {
        Self::io(error)
    }
}

impl From<InvalidOffset> for BytecodeError<GenerationErrorKind> {
    fn from(cause: InvalidOffset) -> Self {
        Self::new(cause.into(), GenerationErrorKind::OutOfRange)
    }
}

impl From<constant_pool::Overflow> for BytecodeError<GenerationErrorKind> {
    fn from(cause: constant_pool::Overflow) -> Self {
        Self::new(cause.into(), GenerationErrorKind::ConstantPool)
    }
}

impl From<TryFromIntError> for BytecodeError<GenerationErrorKind> {
    fn from(cause: TryFromIntError) -> Self {
        Self::new(cause.into(), GenerationErrorKind::OutOfRange)
    }
}

/// The kind of [`GenerationError`].
///
/// This enum represents the different categories of errors that can occur
/// during bytecode generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
#[instability::unstable(feature = "bytecode-generation")]
pub enum GenerationErrorKind {
    /// An error occurred while writing to the underlying output destination.
    IO,
    /// A value exceeds the maximum allowed by the JVM specification.
    ///
    /// For example, an instruction list containing more than 65535 instructions,
    /// which exceeds the maximum number that can be represented in a 16-bit field.
    OutOfRange,
    /// An error occurred when operating on the constant pool.
    ///
    /// This could be due to reaching the maximum number of entries
    /// or trying to add an invalid entry.
    ConstantPool,
    /// Other errors that don't fall into the above categories.
    Other,
}

impl Display for GenerationErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IO => write!(f, "IO Error"),
            Self::OutOfRange => write!(f, "Out of range error"),
            Self::ConstantPool => write!(f, "Constant pool error"),
            Self::Other => write!(f, "Other error"),
        }
    }
}
