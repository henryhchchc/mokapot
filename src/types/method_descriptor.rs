//! Non-generic JVM method descriptors.

use itertools::Itertools;
use std::str::FromStr;

use crate::{jvm::references::ClassRef, macros::see_jvm_spec};

use super::field_type::{FieldType, PrimitiveType};

/// The descriptor of a method.
/// Consists of the parameters types and the return type.
#[doc = see_jvm_spec!(4, 3, 3)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display)]
#[display(
    "({}){return_type}",
    parameters_types.iter().map(FieldType::descriptor).join("")
)]
pub struct MethodDescriptor {
    /// The type of the parameters.
    pub parameters_types: Vec<FieldType>,
    /// The return type.
    pub return_type: ReturnType,
}

/// Denotes the return type of a method.
#[derive(
    Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display, derive_more::From,
)]
pub enum ReturnType {
    /// The method returns a specific type.
    Some(FieldType),
    /// The return type of the method is `void`.
    #[display("void")]
    Void,
}

const PARAM_START: char = '(';
const PARAM_END: char = ')';
const ARRAY_MARKER: char = '[';
const OBJECT_MARKER: char = 'L';
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

fn parse_array_dimension(input: &str) -> Result<(u8, &str), InvalidDescriptor> {
    let count = input.chars().take_while(|&c| c == ARRAY_MARKER).count();
    let remaining = &input[count..];
    let dimension = u8::try_from(count).map_err(|_| InvalidDescriptor)?;
    Ok((dimension, remaining))
}

/// An error indicating that the descriptor string is invalid.
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
    /// Returns the descriptor for return type.
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
    use super::*;
    use proptest::prelude::*;

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
