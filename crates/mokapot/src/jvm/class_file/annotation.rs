//! Annotation decoding and resolution.

pub(super) mod raw;

use crate::intrinsics::ExactSizeIteratorExt;

use super::{ParseError, ResolutionContext, resolve::Resolve};
use crate::{
    jvm::{
        ConstantVal,
        annotation::{
            AnnotationData, ElementValueData, OffsetOf, TargetInfo, TypeAnnotationData,
            TypeArgumentLocation, TypeParameterLocation, TypePathElement, VariableKind,
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

impl Resolve for raw::Annotation {
    type Output = AnnotationData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::Annotation {
            type_index,
            element_value_pairs,
        } = raw;
        let annotation_type = ctx.constant_pool.intern_field_type(type_index)?;
        let element_value_pairs = element_value_pairs
            .into_iter()
            .map(|(name_idx, raw_value)| {
                let element_name = ctx.constant_pool.utf8_id(name_idx)?;
                let element_value = raw_value.resolve(ctx)?;
                Ok((element_name, element_value))
            })
            .try_collect_exact()?;
        Ok(AnnotationData {
            annotation_type,
            element_value_pairs,
        })
    }
}

impl Resolve for raw::TypeAnnotation<'_> {
    type Output = TypeAnnotationData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::TypeAnnotation {
            target_info,
            target_path,
            type_index,
            element_value_pairs,
        } = raw;

        let target_info = TargetInfo::try_from(target_info)?;
        let target_path = target_path
            .iter()
            .copied()
            .map(TypePathElement::try_from)
            .try_collect_exact()?;
        let annotation_type = ctx.constant_pool.intern_field_type(type_index)?;
        let element_value_pairs = element_value_pairs
            .into_iter()
            .map(|(name_idx, value)| {
                let element_name = ctx.constant_pool.utf8_id(name_idx)?;
                let element_value = value.resolve(ctx)?;
                Ok((element_name, element_value))
            })
            .try_collect_exact()?;
        Ok(TypeAnnotationData {
            annotation_type,
            target_info,
            target_path,
            element_value_pairs,
        })
    }
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
                    .try_collect_exact()?;
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
                    .try_collect_exact()?;
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

impl Resolve for raw::ElementValueInfo {
    type Output = ElementValueData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let cp = &mut ctx.constant_pool;
        match raw {
            raw::ElementValueInfo::Const(
                tag @ (b'B' | b'C' | b'I' | b'S' | b'Z' | b'F' | b'D' | b'J'),
                idx,
            ) => {
                use ConstantVal::{Double, Float, Integer, Long};
                let id = cp.constant_value_id(idx)?;
                let kind = match (tag, cp.constant_value(id)) {
                    (b'B', Integer(_)) => PrimitiveType::Byte,
                    (b'C', Integer(_)) => PrimitiveType::Char,
                    (b'I', Integer(_)) => PrimitiveType::Int,
                    (b'S', Integer(_)) => PrimitiveType::Short,
                    (b'Z', Integer(_)) => PrimitiveType::Boolean,
                    (b'F', Float(_)) => PrimitiveType::Float,
                    (b'D', Double(_)) => PrimitiveType::Double,
                    (b'J', Long(_)) => PrimitiveType::Long,
                    _ => return Err(ParseError::malform("Constant value type mismatch")),
                };
                Ok(ElementValueData::Primitive(kind, id))
            }
            raw::ElementValueInfo::Const(b's', idx) => {
                Ok(ElementValueData::String(cp.utf8_id(idx)?))
            }
            raw::ElementValueInfo::Const(_, _) => {
                Err(ParseError::malform("Invalid constant value tag"))
            }
            raw::ElementValueInfo::Enum {
                type_name_index,
                const_name_index,
            } => {
                let enum_type = cp.intern_field_type(type_name_index)?;
                let const_name = cp.utf8_id(const_name_index)?;
                Ok(ElementValueData::EnumConstant {
                    enum_type,
                    const_name,
                })
            }
            raw::ElementValueInfo::ClassInfo(idx) => {
                let return_descriptor = cp.intern_return_type(idx)?;
                Ok(ElementValueData::Class { return_descriptor })
            }
            raw::ElementValueInfo::Annotation(annotation_info) => Ok(
                ElementValueData::AnnotationInterface(annotation_info.resolve(ctx)?),
            ),
            raw::ElementValueInfo::Array(values) => {
                let values = values
                    .into_iter()
                    .map(|raw_value| raw_value.resolve(ctx))
                    .try_collect_exact()?;
                Ok(ElementValueData::Array(values))
            }
        }
    }
}

#[cfg(test)]
mod tests;
