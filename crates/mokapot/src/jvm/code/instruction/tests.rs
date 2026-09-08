use super::{super::ProgramCounter, Instruction::*, WideInstruction};

#[test]
fn opcode_matches_jvm_encoding() {
    assert_eq!(Nop.opcode(), 0x00);
    assert_eq!(AConstNull.opcode(), 0x01);
    assert_eq!(IConstM1.opcode(), 0x02);
    assert_eq!(ILoad(233).opcode(), 0x15);
}

#[test]
fn display_formats_operands() {
    assert_eq!(Nop.to_string(), "nop");
    assert_eq!(AConstNull.to_string(), "aconst_null");
    assert_eq!(ILoad(5).to_string(), "iload 5");
    assert_eq!(BiPush(42).to_string(), "bipush 42");
    assert_eq!(IInc(10, 5).to_string(), "iinc 10 5");
    assert_eq!(
        Wide(WideInstruction::ILoad(1000)).to_string(),
        "wide iload 1000"
    );
    assert_eq!(
        Wide(WideInstruction::IInc(500, 50)).to_string(),
        "wide iinc 500 50"
    );
    assert_eq!(IfEq(ProgramCounter::ZERO).to_string(), "ifeq #0000");
    assert_eq!(Goto(ProgramCounter::ZERO).to_string(), "goto #0000");
}
