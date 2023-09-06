use core::str;
use std::{collections::HashMap, str::Chars};

use bitflags::bitflags;
use itertools::Itertools;

use super::{
    annotation::{Annotation, ElementValue, TypeAnnotation},
    field::{FieldType, PrimitiveType},
    instruction::{Instruction, ProgramCounter},
    parsing::error::ClassFileParsingError,
    references::ClassReference,
};

#[derive(Debug)]
pub struct Method {
    pub access_flags: MethodAccessFlags,
    pub name: String,
    pub descriptor: MethodDescriptor,
    pub body: Option<MethodBody>,
    pub excaptions: Vec<ClassReference>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_visible_parameter_annotations: Vec<Vec<Annotation>>,
    pub runtime_invisible_parameter_annotations: Vec<Vec<Annotation>>,
    pub annotation_default: Option<ElementValue>,
    pub parameters: Vec<MethodParameter>,
    pub is_synthetic: bool,
    pub is_deprecated: bool,
    pub signature: Option<String>,
}

#[derive(Debug)]
pub struct MethodParameter {
    pub name: String,
    pub access_flags: MethodParameterAccessFlags,
}

#[derive(Debug)]
pub struct MethodBody {
    pub max_stack: u16,
    pub max_locals: u16,
    pub instructions: HashMap<ProgramCounter, Instruction>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub line_number_table: Option<Vec<LineNumberTableEntry>>,
    pub local_variable_table: Option<LocalVariableTable>,
    pub stack_map_table: Option<Vec<StackMapFrame>>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

#[derive(Debug)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: Option<ClassReference>,
}

#[derive(Debug)]
pub struct LineNumberTableEntry {
    pub start_pc: u16,
    pub line_number: u16,
}

#[derive(Debug)]
pub struct LocalVariableTable {
    entries: HashMap<LocalVariableKey, LocalVariableTableEntry>,
}

impl LocalVariableTable {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn merge_desc_attr(&mut self, attrs: Vec<LocalVariableDescAttr>) {
        for LocalVariableDescAttr {
            key,
            name,
            field_type,
        } in attrs.into_iter()
        {
            let entry = self.entries.entry(key).or_default();
            entry.name = name;
            entry.var_type = Some(field_type);
        }
    }
    pub(crate) fn merge_type_attr(&mut self, attrs: Vec<LocalVariableTypeAttr>) {
        for LocalVariableTypeAttr {
            key,
            name,
            signature,
        } in attrs.into_iter()
        {
            let entry = self.entries.entry(key).or_default();
            entry.name = name;
            entry.signature = Some(signature);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Default)]
pub struct LocalVariableKey {
    pub start_pc: u16,
    pub length: u16,
    pub index: u16,
}

#[derive(Debug, Default)]
pub struct LocalVariableTableEntry {
    pub key: LocalVariableKey,
    pub name: String,
    pub var_type: Option<FieldType>,
    pub signature: Option<String>,
}

// impl Default for Local

#[derive(Debug)]
pub(crate) struct LocalVariableDescAttr {
    pub key: LocalVariableKey,
    pub name: String,
    pub field_type: FieldType,
}

#[derive(Debug)]
pub(crate) struct LocalVariableTypeAttr {
    pub key: LocalVariableKey,
    pub name: String,
    pub signature: String,
}

#[derive(Debug)]
pub enum VerificationTypeInfo {
    TopVariable,
    IntegerVariable,
    FloatVariable,
    NullVariable,
    UninitializedThisVariable,
    ObjectVariable(ClassReference),
    UninitializedVariable { offset: u16 },
    LongVariable,
    DoubleVariable,
}

#[derive(Debug)]
pub enum StackMapFrame {
    SameFrame {
        offset_delta: u16,
    },
    SameLocals1StackItemFrame(VerificationTypeInfo),
    Semantics1StackItemFrameExtended(u16, VerificationTypeInfo),
    ChopFrame {
        chop_count: u8,
        offset_delta: u16,
    },
    SameFrameExtended {
        offset_delta: u16,
    },
    AppendFrame {
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
    },
    FullFrame {
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
        stack: Vec<VerificationTypeInfo>,
    },
}

bitflags! {
    /// Access flags for a [`Method`].
    #[derive(Debug, PartialEq, Eq)]
    pub struct MethodAccessFlags: u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC = 0x0001;
        /// Declared `private`; accessible only within the defining class and other classes belonging to the same nest.
        const PRIVATE = 0x0002;
        /// Declared `protected`; may be accessed within subclasses.
        const PROTECTED = 0x0004;
        /// Declared `static`.
        const STATIC = 0x0008;
        /// Declared `final`; must not be overridden.
        const FINAL = 0x0010;
        /// Declared `synchronized`; invocation is wrapped by a monitor use.
        const SYNCHRONIZED = 0x0020;
        /// A bridge method, generated by the compiler.
        const BRIDGE = 0x0040;
        /// Declared with variable number of arguments.
        const VARARGS = 0x0080;
        /// Declared `native`; implemented in a language other than Java.
        const NATIVE = 0x0100;
        /// Declared `abstract`; no implementation is provided.
        const ABSTRACT = 0x0400;
        /// In a `class` file whose major version is at least 46 and at most 60; Declared `strictfp`.
        const STRICT = 0x0800;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
    }
}

bitflags! {
    /// The access flags for a method parameter.
    #[derive(Debug, PartialEq, Eq)]
    pub struct MethodParameterAccessFlags: u16 {
        /// Declared `final`; may not be assigned to after initialization.
        const FINAL = 0x0010;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
        /// Declared as either `mandated` or `optional`.
        const MANDATED = 0x8000;
    }
}

/// The descriptor of a method.
/// Consists of the parameters types and the return type.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MethodDescriptor {
    /// The type of the parameters.
    pub parameters_types: Vec<FieldType>,
    /// The return type.
    pub return_type: ReturnType,
}

/// Denotes the return type of a method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReturnType {
    /// The method returns a specific type.
    Some(FieldType),
    /// The return type of the method is `void`.
    Void,
}

impl MethodDescriptor {
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        result.push('(');
        for param in &self.parameters_types {
            result.push_str(&param.descriptor_string());
        }
        result.push(')');
        result.push_str(&self.return_type.descriptor_string());
        result
    }

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
        remaining: &mut Chars,
    ) -> Result<FieldType, ClassFileParsingError> {
        if let Ok(p) = PrimitiveType::new(&prefix) {
            return Ok(FieldType::Base(p));
        }
        let build_err = |rem: &Chars| {
            ClassFileParsingError::InvalidDescriptor(format!("{}{}", prefix, rem.as_str()))
        };
        match prefix {
            'L' => {
                let binary_name: String = remaining.take_while_ref(|c| *c != ';').collect();
                match remaining.next() {
                    Some(';') => Ok(FieldType::Object(ClassReference { binary_name })),
                    _ => Err(build_err(remaining)),
                }
            }
            '[' => {
                let next_prefix = remaining.next().ok_or_else(|| build_err(remaining))?;
                Self::parse_single_param(next_prefix, remaining).map(|p| p.make_array_type())
            }
            _ => Err(build_err(remaining)),
        }
    }

    /// Creates a new [`MethodDescriptor`] from the [descriptor].
    pub fn new(descriptor: &str) -> Result<Self, ClassFileParsingError> {
        let mut chars = descriptor.chars();
        let mut parameters_types = Vec::new();
        let return_type = loop {
            match chars.next() {
                Some('(') => {}
                Some(')') => break ReturnType::new(chars.as_str())?,
                Some(c) => {
                    let param = Self::parse_single_param(c, &mut chars)?;
                    parameters_types.push(param);
                }
                None => Err(ClassFileParsingError::InvalidDescriptor(
                    descriptor.to_string(),
                ))?,
            }
        };
        Ok(Self {
            parameters_types,
            return_type,
        })
    }
}

impl ReturnType {
    pub fn new(descriptor: &str) -> Result<Self, ClassFileParsingError> {
        if descriptor == "V" {
            Ok(ReturnType::Void)
        } else {
            FieldType::new(descriptor).map(ReturnType::Some)
        }
    }

    fn descriptor_string(&self) -> String {
        match self {
            ReturnType::Some(it) => it.descriptor_string(),
            ReturnType::Void => "V".to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::elements::{
        field::{FieldType, PrimitiveType::*},
        method::ReturnType,
        references::ClassReference,
    };

    use super::MethodDescriptor;

    #[test]
    fn single_param() {
        let descriptor = "(I)V";
        let method_descriptor =
            MethodDescriptor::new(descriptor).expect("Failed to parse method descriptor");
        assert_eq!(method_descriptor.return_type, ReturnType::Void);
        assert_eq!(
            method_descriptor.parameters_types,
            vec![FieldType::Base(Int)]
        );
    }

    #[test]
    fn param_complex() {
        let descriptor = "(I[JLjava/lang/String;J)I";
        let method_descriptor =
            MethodDescriptor::new(descriptor).expect("Failed to parse method descriptor");
        let string_type = FieldType::Object(ClassReference {
            binary_name: "java/lang/String".to_string(),
        });
        assert_eq!(
            method_descriptor.return_type,
            ReturnType::Some(FieldType::Base(Int))
        );
        assert_eq!(
            method_descriptor.parameters_types,
            vec![
                FieldType::Base(Int),
                FieldType::Base(Long).make_array_type(),
                string_type,
                FieldType::Base(Long),
            ]
        );
    }

    #[test]
    fn too_many_return_type() {
        let descriptor = "(I)VJ";
        let method_descriptor = MethodDescriptor::new(descriptor);
        assert!(method_descriptor.is_err());
    }
}
