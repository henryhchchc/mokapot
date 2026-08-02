//! Non-generic JVM method descriptors.
//!
//! This module provides functionality for parsing and representing JVM method descriptors,
//! which encode the parameter types and return type of a method.
//!
#![doc = see_jvm_spec!(4, 3, 3)]

use std::str::FromStr;

use itertools::Itertools;

use super::{Descriptor, field_type::FieldType};
use crate::intrinsics::see_jvm_spec;

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

impl FromStr for MethodDescriptor {
    type Err = InvalidDescriptor;

    fn from_str(mut descriptor: &str) -> Result<Self, Self::Err> {
        let parameters_types = parse_params(&mut descriptor)?;
        let return_type = ReturnType::from_str(descriptor)?;
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
fn parse_params(payload: &mut &str) -> Result<Vec<FieldType>, InvalidDescriptor> {
    let mut rest = payload.strip_prefix(PARAM_START).ok_or(InvalidDescriptor)?;

    let parameters_types = std::iter::from_fn(|| {
        if let Some(after_params) = rest.strip_prefix(PARAM_END) {
            rest = after_params;
            return None;
        }
        Some(FieldType::parse_prefix(&mut rest))
    })
    .collect::<Result<_, _>>()
    .inspect(|_| *payload = rest)?;

    Ok(parameters_types)
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
