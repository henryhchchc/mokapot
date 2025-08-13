//! Non-generic JVM method descriptors.
//!
//! This module provides functionality for parsing and representing JVM method descriptors,
//! which encode the parameter types and return type of a method.
//!
#![doc = see_jvm_spec!(4, 3, 3)]

use std::str::FromStr;

use itertools::Itertools;

use super::{
    Descriptor,
    field_type::{FieldType, PrimitiveType},
};
use crate::{intrinsics::see_jvm_spec, jvm::references::ClassRef};

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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display)]
#[display(
    "({}) -> {return_type}",
    parameters_types.iter().map(FieldType::descriptor).join(", ")
)]
pub struct MethodDescriptor {
    /// The types of the method parameters in order of declaration.
    /// For instance, for a method `foo(int x, String y)`, this would contain
    /// `[FieldType::Int, FieldType::Object("java/lang/String")]`.
    pub parameters_types: Vec<FieldType>,
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
    /// ```
    fn descriptor(&self) -> String {
        format!(
            "({}){}",
            self.parameters_types
                .iter()
                .map(FieldType::descriptor)
                .join(""),
            self.return_type.descriptor()
        )
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
#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display, derive_more::From,
)]
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
    fn descriptor(&self) -> String {
        match self {
            ReturnType::Some(field_type) => field_type.descriptor(),
            ReturnType::Void => "V".to_string(),
        }
    }
}

/// Character that starts a method descriptor's parameter list
const PARAM_START: char = '(';
/// Character that ends a method descriptor's parameter list
const PARAM_END: char = ')';
/// Character that indicates an array type
const ARRAY_MARKER: char = '[';
/// Character that starts an object type descriptor
const OBJECT_MARKER: char = 'L';
/// Character that ends an object type descriptor
const OBJECT_END: char = ';';

impl FromStr for MethodDescriptor {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let remaining = descriptor
            .strip_prefix(PARAM_START)
            .ok_or(InvalidDescriptor)?;
        let (parameters_types, remaining) = parse_params(remaining)?;
        let return_type = ReturnType::from_str(remaining)?;
        Ok(Self {
            parameters_types,
            return_type,
        })
    }
}

/// Parses the parameter types portion of a method descriptor.
///
/// This function processes the characters between '(' and ')', extracting each parameter
/// type descriptor and converting it into a `FieldType`.
fn parse_params(
    mut remaining: &str,
) -> Result<(Vec<FieldType>, &str), <MethodDescriptor as FromStr>::Err> {
    let mut parameters_types = Vec::new();
    loop {
        if let Some(remaining) = remaining.strip_prefix(PARAM_END) {
            return Ok((parameters_types, remaining));
        }
        let (dimension, after_dim) = parse_array_dimension(remaining)?;
        let (base_type, after_param) = parse_next_param(after_dim)?;
        let param_type = (0..dimension).fold(base_type, |type_acc, _| type_acc.into_array_type());
        parameters_types.push(param_type);
        remaining = after_param;
    }
}

/// Parses a single parameter type from a method descriptor.
///
/// # Returns
///
/// Returns a tuple containing:
/// - The parsed field type
/// - The remaining unparsed portion of the input string
///
/// # Errors
///
/// Returns `InvalidDescriptor` if:
/// - The input string is empty
/// - The type descriptor is invalid or malformed
/// - An object type descriptor is not properly terminated with ';'
fn parse_next_param(input: &str) -> Result<(FieldType, &str), <MethodDescriptor as FromStr>::Err> {
    let (first_char, remaining) = input
        .chars()
        .next()
        .map(|c| (c, &input[c.len_utf8()..]))
        .ok_or(InvalidDescriptor)?;

    match first_char {
        primitive @ ('Z' | 'C' | 'F' | 'D' | 'B' | 'S' | 'I' | 'J') => {
            let param_type = PrimitiveType::try_from(primitive).map(Into::into)?;
            Ok((param_type, remaining))
        }
        OBJECT_MARKER => {
            let (class_name, rest) = remaining.split_once(OBJECT_END).ok_or(InvalidDescriptor)?;
            Ok((FieldType::Object(ClassRef::new(class_name)), rest))
        }
        _ => Err(InvalidDescriptor),
    }
}

/// Parses array dimensions from a type descriptor.
///
/// Counts consecutive '[' characters to determine the array dimensions.
///
/// # Returns
///
/// Returns a tuple containing:
/// - The number of array dimensions (number of '[' characters)
/// - The remaining unparsed portion of the input string
///
/// # Errors
///
/// Returns `InvalidDescriptor` if the number of array dimensions exceeds 255
fn parse_array_dimension(input: &str) -> Result<(u8, &str), InvalidDescriptor> {
    let count = input.chars().take_while(|&c| c == ARRAY_MARKER).count();
    let remaining = &input[count..];
    let dimension = u8::try_from(count).map_err(|_| InvalidDescriptor)?;
    Ok((dimension, remaining))
}

/// An error indicating that a method descriptor string is invalid according to the JVM specification.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("Invalid descriptor")]
pub struct InvalidDescriptor;

impl FromStr for ReturnType {
    type Err = InvalidDescriptor;
    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        if descriptor == "V" {
            Ok(ReturnType::Void)
        } else {
            FieldType::from_str(descriptor).map(ReturnType::Some)
        }
    }
}

impl ReturnType {
    /// Returns the JVM descriptor string representation of this return type.
    ///
    /// # Returns
    ///
    /// * "V" for void return type
    /// * The field type descriptor for specific return types
    ///
    /// # Examples
    ///
    /// ```
    /// use mokapot::types::method_descriptor::ReturnType;
    /// use mokapot::types::field_type::PrimitiveType;
    ///
    /// assert_eq!(ReturnType::Void.descriptor(), "V");
    /// assert_eq!(ReturnType::Some(PrimitiveType::Int.into()).descriptor(), "I");
    /// ```
    #[must_use]
    pub fn descriptor(&self) -> String {
        match self {
            ReturnType::Some(it) => it.descriptor(),
            ReturnType::Void => "V".to_owned(),
        }
    }
}
#[cfg(test)]
mod test {
    use proptest::prelude::*;

    use super::*;
    use crate::{tests::arb_field_type, types::field_type::FieldType};

    const MAX_PARAMS: usize = 10;

    fn arb_return_type() -> impl Strategy<Value = ReturnType> {
        prop_oneof![
            Just(ReturnType::Void),
            arb_field_type().prop_map(ReturnType::Some),
        ]
    }

    proptest! {

        #[test]
        fn roundtrip(
            params in prop::collection::vec(arb_field_type(), 0..MAX_PARAMS),
            ret in arb_return_type(),
        ) {
            let desc = MethodDescriptor {
                parameters_types: params,
                return_type: ret,
            };
            let str_desc = desc.descriptor();
            let parsed = MethodDescriptor::from_str(&str_desc).expect("Failed to parse method descriptor");
            assert_eq!(desc, parsed);
        }

        #[test]
        fn method_desc_from_str(
            params in prop::collection::vec(arb_field_type(), 0..MAX_PARAMS),
            ret in arb_return_type(),
        ) {
            let descriptor = format!(
                "({}){}",
                params.iter().map(FieldType::descriptor).join(""),
                ret.descriptor()
            );
            let parsed =
                MethodDescriptor::from_str(&descriptor).expect("Failed to parse method descriptor");
            assert_eq!(parsed.return_type, ret);
            assert_eq!(parsed.parameters_types, params);
        }

        #[test]
        fn too_many_return_type(
            params in prop::collection::vec(arb_field_type(), 0..MAX_PARAMS),
            rets in prop::collection::vec(arb_return_type(), 2..5),
        ) {
            let descriptor = format!(
                "({}){}",
                params.iter().map(FieldType::descriptor).join(""),
                rets.iter().map(ReturnType::descriptor).join(""),
            );
            assert!(MethodDescriptor::from_str(&descriptor).is_err());
        }
    }

    #[test]
    fn empty_desc() {
        let descriptor = "";
        let method_descriptor = MethodDescriptor::from_str(descriptor);
        assert!(method_descriptor.is_err());
    }

    #[test]
    fn incomplete_return_type() {
        let descriptor = "()Ljava/lang";
        let method_descriptor = MethodDescriptor::from_str(descriptor);
        assert!(method_descriptor.is_err());
    }

    #[test]
    fn missing_return_type() {
        let descriptor = "(I)";
        let method_descriptor = MethodDescriptor::from_str(descriptor);
        assert!(method_descriptor.is_err());
    }

    #[test]
    fn missing_semicolon() {
        let descriptor = "(I[Ljava/lang/StringJ)V";
        let method_descriptor = MethodDescriptor::from_str(descriptor);
        assert!(method_descriptor.is_err());
    }

    #[test]
    fn invalid_primitive() {
        let descriptor = "(V[Ljava/lang/String;J)V";
        let method_descriptor = MethodDescriptor::from_str(descriptor);
        assert!(method_descriptor.is_err());
    }
}
