//! Non-generic JVM method descriptors.
//!
//! This module provides functionality for parsing and representing JVM method descriptors,
//! which encode the parameter types and return type of a method.
//!
#![doc = see_jvm_spec!(4, 3, 3)]

use std::str::FromStr;

use derive_more::{Display, From};
use itertools::Itertools;

use super::{Descriptor, field_type::FieldType};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{JavaStr, JavaString},
};

/// The descriptor of a method, representing its parameters and return type in JVM format.
///
/// A method descriptor encapsulates:
/// - A list of parameter types in the order they appear in the method signature
/// - A return type (which can be void)
///
#[doc = see_jvm_spec!(4, 3, 3)]
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use mokapot::types::method_descriptor::MethodDescriptor;
///
/// // Parse a method descriptor for: void main(String[] args)
/// let main_method = MethodDescriptor::from_str("([Ljava/lang/String;)V").unwrap();
///
/// // Parse a method descriptor for: int add(int a, int b)
/// let add_method = MethodDescriptor::from_str("(II)I").unwrap();
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display)]
#[display(
    "({}) -> {return_type}",
    parameters_types.iter().map(FieldType::descriptor).join(", ")
)]
pub struct MethodDescriptor {
    /// The types of the method parameters in order of declaration.
    /// For instance, for a method `foo(int x, String y)`, this would contain
    /// `[FieldType::Int, FieldType::Object("java/lang/String")]`.
    pub parameters_types: Box<[FieldType]>,
    /// The return type of the method, which can be either a specific type or void.
    pub return_type: ReturnType,
}

impl Descriptor for MethodDescriptor {
    /// Returns the descriptor string for this method descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// use mokapot::types::method_descriptor::MethodDescriptor;
    ///
    /// // Parse a method descriptor for: int add(int a, int b)
    /// let add_method = MethodDescriptor::from_str("(II)I").unwrap();
    /// use mokapot::types::Descriptor;
    /// assert_eq!(add_method.descriptor(), "(II)I");
    /// ```
    fn descriptor(&self) -> JavaString {
        let mut bytes = vec![b'('];
        for parameter in &self.parameters_types {
            parameter.push_descriptor(&mut bytes);
        }
        bytes.push(b')');
        self.return_type.push_descriptor(&mut bytes);
        JavaString::from_cesu8(bytes).expect("method descriptors contain valid modified UTF-8")
    }
}

/// The return type of a method in the JVM type system.
///
/// In the JVM, a method's return type can be either:
/// - A specific type (primitive or reference type)
/// - Void (representing no return value)
///
/// # Examples
///
/// ```
/// use mokapot::types::method_descriptor::ReturnType;
/// use mokapot::types::field_type::PrimitiveType;
///
/// // void return type
/// let void_return = ReturnType::Void;
///
/// // int return type
/// let int_return = ReturnType::Some(PrimitiveType::Int.into());
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display, From)]
pub enum ReturnType {
    /// Represents a method that returns a specific type.
    /// The contained `FieldType` can be either a primitive type or a reference type.
    Some(FieldType),
    /// Represents a void return type (i.e., the method returns no value).
    /// In JVM descriptor format, this is represented by the character 'V'.
    #[display("void")]
    Void,
}

impl Descriptor for ReturnType {
    fn descriptor(&self) -> JavaString {
        match self {
            ReturnType::Some(field_type) => field_type.descriptor(),
            ReturnType::Void => JavaString::from("V".to_owned()),
        }
    }
}

impl ReturnType {
    fn push_descriptor(&self, bytes: &mut Vec<u8>) {
        match self {
            Self::Some(field_type) => field_type.push_descriptor(bytes),
            Self::Void => bytes.push(b'V'),
        }
    }
}

impl FromStr for MethodDescriptor {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let descriptor = JavaString::from(descriptor.to_owned());
        Self::try_from(descriptor.as_java_str())
    }
}

/// Parses the parameter types portion of a method descriptor.
///
/// Consumes the leading `(` from `input`, then extracts each parameter type
/// descriptor until `)` is reached. On success, `input` is advanced past the
/// closing `)`.
fn parse_params(input: &mut &JavaStr) -> Result<Box<[FieldType]>, InvalidDescriptor> {
    let Some((b'(', mut rest)) = input.split_first_ascii() else {
        return Err(InvalidDescriptor);
    };

    let parameters_types = std::iter::from_fn(|| {
        if let Some((b')', after_paren)) = rest.split_first_ascii() {
            rest = after_paren;
            return None;
        }
        Some(FieldType::parse_prefix(&mut rest))
    })
    .collect::<Result<_, _>>()
    .inspect(|_| *input = rest)?;

    Ok(parameters_types)
}

impl TryFrom<&JavaStr> for MethodDescriptor {
    type Error = InvalidDescriptor;

    fn try_from(descriptor: &JavaStr) -> Result<Self, Self::Error> {
        let mut rest = descriptor;
        let parameters_types = parse_params(&mut rest)?;
        let return_type = ReturnType::try_from(rest)?;
        Ok(Self {
            parameters_types,
            return_type,
        })
    }
}

/// An error indicating that a method descriptor string is invalid according to the JVM specification.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("Invalid descriptor")]
pub struct InvalidDescriptor;

impl FromStr for ReturnType {
    type Err = InvalidDescriptor;
    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let descriptor = JavaString::from(descriptor.to_owned());
        Self::try_from(descriptor.as_java_str())
    }
}

impl TryFrom<&JavaStr> for ReturnType {
    type Error = InvalidDescriptor;

    fn try_from(descriptor: &JavaStr) -> Result<Self, Self::Error> {
        if descriptor.as_bytes() == b"V" {
            Ok(Self::Void)
        } else {
            FieldType::try_from(descriptor).map(Self::Some)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rejects_malformed_descriptor_boundaries() {
        for descriptor in [
            "",
            "(",
            "([)V",
            "(Ljava/lang/String)V",
            "(V)V",
            "([V)V",
            "(IL;)V",
        ] {
            let encoded = JavaString::from(descriptor.to_owned());
            let mut input = encoded.as_java_str();
            assert_eq!(parse_params(&mut input), Err(InvalidDescriptor));
            assert_eq!(input, encoded.as_java_str());
            let parse = descriptor.parse::<MethodDescriptor>();
            assert_eq!(parse, Err(InvalidDescriptor));
        }
        for descriptor in ["", "A", "Vx", "Ix"] {
            assert_eq!(descriptor.parse::<ReturnType>(), Err(InvalidDescriptor));
            let parse = format!("(){descriptor}").parse::<MethodDescriptor>();
            assert_eq!(parse, Err(InvalidDescriptor));
        }
        assert_eq!("(I)".parse::<MethodDescriptor>(), Err(InvalidDescriptor));
    }

    #[test]
    fn parses_and_emits_modified_utf8_class_names() {
        for name in [
            b"p/\xed\xa0\x80".as_slice(),
            b"p/\xed\xa0\xbd\xed\xb8\x80",
            b"p/\xc0\x80",
        ] {
            let bytes = [b"(L", name, b";)[L", name, b";"].concat();
            let descriptor = JavaStr::from_cesu8(&bytes).unwrap();
            let method = MethodDescriptor::try_from(descriptor).unwrap();
            assert_eq!(method.descriptor().as_java_str().as_bytes(), bytes);
        }
    }
}
