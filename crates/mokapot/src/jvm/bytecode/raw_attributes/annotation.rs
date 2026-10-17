//! Raw attribute types for annotation-related attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::{
    FromBytecode, ParseError,
    decoder::{borrow_slice, parse_repeated, read_u8, read_u16},
    fixed_layout::FixedLayout,
};
use crate::{intrinsics::see_jvm_spec, jvm::code::ProgramCounter};

#[doc = see_jvm_spec!(4, 7, 16)]
pub struct Annotation {
    pub type_index: u16,
    pub element_value_pairs: Box<[(u16, ElementValueInfo)]>,
}

impl<'a> FromBytecode<'a> for Annotation {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let type_index = read_u16(bytes)?;
        let num_element_value_pairs: u16 = read_u16(bytes)?;
        let element_value_pairs =
            parse_repeated(bytes, usize::from(num_element_value_pairs), |bytes| {
                let element_name_index = read_u16(bytes)?;
                let element_value = ElementValueInfo::from_bytes(bytes)?;
                Ok((element_name_index, element_value))
            })?;
        Ok(Self {
            type_index,
            element_value_pairs,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 16, 1)]
pub enum ElementValueInfo {
    Const(u8, u16),
    Enum {
        type_name_index: u16,
        const_name_index: u16,
    },
    ClassInfo(u16),
    Annotation(Annotation),
    Array(Box<[ElementValueInfo]>),
}

impl<'a> FromBytecode<'a> for ElementValueInfo {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let tag: u8 = read_u8(bytes)?;
        match tag {
            tag @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's') => {
                Ok(Self::Const(tag, read_u16(bytes)?))
            }
            b'e' => Ok(Self::Enum {
                type_name_index: read_u16(bytes)?,
                const_name_index: read_u16(bytes)?,
            }),
            b'c' => Ok(Self::ClassInfo(read_u16(bytes)?)),
            b'@' => Ok(Self::Annotation(Annotation::from_bytes(bytes)?)),
            b'[' => {
                let num_values: u16 = read_u16(bytes)?;
                let values = parse_repeated(bytes, usize::from(num_values), Self::from_bytes)?;
                Ok(Self::Array(values))
            }
            _ => Err(ParseError::malform(format!(
                "Unknown element value tag: {tag}"
            ))),
        }
    }
}

#[doc = see_jvm_spec!(4, 7, 20)]
pub struct TypeAnnotation<'a> {
    pub target_info: TargetInfo<'a>,
    pub target_path: &'a [[u8; 2]],
    pub type_index: u16,
    pub element_value_pairs: Box<[(u16, ElementValueInfo)]>,
}

impl<'a> FromBytecode<'a> for TypeAnnotation<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let target_info = TargetInfo::from_bytes(bytes)?;
        let target_path_length: u8 = read_u8(bytes)?;
        let target_path = borrow_slice(bytes, usize::from(target_path_length))?;
        let type_index = read_u16(bytes)?;
        let num_element_value_pairs: u16 = read_u16(bytes)?;
        let element_value_pairs =
            parse_repeated(bytes, usize::from(num_element_value_pairs), |bytes| {
                let element_name_index = read_u16(bytes)?;
                let element_value = ElementValueInfo::from_bytes(bytes)?;
                Ok((element_name_index, element_value))
            })?;
        Ok(Self {
            target_info,
            target_path,
            type_index,
            element_value_pairs,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 20, 1)]
#[repr(u8)]
pub enum TargetInfo<'a> {
    TypeParameterOfClass {
        type_parameter_index: u8,
    } = 0x00,
    TypeParameterOfMethod {
        type_parameter_index: u8,
    } = 0x01,
    SuperType {
        supertype_index: u16,
    } = 0x10,
    TypeParameterBoundOfClass {
        type_parameter_index: u8,
        bound_index: u8,
    } = 0x11,
    TypeParameterBoundOfMethod {
        type_parameter_index: u8,
        bound_index: u8,
    } = 0x12,
    Field = 0x13,
    MethodReturn = 0x14,
    Receiver = 0x15,
    FormalParameter {
        formal_parameter_index: u8,
    } = 0x16,
    Throws {
        throws_type_index: u16,
    } = 0x17,
    LocalVariable(&'a [LocalVariableTarget]) = 0x40,
    ResourceVariable(&'a [LocalVariableTarget]) = 0x41,
    Catch {
        exception_table_index: u16,
    } = 0x42,
    InstanceOf {
        offset: u16,
    } = 0x43,
    New {
        offset: u16,
    } = 0x44,
    ConstructorReference {
        offset: u16,
    } = 0x45,
    MethodReference {
        offset: u16,
    } = 0x46,
    TypeInCast {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x47,
    TypeArgumentInConstructor {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x48,
    TypeArgumentInCall {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x49,
    TypeArgumentInConstructorReference {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x4A,
    TypeArgumentInMethodReference {
        offset: ProgramCounter,
        type_argument_index: u8,
    } = 0x4B,
}

impl<'a> FromBytecode<'a> for TargetInfo<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let target_type: u8 = read_u8(bytes)?;
        let target_info = match target_type {
            0x00 => Self::TypeParameterOfClass {
                type_parameter_index: read_u8(bytes)?,
            },
            0x01 => Self::TypeParameterOfMethod {
                type_parameter_index: read_u8(bytes)?,
            },
            0x10 => Self::SuperType {
                supertype_index: read_u16(bytes)?,
            },
            0x11 => Self::TypeParameterBoundOfClass {
                type_parameter_index: read_u8(bytes)?,
                bound_index: read_u8(bytes)?,
            },
            0x12 => Self::TypeParameterBoundOfMethod {
                type_parameter_index: read_u8(bytes)?,
                bound_index: read_u8(bytes)?,
            },
            0x13 => Self::Field,
            0x14 => Self::MethodReturn,
            0x15 => Self::Receiver,
            0x16 => Self::FormalParameter {
                formal_parameter_index: read_u8(bytes)?,
            },
            0x17 => Self::Throws {
                throws_type_index: read_u16(bytes)?,
            },
            0x40 => {
                let table_length: u16 = read_u16(bytes)?;
                let table = borrow_slice(bytes, usize::from(table_length))?;
                Self::LocalVariable(table)
            }
            0x41 => {
                let table_length: u16 = read_u16(bytes)?;
                let table = borrow_slice(bytes, usize::from(table_length))?;
                Self::ResourceVariable(table)
            }
            0x42 => Self::Catch {
                exception_table_index: read_u16(bytes)?,
            },
            0x43 => Self::InstanceOf {
                offset: read_u16(bytes)?,
            },
            0x44 => Self::New {
                offset: read_u16(bytes)?,
            },
            0x45 => Self::ConstructorReference {
                offset: read_u16(bytes)?,
            },
            0x46 => Self::MethodReference {
                offset: read_u16(bytes)?,
            },
            0x47 => Self::TypeInCast {
                offset: read_u16(bytes)?.into(),
                type_argument_index: read_u8(bytes)?,
            },
            0x48 => Self::TypeArgumentInConstructor {
                offset: read_u16(bytes)?.into(),
                type_argument_index: read_u8(bytes)?,
            },
            0x49 => Self::TypeArgumentInCall {
                offset: read_u16(bytes)?.into(),
                type_argument_index: read_u8(bytes)?,
            },
            0x4a => Self::TypeArgumentInConstructorReference {
                offset: read_u16(bytes)?.into(),
                type_argument_index: read_u8(bytes)?,
            },
            0x4b => Self::TypeArgumentInMethodReference {
                offset: read_u16(bytes)?.into(),
                type_argument_index: read_u8(bytes)?,
            },
            unexpected => Err(ParseError::malform(format!(
                "Invalid target type: {unexpected:x}"
            )))?,
        };
        Ok(target_info)
    }
}

#[derive(Debug, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct LocalVariableTarget {
    pub start_pc: U16,
    pub length: U16,
    pub index: U16,
}

impl FixedLayout for LocalVariableTarget {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_tables_borrow_their_input() {
        let bytes = [0xff, 0x40, 0, 1, 0, 2, 0, 3, 0, 4, 1, 0, 0, 0, 1, 0, 0];
        let mut remaining = &bytes[1..];
        let annotation = TypeAnnotation::from_bytes(&mut remaining).unwrap();
        assert_eq!(remaining, b"");
        let TargetInfo::LocalVariable(table) = annotation.target_info else {
            panic!("expected a local-variable target");
        };
        assert_eq!(table[0].start_pc.get(), 2);
        assert_eq!(table[0].length.get(), 3);
        assert_eq!(table[0].index.get(), 4);
        assert!(std::ptr::eq(
            table.as_ptr().cast::<u8>(),
            bytes[4..].as_ptr()
        ));
        assert_eq!(annotation.target_path, [[0, 0]]);
        assert!(std::ptr::eq(
            annotation.target_path.as_ptr().cast::<u8>(),
            bytes[11..].as_ptr()
        ));
    }
}
