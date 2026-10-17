//! [`ClassElement`] implementations for annotation types.

use itertools::Itertools;

use super::{ParseError, ParsingContext, class_element::ClassElement, raw_attributes};
use crate::{
    jvm::{
        Annotation, ConstantValue, TypeAnnotation,
        annotation::{
            ElementValue, OffsetOf, TargetInfo, TypeArgumentLocation, TypeParameterLocation,
            TypePathElement, VariableKind,
        },
        code::LocalVariableId,
        errors::ParsingErrorContext,
    },
    types::field_type::PrimitiveType,
};

impl ClassElement for TypePathElement {
    type Raw = (u8, u8);
    fn from_raw(raw: Self::Raw, _ctx: &ParsingContext) -> Result<Self, ParseError> {
        let (kind, argument_index) = raw;
        match (kind, argument_index) {
            (0, 0) => Ok(Self::Array),
            (1, 0) => Ok(Self::Nested),
            (2, 0) => Ok(Self::Bound),
            (3, idx) => Ok(Self::TypeArgument(idx)),
            _ => Err(ParseError::malform("Invalid type path kind")),
        }
    }
}

impl ClassElement for Annotation {
    type Raw = raw_attributes::Annotation;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            type_index,
            element_value_pairs,
        } = raw;
        let annotation_type = ctx
            .constant_pool
            .get_str(type_index)?
            .parse()
            .context("Invalid annotation type descriptor")?;
        let element_value_pairs = element_value_pairs
            .into_iter()
            .map(|(name_idx, raw_value)| {
                let element_name = ctx.constant_pool.get_str(name_idx)?;
                let element_value = ElementValue::from_raw(raw_value, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, ParseError>>()?;
        Ok(Annotation {
            annotation_type,
            element_value_pairs,
        })
    }
}

impl ClassElement for TypeAnnotation {
    type Raw = raw_attributes::TypeAnnotation;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            target_info,
            target_path,
            type_index,
            element_value_pairs,
        } = raw;

        let target_info = TargetInfo::from_raw(target_info, ctx)?;
        let target_path = target_path
            .into_iter()
            .map(|raw| ClassElement::from_raw(raw, ctx))
            .collect::<Result<_, _>>()?;
        let annotation_type = ctx
            .constant_pool
            .get_str(type_index)?
            .parse()
            .context("Invalid annotation type descriptor")?;
        let element_value_pairs = element_value_pairs
            .into_iter()
            .map(|(name_idx, value)| {
                let element_name = ctx.constant_pool.get_str(name_idx)?;
                let element_value = ClassElement::from_raw(value, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, ParseError>>()?;
        Ok(TypeAnnotation {
            annotation_type,
            target_info,
            target_path,
            element_value_pairs,
        })
    }
}

impl ClassElement for TargetInfo {
    type Raw = raw_attributes::TargetInfo;

    #[allow(clippy::enum_glob_use)]
    #[expect(
        clippy::too_many_lines,
        reason = "Exhaustive conversion of JVMS annotation targets"
    )]
    fn from_raw(raw: Self::Raw, _ctx: &ParsingContext) -> Result<Self, ParseError> {
        use OffsetOf::{InstanceOf, New};
        use TypeArgumentLocation::*;
        use TypeParameterLocation::*;
        use VariableKind::*;

        let lifted = match raw {
            Self::Raw::TypeParameterOfClass {
                type_parameter_index,
            } => Self::TypeParameter {
                location: Class,
                index: type_parameter_index,
            },
            Self::Raw::TypeParameterOfMethod {
                type_parameter_index,
            } => Self::TypeParameter {
                location: Method,
                index: type_parameter_index,
            },
            Self::Raw::SuperType { supertype_index } => Self::SuperType {
                index: supertype_index,
            },
            Self::Raw::TypeParameterBoundOfClass {
                type_parameter_index,
                bound_index,
            } => Self::TypeParameterBound {
                location: Class,
                type_parameter_index,
                bound_index,
            },
            Self::Raw::TypeParameterBoundOfMethod {
                type_parameter_index,
                bound_index,
            } => Self::TypeParameterBound {
                location: Method,
                type_parameter_index,
                bound_index,
            },
            Self::Raw::Field => Self::Field,
            Self::Raw::MethodReturn => Self::MethodReturn,
            Self::Raw::Receiver => Self::Receiver,
            Self::Raw::FormalParameter {
                formal_parameter_index,
            } => Self::FormalParameter {
                index: formal_parameter_index,
            },
            Self::Raw::Throws { throws_type_index } => Self::Throws {
                index: throws_type_index,
            },
            Self::Raw::LocalVariable(table) => {
                let table = table
                    .into_iter()
                    .map(|(start, len, index)| -> Result<_, ParseError> {
                        let range = start..(start + len).context("Invalid jump offset")?;
                        Ok(LocalVariableId { range, index })
                    })
                    .try_collect()?;
                Self::LocalVar(Local, table)
            }
            Self::Raw::ResourceVariable(table) => {
                let table = table
                    .into_iter()
                    .map(|(start, len, index)| -> Result<_, ParseError> {
                        let range = start..(start + len).context("Invalid jump offset")?;
                        Ok(LocalVariableId { range, index })
                    })
                    .try_collect()?;
                Self::LocalVar(Resource, table)
            }
            Self::Raw::Catch {
                exception_table_index,
            } => Self::Catch {
                index: exception_table_index,
            },
            Self::Raw::InstanceOf { offset } => Self::Offset(InstanceOf, offset),
            Self::Raw::New { offset } => Self::Offset(New, offset),
            Self::Raw::ConstructorReference { offset } => {
                Self::Offset(OffsetOf::ConstructorReference, offset)
            }
            Self::Raw::MethodReference { offset } => {
                Self::Offset(OffsetOf::MethodReference, offset)
            }
            Self::Raw::TypeInCast {
                offset,
                type_argument_index,
            } => Self::TypeArgument {
                location: Cast,
                offset,
                index: type_argument_index,
            },
            Self::Raw::TypeArgumentInConstructor {
                offset,
                type_argument_index,
            } => Self::TypeArgument {
                location: Constructor,
                offset,
                index: type_argument_index,
            },
            Self::Raw::TypeArgumentInCall {
                offset,
                type_argument_index,
            } => Self::TypeArgument {
                location: MethodCall,
                offset,
                index: type_argument_index,
            },
            Self::Raw::TypeArgumentInConstructorReference {
                offset,
                type_argument_index,
            } => Self::TypeArgument {
                location: ConstructorReference,
                offset,
                index: type_argument_index,
            },
            Self::Raw::TypeArgumentInMethodReference {
                offset,
                type_argument_index,
            } => Self::TypeArgument {
                location: MethodReference,
                offset,
                index: type_argument_index,
            },
        };
        Ok(lifted)
    }
}

impl ClassElement for ElementValue {
    type Raw = raw_attributes::ElementValueInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let cp = &ctx.constant_pool;
        match raw {
            Self::Raw::Const(
                tag @ (b'B' | b'C' | b'I' | b'S' | b'Z' | b'F' | b'D' | b'J'),
                idx,
            ) => {
                use ConstantValue::{Double, Float, Integer, Long};
                let value = cp.get_constant_value(idx)?;
                match (tag, value) {
                    (b'B', it @ Integer(_)) => Ok(Self::Primitive(PrimitiveType::Byte, it)),
                    (b'C', it @ Integer(_)) => Ok(Self::Primitive(PrimitiveType::Char, it)),
                    (b'I', it @ Integer(_)) => Ok(Self::Primitive(PrimitiveType::Int, it)),
                    (b'S', it @ Integer(_)) => Ok(Self::Primitive(PrimitiveType::Short, it)),
                    (b'Z', it @ Integer(_)) => Ok(Self::Primitive(PrimitiveType::Boolean, it)),
                    (b'F', it @ Float(_)) => Ok(Self::Primitive(PrimitiveType::Float, it)),
                    (b'D', it @ Double(_)) => Ok(Self::Primitive(PrimitiveType::Double, it)),
                    (b'J', it @ Long(_)) => Ok(Self::Primitive(PrimitiveType::Long, it)),
                    _ => Err(ParseError::malform("Constant value type mismatch")),
                }
            }
            Self::Raw::Const(b's', idx) => Ok(Self::String(cp.get_java_string(idx)?)),
            Self::Raw::Const(_, _) => Err(ParseError::malform("Invalid constant value tag")),
            Self::Raw::Enum {
                type_name_index,
                const_name_index,
            } => {
                let enum_type = cp
                    .get_str(type_name_index)?
                    .parse()
                    .context("Invalid enum type descriptor")?;
                let const_name = cp.get_str(const_name_index)?.to_owned();
                Ok(Self::EnumConstant {
                    enum_type,
                    const_name,
                })
            }
            Self::Raw::ClassInfo(idx) => {
                let return_descriptor = cp
                    .get_str(idx)?
                    .parse()
                    .context("Invalid return type descriptor")?;
                Ok(Self::Class { return_descriptor })
            }
            Self::Raw::Annotation(annotation_info) => Ok(Self::AnnotationInterface(
                Annotation::from_raw(annotation_info, ctx)?,
            )),
            Self::Raw::Array(values) => {
                let values = values
                    .into_iter()
                    .map(|raw_value| ClassElement::from_raw(raw_value, ctx))
                    .collect::<Result<_, _>>()?;
                Ok(Self::Array(values))
            }
        }
    }
}
