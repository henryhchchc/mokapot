//! Error handling for JVM class-file parsing.
//!
//! [`ParseError`] is a [`BytecodeError`] parameterized by [`ParseErrorKind`].
//! Parsing errors can retain context describing where a failure occurred.

use derive_more::Display;

#[cfg(debug_assertions)]
use std::backtrace::Backtrace;
use std::{
    error::Error,
    fmt::{self, Display},
};

/// A generic JVM bytecode error parameterized by its error kind.
///
/// [`ParseError`] specializes this struct with [`ParseErrorKind`].
#[derive(Debug)]
pub struct BytecodeError<K> {
    cause: Option<Box<dyn Error + Send + Sync>>,
    kind: K,
    #[cfg(debug_assertions)]
    backtrace: Backtrace,
}

impl<K> BytecodeError<K> {
    /// Creates a new `BytecodeError` with the given cause and kind.
    #[must_use]
    pub fn new(cause: Box<dyn Error + Send + Sync>, kind: K) -> Self {
        Self {
            cause: Some(cause),
            ..Self::from_kind(kind)
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

    pub(crate) fn add_context(mut self, message: impl fmt::Display) -> Self {
        self.cause = Some(Box::new(ContextCause {
            message: message.to_string(),
            source: self.cause.take(),
        }));
        self
    }
    /// Creates an error described entirely by its kind.
    #[must_use]
    pub fn from_kind(kind: K) -> Self {
        Self {
            cause: None,
            kind,
            #[cfg(debug_assertions)]
            backtrace: Backtrace::capture(),
        }
    }
}

impl<K: Display> Display for BytecodeError<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.kind, f)?;
        if let Some(cause) = &self.cause {
            write!(f, ": {cause}")?;
        }
        #[cfg(debug_assertions)]
        {
            write!(f, "\nBacktrace: \n{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl<K: fmt::Debug + Display + Send + Sync + 'static> Error for BytecodeError<K> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_deref().map(|cause| cause as &dyn Error)
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

    pub(crate) fn truncated(requested: usize, available: usize) -> Self {
        Self::from_kind(ParseErrorKind::Truncated {
            requested,
            available,
        })
    }
}

#[derive(Debug)]
struct ContextCause {
    message: String,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl Display for ContextCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl Error for ContextCause {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref().map(|source| source as &dyn Error)
    }
}

/// The kind of [`ParseError`].
///
/// This enum represents the different categories of errors that can occur
/// during parsing of a class file.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Display)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// The class file is malformed and does not conform to the JVM specification.
    #[display("Malformed class file")]
    Malformed,
    /// A constant-pool reference points outside the pool or to a reserved slot.
    #[display("Invalid constant pool index {index}")]
    InvalidConstantPoolIndex {
        /// The referenced index.
        index: u16,
    },
    /// A constant-pool entry has an unexpected type or value category.
    #[display("Unexpected constant pool entry at {index}: expected {expected}, got {actual}")]
    UnexpectedConstantPoolEntry {
        /// The referenced index.
        index: u16,
        /// The required entry category.
        expected: &'static str,
        /// The actual JVM constant kind.
        actual: &'static str,
    },
    /// An instruction has an invalid opcode (including an invalid wide opcode).
    #[display("Invalid opcode {opcode:#04x} at instruction {pc}")]
    InvalidOpcode {
        /// The invalid byte.
        opcode: u8,
        /// The offset of the instruction in the code array.
        pc: u16,
    },
    /// The class-file version is newer than this parser supports.
    #[display("Unsupported class-file version {major}.{minor}")]
    UnsupportedVersion {
        /// The major version.
        major: u16,
        /// The minor version.
        minor: u16,
    },
    /// The input ended before a value could be read.
    #[display("Truncated input: requested {requested} bytes, available {available}")]
    Truncated {
        /// Bytes needed for the current value.
        requested: usize,
        /// Bytes remaining in the current input.
        available: usize,
    },
}

// ---------------------------------------------------------------------------
// ParsingErrorContext
// ---------------------------------------------------------------------------

/// Converts missing values and external errors into malformed-class errors.
/// Use [`BytecodeError::add_context`] for an existing [`ParseError`] to retain its kind.
pub(crate) trait ParsingErrorContext {
    type Output;

    fn context(self, message: impl fmt::Display) -> Result<Self::Output, ParseError>;
}

impl<T, E: Error + Send + Sync + 'static> ParsingErrorContext for Result<T, E> {
    type Output = T;

    fn context(self, message: impl fmt::Display) -> Result<T, ParseError> {
        self.map_err(|error| {
            ParseError::new(Box::new(error), ParseErrorKind::Malformed).add_context(message)
        })
    }
}

impl<T> ParsingErrorContext for Option<T> {
    type Output = T;

    fn context(self, message: impl fmt::Display) -> Result<T, ParseError> {
        self.ok_or_else(|| ParseError::malform(message))
    }
}
