use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
};

use itertools::Itertools;

use crate::{
    elements::{
        instruction::{Instruction, ProgramCounter},
        references::FieldReference,
        ConstantValue,
    },
    types::FieldType,
};

#[derive(Debug)]
pub enum MokaInstruction {
    Nop,
    Assignment {
        lhs: Identifier,
        rhs: Expression,
    },
    SideEffect {
        rhs: Expression,
    },
    Jump {
        target: ProgramCounter,
    },
    UnitaryConditionalJump {
        condition: ValueRef,
        target: ProgramCounter,
        instruction: Instruction,
    },
    BinaryConditionalJump {
        condition: [ValueRef; 2],
        target: ProgramCounter,
        instruction: Instruction,
    },
    Switch {
        condition: ValueRef,
        instruction: Instruction,
    },
    Return {
        value: Option<ValueRef>,
    },
    SubRoutineRet {
        target: ValueRef,
    },
}

impl Display for MokaInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nop => write!(f, "nop"),
            Self::Assignment { lhs, rhs } => write!(f, "{} = {}", lhs, rhs),
            Self::SideEffect { rhs: op } => write!(f, "{}", op),
            Self::Jump { target } => write!(f, "goto {}", target),
            Self::UnitaryConditionalJump {
                condition,
                target,
                instruction,
            } => write!(f, "{}({}) goto {}", instruction.name(), condition, target),
            Self::BinaryConditionalJump {
                condition,
                target,
                instruction,
            } => {
                write!(
                    f,
                    "{}({}, {}) goto {}",
                    instruction.name(),
                    condition[0],
                    condition[1],
                    target
                )
            }
            Self::Switch {
                condition,
                instruction,
            } => write!(f, "{}({})", instruction.name(), condition),
            Self::Return { value } => {
                if let Some(value) = value {
                    write!(f, "return {}", value)
                } else {
                    write!(f, "return")
                }
            }
            Self::SubRoutineRet { target } => write!(f, "ret {}", target),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueRef {
    Def(Identifier),
    Phi(HashSet<Identifier>),
}

impl Display for ValueRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Def(id) => write!(f, "{}", id),
            Self::Phi(ids) => {
                write!(
                    f,
                    "Phi({})",
                    ids.iter().map(|id| format!("{}", id)).join(", ")
                )
            }
        }
    }
}

impl From<Identifier> for ValueRef {
    fn from(value: Identifier) -> Self {
        Self::Def(value)
    }
}

#[derive(Debug)]
pub enum Expression {
    Const(ConstantValue),
    ReturnAddress(ProgramCounter),
    Field(FieldAccess),
    Array(ArrayOperation),
    Insn {
        instruction: Instruction,
        arguments: Vec<ValueRef>,
    },
}

impl Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Expression::*;
        match self {
            Const(c) => write!(f, "{:?}", c),
            ReturnAddress(pc) => write!(f, "{:?}", pc),
            Field(field_op) => field_op.fmt(f),
            Array(array_op) => array_op.fmt(f),
            Insn {
                instruction,
                arguments,
            } => {
                write!(
                    f,
                    "{}({})",
                    instruction.name(),
                    arguments.iter().map(|it| it.to_string()).join(", ")
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum ArrayOperation {
    New {
        element_type: FieldType,
        length: ValueRef,
    },
    NewMD {
        element_type: FieldType,
        dimensions: Vec<ValueRef>,
    },
    Read {
        array_ref: ValueRef,
        index: ValueRef,
    },
    Write {
        array_ref: ValueRef,
        index: ValueRef,
        value: ValueRef,
    },
    Length {
        array_ref: ValueRef,
    },
}

impl Display for ArrayOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use ArrayOperation::*;
        match self {
            New {
                element_type,
                length,
            } => write!(f, "new {}[{}]", element_type.descriptor_string(), length),
            NewMD {
                element_type,
                dimensions,
            } => {
                write!(
                    f,
                    "new {}[{}]",
                    element_type.descriptor_string(),
                    dimensions.iter().map(|it| it.to_string()).join(", ")
                )
            }
            Read { array_ref, index } => write!(f, "{}[{}]", array_ref, index),
            Write {
                array_ref,
                index,
                value,
            } => write!(f, "{}[{}] = {}", array_ref, index, value),
            Length { array_ref } => write!(f, "array_len({})", array_ref),
        }
    }
}

#[derive(Debug)]
pub enum FieldAccess {
    ReadStatic {
        field: FieldReference,
    },
    WriteStatic {
        field: FieldReference,
        value: ValueRef,
    },
    ReadInstance {
        object_ref: ValueRef,
        field: FieldReference,
    },
    WriteInstance {
        object_ref: ValueRef,
        field: FieldReference,
        value: ValueRef,
    },
}

impl Display for FieldAccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use FieldAccess::*;
        match self {
            ReadStatic { field } => write!(f, "{}", field),
            WriteStatic { field, value } => write!(f, "{} = {}", field, value),
            ReadInstance { object_ref, field } => write!(f, "{}.{}", object_ref, field),
            WriteInstance {
                object_ref,
                field,
                value,
            } => write!(f, "{}.{} = {}", object_ref, field, value),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Identifier {
    Val(u16),
    This,
    Arg(u16),
    CaughtException,
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Identifier::*;
        match self {
            Val(idx) => write!(f, "v{}", idx),
            This => write!(f, "this"),
            Arg(idx) => write!(f, "arg{}", idx),
            CaughtException => write!(f, "exception"),
        }
    }
}
