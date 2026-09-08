use std::fmt;

use itertools::Itertools as _;

use super::{Instruction, WideInstruction};

impl fmt::Display for WideInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ILoad(index) => write!(f, "iload {index}"),
            Self::LLoad(index) => write!(f, "lload {index}"),
            Self::FLoad(index) => write!(f, "fload {index}"),
            Self::DLoad(index) => write!(f, "dload {index}"),
            Self::ALoad(index) => write!(f, "aload {index}"),
            Self::IStore(index) => write!(f, "istore {index}"),
            Self::LStore(index) => write!(f, "lstore {index}"),
            Self::FStore(index) => write!(f, "fstore {index}"),
            Self::DStore(index) => write!(f, "dstore {index}"),
            Self::AStore(index) => write!(f, "astore {index}"),
            Self::IInc(index, value) => write!(f, "iinc {index} {value}"),
            Self::Ret(index) => write!(f, "ret {index}"),
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[allow(clippy::enum_glob_use, reason = "JVM instructions are numerous")]
        use Instruction::*;

        match self {
            Nop | AConstNull | IConstM1 | IConst0 | IConst1 | IConst2 | IConst3 | IConst4
            | IConst5 | LConst0 | LConst1 | FConst0 | FConst1 | FConst2 | DConst0 | DConst1
            | ILoad0 | ILoad1 | ILoad2 | ILoad3 | LLoad0 | LLoad1 | LLoad2 | LLoad3 | FLoad0
            | FLoad1 | FLoad2 | FLoad3 | DLoad0 | DLoad1 | DLoad2 | DLoad3 | ALoad0 | ALoad1
            | ALoad2 | ALoad3 | IALoad | LALoad | FALoad | DALoad | AALoad | BALoad | CALoad
            | SALoad | IStore0 | IStore1 | IStore2 | IStore3 | LStore0 | LStore1 | LStore2
            | LStore3 | FStore0 | FStore1 | FStore2 | FStore3 | DStore0 | DStore1 | DStore2
            | DStore3 | AStore0 | AStore1 | AStore2 | AStore3 | IAStore | LAStore | FAStore
            | DAStore | AAStore | BAStore | CAStore | SAStore | Pop | Pop2 | Dup | DupX1
            | DupX2 | Dup2 | Dup2X1 | Dup2X2 | Swap | IAdd | LAdd | FAdd | DAdd | ISub | LSub
            | FSub | DSub | IMul | LMul | FMul | DMul | IDiv | LDiv | FDiv | DDiv | IRem | LRem
            | FRem | DRem | INeg | LNeg | FNeg | DNeg | IShl | LShl | IShr | LShr | IUShr
            | LUShr | IAnd | LAnd | IOr | LOr | IXor | LXor | I2L | I2F | I2D | L2I | L2F | L2D
            | F2I | F2L | F2D | D2I | D2L | D2F | I2B | I2C | I2S | LCmp | FCmpL | FCmpG
            | DCmpL | DCmpG | IReturn | LReturn | FReturn | DReturn | AReturn | Return
            | ArrayLength | AThrow | MonitorEnter | MonitorExit | Breakpoint | ImpDep1
            | ImpDep2 => write!(f, "{}", self.name()),
            BiPush(value) => write!(f, "{} {value}", self.name()),
            SiPush(value) => write!(f, "{} {value}", self.name()),
            ILoad(index) | LLoad(index) | FLoad(index) | DLoad(index) | ALoad(index)
            | IStore(index) | LStore(index) | FStore(index) | DStore(index) | AStore(index)
            | Ret(index) => write!(f, "{} {index}", self.name()),
            IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
            | IfLe(target) | IfICmpEq(target) | IfICmpNe(target) | IfICmpLt(target)
            | IfICmpGe(target) | IfICmpGt(target) | IfICmpLe(target) | IfACmpEq(target)
            | IfACmpNe(target) | Goto(target) | Jsr(target) | IfNull(target)
            | IfNonNull(target) | GotoW(target) | JsrW(target) => {
                write!(f, "{} {target}", self.name())
            }
            GetStatic(reference) | PutStatic(reference) | GetField(reference)
            | PutField(reference) => write!(f, "{} {reference}", self.name()),
            InvokeVirtual(reference) | InvokeSpecial(reference) | InvokeStatic(reference) => {
                write!(f, "{} {reference}", self.name())
            }
            Ldc(constant) | LdcW(constant) | Ldc2W(constant) => {
                write!(f, "{} {constant}", self.name())
            }
            New(reference) => write!(f, "{} {reference}", self.name()),
            ANewArray(field_type) => write!(f, "{} {field_type}", self.name()),
            NewArray(primitive_type) => write!(f, "{} {primitive_type}", self.name()),
            CheckCast(field_type) | InstanceOf(field_type) => {
                write!(f, "{} {field_type}", self.name())
            }
            IInc(index, value) => write!(f, "{} {index} {value}", self.name()),
            InvokeInterface(reference, count) => {
                write!(f, "{} {reference} count {count}", self.name())
            }
            InvokeDynamic {
                bootstrap_method_index,
                name,
                descriptor,
            } => write!(
                f,
                "{} #{bootstrap_method_index} {name} {descriptor}",
                self.name()
            ),
            TableSwitch {
                range,
                jump_targets,
                default,
            } => {
                write!(
                    f,
                    "{} {{\n  range: {}..={}\n  default: {default}\n",
                    self.name(),
                    range.start(),
                    range.end(),
                )?;
                for (value, target) in range.clone().zip_eq(jump_targets) {
                    writeln!(f, "  {value}: {target}")?;
                }
                write!(f, "}}")
            }
            LookupSwitch {
                default,
                match_targets,
            } => {
                write!(f, "{} {{\n  default: {default}\n", self.name())?;
                for (key, target) in match_targets {
                    writeln!(f, "  {key}: {target}")?;
                }
                write!(f, "}}")
            }
            Wide(instruction) => write!(f, "{} {instruction}", self.name()),
            MultiANewArray(field_type, dimensions) => {
                write!(f, "{} {field_type} {dimensions}", self.name())
            }
        }
    }
}
