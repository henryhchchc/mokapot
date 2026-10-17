//! Annotation decoding and resolution.

mod raw;

use itertools::Itertools;

use super::{
    Decode, ParseError, ResolutionContext,
    decoder::{borrow_slice, decode_value, parse_repeated},
};
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

impl TryFrom<raw::TypePathElement> for TypePathElement {
    type Error = ParseError;
    fn try_from(raw: raw::TypePathElement) -> Result<Self, Self::Error> {
        let raw::TypePathElement {
            kind,
            argument_index,
        } = raw;
        match (kind, argument_index) {
            (0, 0) => Ok(TypePathElement::Array),
            (1, 0) => Ok(TypePathElement::Nested),
            (2, 0) => Ok(TypePathElement::Bound),
            (3, idx) => Ok(TypePathElement::TypeArgument(idx)),
            _ => Err(ParseError::malform("Invalid type path kind")),
        }
    }
}

pub(super) fn decode_annotation(
    bytes: &mut &[u8],
    ctx: &ResolutionContext,
) -> Result<Annotation, ParseError> {
    let type_index = decode_value(bytes)?;
    let annotation_type = ctx
        .constant_pool
        .get_str(type_index)?
        .parse()
        .context("Invalid annotation type descriptor")?;
    let count: u16 = decode_value(bytes)?;
    let element_value_pairs = parse_repeated(bytes, count, |bytes| {
        let name_index = decode_value(bytes)?;
        let element_name = ctx.constant_pool.get_str(name_index)?;
        let element_value = decode_element_value(bytes, ctx)?;
        Ok((element_name.to_owned(), element_value))
    })?;
    Ok(Annotation {
        annotation_type,
        element_value_pairs,
    })
}

pub(super) fn decode_type_annotation(
    bytes: &mut &[u8],
    ctx: &ResolutionContext,
) -> Result<TypeAnnotation, ParseError> {
    let target_info = TargetInfo::try_from(raw::TargetInfo::decode(bytes)?)?;
    let path_length: u8 = decode_value(bytes)?;
    let target_path = borrow_slice::<raw::TypePathElement>(bytes, usize::from(path_length))?
        .iter()
        .copied()
        .map(TypePathElement::try_from)
        .collect::<Result<_, _>>()?;
    let Annotation {
        annotation_type,
        element_value_pairs,
    } = decode_annotation(bytes, ctx)?;
    Ok(TypeAnnotation {
        annotation_type,
        target_info,
        target_path,
        element_value_pairs,
    })
}

impl<'a> TryFrom<raw::TargetInfo<'a>> for TargetInfo {
    type Error = ParseError;

    #[allow(clippy::enum_glob_use)]
    #[expect(
        clippy::too_many_lines,
        reason = "Exhaustive conversion of JVMS annotation targets"
    )]
    fn try_from(raw: raw::TargetInfo<'a>) -> Result<Self, Self::Error> {
        use OffsetOf::{InstanceOf, New};
        use TypeArgumentLocation::*;
        use TypeParameterLocation::*;
        use VariableKind::*;

        let lifted = match raw {
            raw::TargetInfo::TypeParameterOfClass {
                type_parameter_index,
            } => TargetInfo::TypeParameter {
                location: Class,
                index: type_parameter_index,
            },
            raw::TargetInfo::TypeParameterOfMethod {
                type_parameter_index,
            } => TargetInfo::TypeParameter {
                location: Method,
                index: type_parameter_index,
            },
            raw::TargetInfo::SuperType { supertype_index } => TargetInfo::SuperType {
                index: supertype_index,
            },
            raw::TargetInfo::TypeParameterBoundOfClass {
                type_parameter_index,
                bound_index,
            } => TargetInfo::TypeParameterBound {
                location: Class,
                type_parameter_index,
                bound_index,
            },
            raw::TargetInfo::TypeParameterBoundOfMethod {
                type_parameter_index,
                bound_index,
            } => TargetInfo::TypeParameterBound {
                location: Method,
                type_parameter_index,
                bound_index,
            },
            raw::TargetInfo::Field => TargetInfo::Field,
            raw::TargetInfo::MethodReturn => TargetInfo::MethodReturn,
            raw::TargetInfo::Receiver => TargetInfo::Receiver,
            raw::TargetInfo::FormalParameter {
                formal_parameter_index,
            } => TargetInfo::FormalParameter {
                index: formal_parameter_index,
            },
            raw::TargetInfo::Throws { throws_type_index } => TargetInfo::Throws {
                index: throws_type_index,
            },
            raw::TargetInfo::LocalVariable(table) => {
                let table = table
                    .iter()
                    .map(|entry| -> Result<_, ParseError> {
                        let start = crate::jvm::code::ProgramCounter::from(entry.start_pc);
                        let len = entry.length.get();
                        let index = entry.index.get();
                        let range = start..(start + len).context("Invalid jump offset")?;
                        Ok(LocalVariableId { range, index })
                    })
                    .try_collect()?;
                TargetInfo::LocalVar(Local, table)
            }
            raw::TargetInfo::ResourceVariable(table) => {
                let table = table
                    .iter()
                    .map(|entry| -> Result<_, ParseError> {
                        let start = crate::jvm::code::ProgramCounter::from(entry.start_pc);
                        let len = entry.length.get();
                        let index = entry.index.get();
                        let range = start..(start + len).context("Invalid jump offset")?;
                        Ok(LocalVariableId { range, index })
                    })
                    .try_collect()?;
                TargetInfo::LocalVar(Resource, table)
            }
            raw::TargetInfo::Catch {
                exception_table_index,
            } => TargetInfo::Catch {
                index: exception_table_index,
            },
            raw::TargetInfo::InstanceOf { offset } => TargetInfo::Offset(InstanceOf, offset),
            raw::TargetInfo::New { offset } => TargetInfo::Offset(New, offset),
            raw::TargetInfo::ConstructorReference { offset } => {
                TargetInfo::Offset(OffsetOf::ConstructorReference, offset)
            }
            raw::TargetInfo::MethodReference { offset } => {
                TargetInfo::Offset(OffsetOf::MethodReference, offset)
            }
            raw::TargetInfo::TypeInCast {
                offset,
                type_argument_index,
            } => TargetInfo::TypeArgument {
                location: Cast,
                offset: offset.into(),
                index: type_argument_index,
            },
            raw::TargetInfo::TypeArgumentInConstructor {
                offset,
                type_argument_index,
            } => TargetInfo::TypeArgument {
                location: Constructor,
                offset: offset.into(),
                index: type_argument_index,
            },
            raw::TargetInfo::TypeArgumentInCall {
                offset,
                type_argument_index,
            } => TargetInfo::TypeArgument {
                location: MethodCall,
                offset: offset.into(),
                index: type_argument_index,
            },
            raw::TargetInfo::TypeArgumentInConstructorReference {
                offset,
                type_argument_index,
            } => TargetInfo::TypeArgument {
                location: ConstructorReference,
                offset: offset.into(),
                index: type_argument_index,
            },
            raw::TargetInfo::TypeArgumentInMethodReference {
                offset,
                type_argument_index,
            } => TargetInfo::TypeArgument {
                location: MethodReference,
                offset: offset.into(),
                index: type_argument_index,
            },
        };
        Ok(lifted)
    }
}

pub(super) fn decode_element_value(
    bytes: &mut &[u8],
    ctx: &ResolutionContext,
) -> Result<ElementValue, ParseError> {
    let cp = &ctx.constant_pool;
    let tag: u8 = decode_value(bytes)?;
    match tag {
        b'B' | b'C' | b'I' | b'S' | b'Z' | b'F' | b'D' | b'J' => {
            use ConstantValue::{Double, Float, Integer, Long};
            let idx = decode_value(bytes)?;
            let value = cp.get_constant_value(idx)?;
            match (tag, value) {
                (b'B', it @ Integer(_)) => Ok(ElementValue::Primitive(PrimitiveType::Byte, it)),
                (b'C', it @ Integer(_)) => Ok(ElementValue::Primitive(PrimitiveType::Char, it)),
                (b'I', it @ Integer(_)) => Ok(ElementValue::Primitive(PrimitiveType::Int, it)),
                (b'S', it @ Integer(_)) => Ok(ElementValue::Primitive(PrimitiveType::Short, it)),
                (b'Z', it @ Integer(_)) => Ok(ElementValue::Primitive(PrimitiveType::Boolean, it)),
                (b'F', it @ Float(_)) => Ok(ElementValue::Primitive(PrimitiveType::Float, it)),
                (b'D', it @ Double(_)) => Ok(ElementValue::Primitive(PrimitiveType::Double, it)),
                (b'J', it @ Long(_)) => Ok(ElementValue::Primitive(PrimitiveType::Long, it)),
                _ => Err(ParseError::malform("Constant value type mismatch")),
            }
        }
        b's' => {
            let index = decode_value(bytes)?;
            Ok(ElementValue::String(cp.get_java_string(index)?))
        }
        b'e' => {
            let type_name_index = decode_value(bytes)?;
            let const_name_index = decode_value(bytes)?;
            let enum_type = cp
                .get_str(type_name_index)?
                .parse()
                .context("Invalid enum type descriptor")?;
            let const_name = cp.get_str(const_name_index)?.to_owned();
            Ok(ElementValue::EnumConstant {
                enum_type,
                const_name,
            })
        }
        b'c' => {
            let index = decode_value(bytes)?;
            let return_descriptor = cp
                .get_str(index)?
                .parse()
                .context("Invalid return type descriptor")?;
            Ok(ElementValue::Class { return_descriptor })
        }
        b'@' => decode_annotation(bytes, ctx).map(ElementValue::AnnotationInterface),
        b'[' => {
            let count: u16 = decode_value(bytes)?;
            parse_repeated(bytes, count, |bytes| decode_element_value(bytes, ctx))
                .map(ElementValue::Array)
        }
        _ => Err(ParseError::malform(format!(
            "Unknown element value tag: {tag}"
        ))),
    }
}

#[cfg(test)]
mod tests;
