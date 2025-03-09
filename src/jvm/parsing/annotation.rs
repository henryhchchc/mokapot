use itertools::Itertools;

use crate::{
    jvm::{
        Annotation, ConstantValue, TypeAnnotation,
        annotation::{ElementValue, TargetInfo, TypePathElement},
        class::{ConstantPool, constant_pool},
        code::LocalVariableId,
    },
    types::field_type::PrimitiveType,
};

use super::{Context, Error, ToWriterError, jvm_element_parser::ClassElement, raw_attributes};

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

    fn into_raw(self, _cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        match self {
            Self::Array => Ok((0, 0)),
            Self::Nested => Ok((1, 0)),
            Self::Bound => Ok((2, 0)),
            Self::TypeArgument(idx) => Ok((3, idx)),
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let type_index = cp.put_string(self.annotation_type.to_string())?;
        let element_value_pairs = self
            .element_value_pairs
            .into_iter()
            .map(|(name, value)| -> Result<_, ToWriterError> {
                let name_index = cp.put_string(name)?;
                let raw_value = value.into_raw(cp)?;
                Ok((name_index, raw_value))
            })
            .try_collect()?;
        Ok(Self::Raw {
            type_index,
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let target_info = self.target_info.into_raw(cp)?;
        let target_path = self
            .target_path
            .into_iter()
            .map(|it| it.into_raw(cp))
            .try_collect()?;
        let type_index = cp.put_string(self.annotation_type.to_string())?;
        let element_value_pairs = self
            .element_value_pairs
            .into_iter()
            .map(|(name, value)| -> Result<_, ToWriterError> {
                let name_index = cp.put_string(name)?;
                let value = value.into_raw(cp)?;
                Ok((name_index, value))
            })
            .try_collect()?;
        Ok(Self::Raw {
            target_info,
            target_path,
            type_index,
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

    fn into_raw(self, _cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let raw = match self {
            Self::TypeParameter { index } => Self::Raw::TypeParameter { index },
            Self::SuperType { index } => Self::Raw::SuperType { index },
            Self::TypeParameterBound {
                type_parameter_index: type_parameter,
                bound_index,
            } => Self::Raw::TypeParameterBound {
                type_parameter,
                bound_index,
            },
            Self::Empty => Self::Raw::Empty,
            Self::FormalParameter { index } => Self::Raw::FormalParameter { index },
            Self::Throws { index } => Self::Raw::Throws { index },
            Self::LocalVar(table) => Self::Raw::LocalVariable(
                table
                    .into_iter()
                    .map(|var| {
                        let LocalVariableId {
                            effective_range,
                            index,
                        } = var;
                        let start = effective_range.start;
                        let len = u16::from(effective_range.end) - u16::from(effective_range.start);
                        (start, index, len)
                    })
                    .collect(),
            ),
            Self::Catch { index } => Self::Raw::Catch {
                exception_table_index: index,
            },
            Self::Offset(offset) => Self::Raw::Offset(offset),
            Self::TypeArgument { offset, index } => Self::Raw::TypeArgument { offset, index },
        };
        Ok(raw)
    }
}

impl ClassElement for ElementValue {
    type Raw = raw_attributes::ElementValueInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
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
                    _ => Err(Error::Other("Constant value type mismatch")),
                }
            }
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let raw = match self {
            ElementValue::Primitive(primitive_type, constant_value) => {
                let tag = match primitive_type {
                    PrimitiveType::Byte => b'B',
                    PrimitiveType::Char => b'C',
                    PrimitiveType::Double => b'D',
                    PrimitiveType::Float => b'F',
                    PrimitiveType::Int => b'I',
                    PrimitiveType::Long => b'J',
                    PrimitiveType::Short => b'S',
                    PrimitiveType::Boolean => b'Z',
                };
                let value_idx = cp.put_constant_value(constant_value)?;
                Self::Raw::Const(tag, value_idx)
            }
            ElementValue::String(constant_value) => {
                let value_idx = cp.put_constant_value(constant_value)?;
                Self::Raw::Const(b'S', value_idx)
            }
            ElementValue::EnumConstant {
                enum_type_name,
                const_name,
            } => {
                let type_name_index = cp.put_string(enum_type_name)?;
                let const_name_index = cp.put_string(const_name)?;
                Self::Raw::Enum {
                    type_name_index,
                    const_name_index,
                }
            }
            ElementValue::Class { return_descriptor } => {
                let class_name_index = cp.put_string(return_descriptor.descriptor())?;
                Self::Raw::ClassInfo(class_name_index)
            }
            ElementValue::AnnotationInterface(annotation) => {
                Self::Raw::Annotation(annotation.into_raw(cp)?)
            }
            ElementValue::Array(elements) => Self::Raw::Array(
                elements
                    .into_iter()
                    .map(|it| it.into_raw(cp))
                    .try_collect()?,
            ),
        };
        Ok(raw)
    }
}
