use super::{RawInstruction, RawWideInstruction};
use crate::jvm::{bytecode::GenerationError, code::ProgramCounter};

impl RawInstruction {
    /// Returns the encoded length of this instruction at `pc`.
    pub(crate) fn num_bytes(&self, pc: ProgramCounter) -> Result<u16, GenerationError> {
        #[allow(clippy::enum_glob_use, reason = "JVM instructions are numerous")]
        use RawInstruction::*;

        Ok(match self {
            AALoad | AAStore | AConstNull | ALoad0 | ALoad1 | ALoad2 | ALoad3 | AReturn
            | ArrayLength | AStore0 | AStore1 | AStore2 | AStore3 | AThrow | BALoad | BAStore
            | CALoad | CAStore | D2F | D2I | D2L | DAdd | DALoad | DAStore | DCmpG | DCmpL
            | DConst0 | DConst1 | DDiv | DLoad0 | DLoad1 | DLoad2 | DLoad3 | DMul | DNeg | DRem
            | DReturn | DStore0 | DStore1 | DStore2 | DStore3 | DSub | Dup | DupX1 | DupX2
            | Dup2 | Dup2X1 | Dup2X2 | F2D | F2I | F2L | FAdd | FALoad | FAStore | FCmpG
            | FCmpL | FConst0 | FConst1 | FConst2 | FDiv | FLoad0 | FLoad1 | FLoad2 | FLoad3
            | FMul | FNeg | FRem | FReturn | FStore0 | FStore1 | FStore2 | FStore3 | FSub | I2B
            | I2C | I2D | I2F | I2L | I2S | IAdd | IALoad | IAnd | IAStore | IConstM1 | IConst0
            | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 | IDiv | ILoad0 | ILoad1 | ILoad2
            | ILoad3 | IMul | INeg | IOr | IRem | IReturn | IShl | IShr | IStore0 | IStore1
            | IStore2 | IStore3 | ISub | IUShr | IXor | L2D | L2F | L2I | LAdd | LALoad | LAnd
            | LAStore | LCmp | LConst0 | LConst1 | LDiv | LLoad0 | LLoad1 | LLoad2 | LLoad3
            | LMul | LNeg | LOr | LRem | LReturn | LShl | LShr | LStore0 | LStore1 | LStore2
            | LStore3 | LSub | LUShr | LXor | MonitorEnter | MonitorExit | Nop | Pop | Pop2
            | Return | SALoad | SAStore | Swap | Breakpoint | ImpDep1 | ImpDep2 => 1,
            BiPush { .. }
            | Ldc { .. }
            | ALoad { .. }
            | FLoad { .. }
            | DLoad { .. }
            | ILoad { .. }
            | LLoad { .. }
            | AStore { .. }
            | FStore { .. }
            | DStore { .. }
            | IStore { .. }
            | LStore { .. }
            | NewArray { .. }
            | Ret { .. } => 2,
            SiPush { .. }
            | LdcW { .. }
            | Ldc2W { .. }
            | GetField { .. }
            | GetStatic { .. }
            | PutField { .. }
            | PutStatic { .. }
            | InvokeVirtual { .. }
            | InvokeSpecial { .. }
            | InvokeStatic { .. }
            | New { .. }
            | ANewArray { .. }
            | CheckCast { .. }
            | InstanceOf { .. }
            | Goto { .. }
            | IfEq { .. }
            | IfNe { .. }
            | IfLt { .. }
            | IfGe { .. }
            | IfGt { .. }
            | IfLe { .. }
            | IfICmpEq { .. }
            | IfICmpNe { .. }
            | IfICmpLt { .. }
            | IfICmpGe { .. }
            | IfICmpGt { .. }
            | IfICmpLe { .. }
            | IfACmpEq { .. }
            | IfACmpNe { .. }
            | IfNull { .. }
            | IfNonNull { .. }
            | Jsr { .. }
            | IInc { .. }
            | MultiANewArray { .. } => 3,
            InvokeInterface { .. } | InvokeDynamic { .. } | GotoW { .. } | JsrW { .. } => 5,
            Wide(RawWideInstruction::IInc { .. }) => 6,
            Wide(_) => 4,
            TableSwitch { low, high, .. } => {
                let padding = (4 - (u16::from(pc) + 1) % 4) % 4;
                let entries = u16::try_from(*high - *low + 1)
                    .map_err(|_| GenerationError::other("Invalid jump offset"))?;
                1 + padding + 12 + 4 * entries
            }
            LookupSwitch { match_offsets, .. } => {
                let padding = (4 - (u16::from(pc) + 1) % 4) % 4;
                let entries = u16::try_from(match_offsets.len())
                    .map_err(|_| GenerationError::other("Invalid jump offset"))?;
                1 + padding + 8 + 8 * entries
            }
        })
    }
}
