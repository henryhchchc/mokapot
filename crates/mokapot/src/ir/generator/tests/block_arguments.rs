use super::*;

#[test]
fn value_missing_on_one_predecessor_cannot_be_used_at_the_join() {
    let method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(5.into())),
            (2, Instruction::IConst1),
            (3, Instruction::IStore1),
            (4, Instruction::Goto(6.into())),
            (5, Instruction::Nop),
            (6, Instruction::ILoad1),
            (7, Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::InvalidFrame {
            pc: Some(pc),
            ..
        }) if pc == 6.into()
    ));
}
