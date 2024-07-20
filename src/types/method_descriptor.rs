//! Non-generic JVM method descriptors.

use itertools::Itertools;
use std::{
    fmt::Display,
    str::{Chars, FromStr},
};

use crate::{jvm::references::ClassRef, macros::see_jvm_spec};

use super::field_type::{FieldType, PrimitiveType};

/// The descriptor of a method.
/// Consists of the parameters types and the return type.
#[doc = see_jvm_spec!(4, 3, 3)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct MethodDescriptor {
    /// The type of the parameters.
    pub parameters_types: Vec<FieldType>,
    /// The return type.
    pub return_type: ReturnType,
}

/// Denotes the return type of a method.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ReturnType {
    /// The method returns a specific type.
    Some(FieldType),
    /// The return type of the method is `void`.
    Void,
}

impl Display for ReturnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReturnType::Some(t) => t.fmt(f),
            ReturnType::Void => write!(f, "void"),
        }
    }
}

impl Display for MethodDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        for param in &self.parameters_types {
            write!(f, "{}", param.descriptor())?;
        }
        write!(f, ")")?;
        write!(f, "{}", self.return_type.descriptor())
    }
}

impl MethodDescriptor {
    /// Parses a method descriptor from a string and advances the iterator.
    /// For an input as follows.
    /// ```text
    ///   L      java/lang/String;IJB)V
    ///   ^      ^
    ///   prefix remaining
    /// ````
    /// It returns a [`FieldType::Object`] with `"java/lang/String"` and the [remaining] is as
    /// follows.
    /// ```text
    ///   ...;IJB)V
    ///       ^
    ///       remaining
    /// ````
    fn parse_single_param(
        prefix: char,
        remaining: &mut Chars<'_>,
    ) -> Result<FieldType, InvalidDescriptor> {
        if let Ok(p) = PrimitiveType::try_from(prefix) {
            Ok(FieldType::Base(p))
        } else {
            match prefix {
                'L' => {
                    let binary_name: String = remaining.take_while_ref(|c| *c != ';').collect();
                    match remaining.next() {
                        Some(';') => Ok(FieldType::Object(ClassRef::new(binary_name))),
                        _ => Err(InvalidDescriptor),
                    }
                }
                '[' => {
                    let next_prefix = remaining.next().ok_or(InvalidDescriptor)?;
                    Self::parse_single_param(next_prefix, remaining).map(FieldType::into_array_type)
                }
                _ => Err(InvalidDescriptor),
            }
        }
    }
}

impl FromStr for MethodDescriptor {
    type Err = InvalidDescriptor;

    fn from_str(descriptor: &str) -> Result<Self, Self::Err> {
        let mut chars = descriptor.chars();
        let mut parameters_types = Vec::new();
        let return_type = loop {
            match chars.next() {
                Some('(') => {}
                Some(')') => break ReturnType::from_str(chars.as_str())?,
                Some(c) => {
                    let param = Self::parse_single_param(c, &mut chars)?;
                    parameters_types.push(param);
                }
                None => Err(InvalidDescriptor)?,
            }
        };
        Ok(Self {
            parameters_types,
            return_type,
        })
    }
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
