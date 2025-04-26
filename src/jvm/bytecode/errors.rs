use std::{
    backtrace::Backtrace,
    error::Error,
    fmt::{self, Display},
    io,
    num::TryFromIntError,
};

use crate::jvm::{class::constant_pool, code::InvalidOffset};

/// An error that occurs during parsing of a class file.
#[derive(Debug)]
pub struct ParseError {
    cause: Box<dyn Error + Send + Sync>,
    kind: ParsingErrorKind,
    #[cfg(debug_assertions)]
    backtrace: Backtrace,
}

impl Error for ParseError {
    fn cause(&self) -> Option<&dyn Error> {
        Some(self.cause.as_ref())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ParsingErrorKind::IO => write!(f, "IO Error: {}", self.cause)?,
            ParsingErrorKind::Malformed => write!(f, "Malformed class file: {}", self.cause)?,
        }
        if cfg!(debug_assertions) {
            write!(f, "Backtrace: \n{}", self.backtrace)?;
        }
        Ok(())
    }
}

impl ParseError {
    pub(crate) fn malform(message: impl fmt::Display) -> Self {
        Self {
            cause: format!("{message}").into(),
            kind: ParsingErrorKind::Malformed,
            #[cfg(debug_assertions)]
            backtrace: Backtrace::capture(),
        }
    }

    /// Returns the kind of error.
    #[must_use]
    pub const fn kind(&self) -> ParsingErrorKind {
        self.kind
    }
}

impl From<std::io::Error> for ParseError {
    fn from(value: std::io::Error) -> Self {
        Self {
            cause: value.into(),
            kind: ParsingErrorKind::IO,
            #[cfg(debug_assertions)]
            backtrace: Backtrace::capture(),
        }
    }
}

/// The Kind of [`ParsingError`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ParsingErrorKind {
    /// Due to an IO error in the underlying reader
    IO,
    /// Due to a malformed class file
    Malformed,
}

pub(crate) trait ParsingErrorContext {
    type Output;
    type Error;

    fn context<Message>(self, message: Message) -> Result<Self::Output, ParseError>
    where
        Message: fmt::Display;

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

/// An error that occurs during parsing of a class file.
#[derive(Debug)]
pub struct GenerationError {
    cause: Box<dyn Error + Send + Sync>,
    kind: GenerationErrorKind,
    #[cfg(debug_assertions)]
    backtrace: Backtrace,
}

impl Error for GenerationError {
    fn cause(&self) -> Option<&dyn Error> {
        Some(self.cause.as_ref())
    }
}

impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            GenerationErrorKind::IO => write!(f, "IO Error: {}", self.cause)?,
            GenerationErrorKind::OutOfRange => write!(f, "Out of range error: {}", self.cause)?,
            GenerationErrorKind::ConstantPool => write!(f, "Constant pool error: {}", self.cause)?,
            GenerationErrorKind::Other => write!(f, "Other error: {}", self.cause)?,
        }
        if cfg!(debug_assertions) {
            write!(f, "Backtrace: \n{}", self.backtrace)?;
        }
        Ok(())
    }
}

/// The kind of [`GenerationError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationErrorKind {
    /// Due to an IO error in the underlying writer
    IO,
    /// The length of a list if beyond the max value of the data type to store the range. For instance an instruction list containing more than 65535 instructions.
    OutOfRange,
    /// An error when operating the constant pool
    ConstantPool,
    /// Other errors
    Other,
}

impl GenerationError {
    /// Creates a new `GenerationError` with the given cause and kind.
    #[must_use]
    pub fn new(cause: Box<dyn Error + Send + Sync>, kind: GenerationErrorKind) -> Self {
        Self {
            cause,
            kind,
            #[cfg(debug_assertions)]
            backtrace: Backtrace::capture(),
        }
    }

    /// Creates a new `GenerationError` with the given message and kind.
    #[must_use]
    pub fn other<Message>(message: Message) -> Self
    where
        Message: Display,
    {
        Self::new(format!("{message}").into(), GenerationErrorKind::Other)
    }
}

impl From<io::Error> for GenerationError {
    fn from(cause: io::Error) -> Self {
        Self::new(cause.into(), GenerationErrorKind::IO)
    }
}

impl From<InvalidOffset> for GenerationError {
    fn from(cause: InvalidOffset) -> Self {
        Self::new(cause.into(), GenerationErrorKind::OutOfRange)
    }
}

impl From<constant_pool::Overflow> for GenerationError {
    fn from(cause: constant_pool::Overflow) -> Self {
        Self::new(cause.into(), GenerationErrorKind::ConstantPool)
    }
}

impl From<TryFromIntError> for GenerationError {
    fn from(cause: TryFromIntError) -> Self {
        Self::new(cause.into(), GenerationErrorKind::OutOfRange)
    }
}
