use super::*;

#[test]
fn value_missing_on_one_predecessor_cannot_be_used_at_the_join() {
    let method = method(
        [
            (0, InstructionData::ILoad0),
            (1, InstructionData::IfEq(5.into())),
            (2, InstructionData::IConst1),
            (3, InstructionData::IStore1),
            (4, InstructionData::Goto(6.into())),
            (5, InstructionData::Nop),
            (6, InstructionData::ILoad1),
            (7, InstructionData::IReturn),
        ],
        "(I)I",
        vec![],
    );

    assert_eq!(build_failure(&method).0, Some(6.into()));
}
