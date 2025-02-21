use crate::{
    jvm::{
        Annotation, ConstantValue, TypeAnnotation,
        annotation::{ElementValue, TargetInfo, TypePathElement},
        class::constant_pool,
        code::LocalVariableId,
    },
    types::field_type::PrimitiveType,
};

use super::{Context, Error, jvm_element_parser::ClassElement, raw_attributes};

impl ClassElement for TypePathElement {
    type Raw = (u8, u8);
    fn from_raw(raw: Self::Raw, _ctx: &Context) -> Result<Self, Error> {
        let (kind, argument_index) = raw;
        match (kind, argument_index) {
            (0, 0) => Ok(Self::Array),
            (1, 0) => Ok(Self::Nested),
            (2, 0) => Ok(Self::Bound),
            (3, idx) => Ok(Self::TypeArgument(idx)),
            _ => Err(Error::InvalidTypePathKind),
        }
    }
}

impl ClassElement for Annotation {
    type Raw = raw_attributes::Annotation;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            type_index,
            element_value_pairs,
        } = raw;
        let annotation_type = ctx.constant_pool.get_str(type_index)?.parse()?;
        let element_value_pairs = element_value_pairs
            .into_iter()
            .map(|(name_idx, raw_value)| {
                let element_name = ctx.constant_pool.get_str(name_idx)?;
                let element_value = ElementValue::from_raw(raw_value, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Annotation {
            annotation_type,
            element_value_pairs,
        })
    }
}
impl ClassElement for TypeAnnotation {
    type Raw = raw_attributes::TypeAnnotation;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
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
        let annotation_type = ctx.constant_pool.get_str(type_index)?.parse()?;
        let element_value_pairs = element_value_pairs
            .into_iter()
            .map(|(name_idx, value)| {
                let element_name = ctx.constant_pool.get_str(name_idx)?;
                let element_value = ClassElement::from_raw(value, ctx)?;
                Ok((element_name.to_owned(), element_value))
            })
            .collect::<Result<_, Error>>()?;
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

    fn from_raw(raw: Self::Raw, _ctx: &Context) -> Result<Self, Error> {
        match raw {
            Self::Raw::TypeParameter { index } => Ok(Self::TypeParameter { index }),
            Self::Raw::SuperType { index } => Ok(Self::SuperType { index }),
            Self::Raw::TypeParameterBound {
                type_parameter: type_parameter_index,
                bound_index,
            } => Ok(Self::TypeParameterBound {
                type_parameter_index,
                bound_index,
            }),
            Self::Raw::Empty => Ok(Self::Empty),
            Self::Raw::FormalParameter { index } => Ok(Self::FormalParameter { index }),
            Self::Raw::Throws { index } => Ok(Self::Throws { index }),
            Self::Raw::LocalVariable(table) => Ok(Self::LocalVar(
                table
                    .into_iter()
                    .map(|(start, len, index)| {
                        let effective_range = start..(start + len)?;
                        Ok(LocalVariableId {
                            effective_range,
                            index,
                        })
                    })
                    .collect::<Result<_, Error>>()?,
            )),
            Self::Raw::Catch {
                exception_table_index,
            } => Ok(Self::Catch {
                index: exception_table_index,
            }),
            Self::Raw::Offset(offset) => Ok(Self::Offset(offset)),
            Self::Raw::TypeArgument { offset, index } => Ok(Self::TypeArgument { offset, index }),
        }
    }
}

impl ClassElement for ElementValue {
    type Raw = raw_attributes::ElementValueInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let cp = &ctx.constant_pool;
        match raw {
            Self::Raw::Const(b'B', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Integer(_) => Ok(Self::Primitive(PrimitiveType::Byte, it)),
                _ => Err(Error::Other("Expected integer constant value")),
            },
            Self::Raw::Const(b'C', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Integer(_) => Ok(Self::Primitive(PrimitiveType::Char, it)),
                _ => Err(Error::Other("Expected integer constant value")),
            },
            Self::Raw::Const(b'I', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Integer(_) => Ok(Self::Primitive(PrimitiveType::Int, it)),
                _ => Err(Error::Other("Expected integer constant value")),
            },
            Self::Raw::Const(b'S', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Integer(_) => Ok(Self::Primitive(PrimitiveType::Short, it)),
                _ => Err(Error::Other("Expected integer constant value")),
            },
            Self::Raw::Const(b'Z', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Integer(_) => Ok(Self::Primitive(PrimitiveType::Boolean, it)),
                _ => Err(Error::Other("Expected integer constant value")),
            },
            Self::Raw::Const(b'D', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Double(_) => Ok(Self::Primitive(PrimitiveType::Double, it)),
                _ => Err(Error::Other("Expected double constant value")),
            },
            Self::Raw::Const(b'F', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Float(_) => Ok(Self::Primitive(PrimitiveType::Float, it)),
                _ => Err(Error::Other("Expected float constant value")),
            },
            Self::Raw::Const(b'J', idx) => match cp.get_constant_value(idx)? {
                it @ ConstantValue::Long(_) => Ok(Self::Primitive(PrimitiveType::Long, it)),
                _ => Err(Error::Other("Expected long constant value")),
            },
            Self::Raw::Const(b's', idx) => match cp.get_entry(idx)? {
                constant_pool::Entry::Utf8(s) => {
                    Ok(Self::String(ConstantValue::String(s.to_owned())))
                }
                _ => Err(Error::Other("Expected string constant value")),
            },
            Self::Raw::Const(_, _) => Err(Error::Other("Invalid constant value tag")),
            Self::Raw::Enum {
                type_name_index,
                const_name_index,
            } => {
                let enum_type = cp.get_str(type_name_index)?.to_owned();
                let const_name = cp.get_str(const_name_index)?.to_owned();
                Ok(Self::EnumConstant {
                    enum_type_name: enum_type,
                    const_name,
                })
            }
            Self::Raw::ClassInfo(idx) => {
                let return_descriptor = cp.get_str(idx)?.parse()?;
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
