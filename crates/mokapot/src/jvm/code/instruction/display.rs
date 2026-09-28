use std::fmt;

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

        let name = self.name();
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
            | ImpDep2 => write!(f, "{name}"),
            BiPush(value) => write!(f, "{name} {value}"),
            SiPush(value) => write!(f, "{name} {value}"),
            ILoad(index) | LLoad(index) | FLoad(index) | DLoad(index) | ALoad(index)
            | IStore(index) | LStore(index) | FStore(index) | DStore(index) | AStore(index)
            | Ret(index) => write!(f, "{name} {index}"),
            IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
            | IfLe(target) | IfICmpEq(target) | IfICmpNe(target) | IfICmpLt(target)
            | IfICmpGe(target) | IfICmpGt(target) | IfICmpLe(target) | IfACmpEq(target)
            | IfACmpNe(target) | Goto(target) | Jsr(target) | IfNull(target)
            | IfNonNull(target) | GotoW(target) | JsrW(target) => {
                write!(f, "{name} {target}")
            }
            GetStatic(reference) | PutStatic(reference) | GetField(reference)
            | PutField(reference) => write!(f, "{name} {reference}"),
            InvokeVirtual(reference) | InvokeSpecial(reference) | InvokeStatic(reference) => {
                write!(f, "{name} {reference}")
            }
            Ldc(constant) | LdcW(constant) | Ldc2W(constant) => {
                write!(f, "{name} {constant}")
            }
            New(reference) => write!(f, "{name} {reference}"),
            ANewArray(field_type) => write!(f, "{name} {field_type}"),
            NewArray(primitive_type) => write!(f, "{name} {primitive_type}"),
            CheckCast(field_type) | InstanceOf(field_type) => {
                write!(f, "{name} {field_type}")
            }
            IInc(index, value) => write!(f, "{name} {index} {value}"),
            InvokeInterface(reference, count) => {
                write!(f, "{name} {reference} count {count}")
            }
            InvokeDynamic {
                bootstrap_method_index: bsi,
                name: closure_name,
                descriptor,
            } => write!(f, "{name} #{bsi} {closure_name} {descriptor}"),
            TableSwitch {
                low,
                jump_targets,
                default,
            } => {
                let high = i128::from(*low) + jump_targets.len() as i128 - 1;
                writeln!(f, "{name} {low}..={high} {{")?;
                for (offset, target) in jump_targets.iter().enumerate() {
                    let value = i128::from(*low) + offset as i128;
                    writeln!(f, "  {value}: {target}")?;
                }
                writeln!(f, "  default: {default}")?;
                write!(f, "}}")
            }
            LookupSwitch {
                default,
                match_targets,
            } => {
                writeln!(f, "{name} {{")?;
                for (key, target) in match_targets {
                    writeln!(f, "  {key}: {target}")?;
                }
                writeln!(f, "  default: {default}")?;
                write!(f, "}}")
            }
            Wide(instruction) => write!(f, "{name} {instruction}"),
            MultiANewArray(field_type, dimensions) => {
                write!(f, "{name} {field_type} {dimensions}")
            }
        }
    }
}
