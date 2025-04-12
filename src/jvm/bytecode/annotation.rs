use itertools::Itertools;

use super::{
    ParsingContext, ParsingError, errors::ToWriterError, jvm_element_parser::ClassElement,
    raw_attributes,
};
use crate::{
    jvm::{
        Annotation, ConstantValue, TypeAnnotation,
        annotation::{
            ElementValue, OffsetOf, TargetInfo, TypeArgumentLocation, TypeParameterLocation,
            TypePathElement, VariableKind,
        },
        class::{ConstantPool, constant_pool},
        code::LocalVariableId,
    },
    types::{Descriptor, field_type::PrimitiveType},
};

impl ClassElement for TypePathElement {
    type Raw = (u8, u8);
    fn from_raw(raw: Self::Raw, _ctx: &ParsingContext) -> Result<Self, ParsingError> {
        let (kind, argument_index) = raw;
        match (kind, argument_index) {
            (0, 0) => Ok(Self::Array),
            (1, 0) => Ok(Self::Nested),
            (2, 0) => Ok(Self::Bound),
            (3, idx) => Ok(Self::TypeArgument(idx)),
            _ => Err(ParsingError::InvalidTypePathKind),
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

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParsingError> {
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
            .collect::<Result<_, ParsingError>>()?;
        Ok(Annotation {
            annotation_type,
            element_value_pairs,
        })
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let type_index = cp.put_string(self.annotation_type.descriptor())?;
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

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParsingError> {
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
            .collect::<Result<_, ParsingError>>()?;
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
        let type_index = cp.put_string(self.annotation_type.descriptor())?;
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

    #[allow(clippy::enum_glob_use)]
    fn from_raw(raw: Self::Raw, _ctx: &ParsingContext) -> Result<Self, ParsingError> {
        use OffsetOf::{InstanceOf, New};
        use TypeArgumentLocation::*;
        use TypeParameterLocation::*;
        use VariableKind::*;

        let lifted = match raw {
            Self::Raw::TypeParameterOfClass { index } => Self::TypeParameter {
                location: Class,
                index,
            },
            Self::Raw::TypeParameterOfMethod { index } => Self::TypeParameter {
                location: Method,
                index,
            },
            Self::Raw::SuperType { index } => Self::SuperType { index },
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
            Self::Raw::TypeOfField => Self::FieldType,
            Self::Raw::Receiver => Self::Receiver,
            Self::Raw::FormalParameter { index } => Self::FormalParameter { index },
            Self::Raw::Throws { index } => Self::Throws { index },
            Self::Raw::LocalVariable(table) => {
                let table = table
                    .into_iter()
                    .map(|(start, len, index)| -> Result<_, ParsingError> {
                        let effective_range = start..(start + len)?;
                        Ok(LocalVariableId {
                            effective_range,
                            index,
                        })
                    })
                    .try_collect()?;
                Self::LocalVar(Local, table)
            }
            Self::Raw::ResourceVariable(table) => {
                let table = table
                    .into_iter()
                    .map(|(start, len, index)| -> Result<_, ParsingError> {
                        let effective_range = start..(start + len)?;
                        Ok(LocalVariableId {
                            effective_range,
                            index,
                        })
                    })
                    .try_collect()?;
                Self::LocalVar(Resource, table)
            }
            Self::Raw::Catch { index } => Self::Catch { index },
            Self::Raw::InstanceOf { offset } => Self::Offset(InstanceOf, offset),
            Self::Raw::New { offset } => Self::Offset(New, offset),
            Self::Raw::NewMethodReference { offset } => {
                Self::Offset(OffsetOf::ConstructorReference, offset)
            }
            Self::Raw::VarMethodReference { offset } => {
                Self::Offset(OffsetOf::MethodReference, offset)
            }
            Self::Raw::TypeInCast { offset, index } => Self::TypeArgument {
                location: Cast,
                offset,
                index,
            },
            Self::Raw::TypeArgumentInConstructor { offset, index } => Self::TypeArgument {
                location: Constructor,
                offset,
                index,
            },
            Self::Raw::TypeArgumentInCall { offset, index } => Self::TypeArgument {
                location: MethodCall,
                offset,
                index,
            },
            Self::Raw::TypeArgumentInConstructorReference { offset, index } => Self::TypeArgument {
                location: ConstructorReference,
                offset,
                index,
            },
            Self::Raw::TypeArgumentInMethodReference { offset, index } => Self::TypeArgument {
                location: MethodReference,
                offset,
                index,
            },
        };
        Ok(lifted)
    }

    fn into_raw(self, _cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let raw = match self {
            TargetInfo::TypeParameter { location, index } => match location {
                TypeParameterLocation::Class => Self::Raw::TypeParameterOfClass { index },
                TypeParameterLocation::Method => Self::Raw::TypeParameterOfMethod { index },
            },
            TargetInfo::SuperType { index } => Self::Raw::SuperType { index },
            TargetInfo::TypeParameterBound {
                location,
                type_parameter_index,
                bound_index,
            } => match location {
                TypeParameterLocation::Class => Self::Raw::TypeParameterBoundOfClass {
                    type_parameter_index,
                    bound_index,
                },
                TypeParameterLocation::Method => Self::Raw::TypeParameterBoundOfMethod {
                    type_parameter_index,
                    bound_index,
                },
            },
            TargetInfo::Field => Self::Raw::Field,
            TargetInfo::FieldType => Self::Raw::TypeOfField,
            TargetInfo::Receiver => Self::Raw::Receiver,
            TargetInfo::FormalParameter { index } => Self::Raw::FormalParameter { index },
            TargetInfo::Throws { index } => Self::Raw::Throws { index },
            TargetInfo::LocalVar(variable_kind, table) => {
                let table = table
                    .into_iter()
                    .map(|entry| {
                        let start_pc = entry.effective_range.start;
                        let length = u16::from(entry.effective_range.end) - u16::from(start_pc);
                        let index = entry.index;
                        (start_pc, length, index)
                    })
                    .collect();
                match variable_kind {
                    VariableKind::Local => Self::Raw::LocalVariable(table),
                    VariableKind::Resource => Self::Raw::ResourceVariable(table),
                }
            }
            TargetInfo::Catch { index } => Self::Raw::Catch { index },
            TargetInfo::Offset(offset_of, offset) => match offset_of {
                OffsetOf::InstanceOf => Self::Raw::InstanceOf { offset },
                OffsetOf::New => Self::Raw::New { offset },
                OffsetOf::MethodReference => Self::Raw::VarMethodReference { offset },
                OffsetOf::ConstructorReference => Self::Raw::NewMethodReference { offset },
            },
            TargetInfo::TypeArgument {
                location,
                offset,
                index,
            } => match location {
                TypeArgumentLocation::Cast => Self::Raw::TypeInCast { offset, index },
                TypeArgumentLocation::Constructor => {
                    Self::Raw::TypeArgumentInConstructor { offset, index }
                }
                TypeArgumentLocation::MethodCall => Self::Raw::TypeArgumentInCall { offset, index },
                TypeArgumentLocation::ConstructorReference => {
                    Self::Raw::TypeArgumentInConstructorReference { offset, index }
                }
                TypeArgumentLocation::MethodReference => {
                    Self::Raw::TypeArgumentInMethodReference { offset, index }
                }
            },
        };
        Ok(raw)
    }
}

impl ClassElement for ElementValue {
    type Raw = raw_attributes::ElementValueInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParsingError> {
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
                    _ => Err(ParsingError::Other("Constant value type mismatch")),
                }
            }
            Self::Raw::Const(b's', idx) => match cp.get_entry(idx)? {
                constant_pool::Entry::Utf8(s) => Ok(Self::String(s.to_owned())),
                _ => Err(ParsingError::Other("Expected string constant value")),
            },
            Self::Raw::Const(_, _) => Err(ParsingError::Other("Invalid constant value tag")),
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
                let tag = match (primitive_type, &constant_value) {
                    (PrimitiveType::Byte, ConstantValue::Integer(_)) => b'B',
                    (PrimitiveType::Char, ConstantValue::Integer(_)) => b'C',
                    (PrimitiveType::Double, ConstantValue::Double(_)) => b'D',
                    (PrimitiveType::Float, ConstantValue::Float(_)) => b'F',
                    (PrimitiveType::Int, ConstantValue::Integer(_)) => b'I',
                    (PrimitiveType::Long, ConstantValue::Long(_)) => b'J',
                    (PrimitiveType::Short, ConstantValue::Integer(_)) => b'S',
                    (PrimitiveType::Boolean, ConstantValue::Integer(_)) => b'Z',
                    _ => return Err(ToWriterError::Other("Constant value type mismatch")),
                };
                let value_idx = cp.put_constant_value(constant_value)?;
                Self::Raw::Const(tag, value_idx)
            }
            ElementValue::String(string) => {
                let entry = constant_pool::Entry::Utf8(string);
                let value_idx = cp.put_entry(entry)?;
                Self::Raw::Const(b's', value_idx)
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
