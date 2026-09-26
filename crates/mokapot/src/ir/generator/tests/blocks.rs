use super::*;

#[test]
fn unreachable_bytecode_is_omitted() {
    // Instructions the entry cannot reach contribute no IR nodes.
    let unreachable = method(
        [
            (0, Instruction::Goto(100.into())),
            (10, Instruction::IConst0),
            (11, Instruction::IReturn),
            (100, Instruction::Return),
        ],
        "()V",
        vec![],
    );
    let ir = build(&unreachable).unwrap();
    assert_eq!(ir.source_map.instructions_at(10.into()).count(), 0);
    assert_eq!(ir.source_map.instructions_at(11.into()).count(), 0);

    let frame_invalid = method(
        [
            (0, Instruction::Goto(10.into())),
            (3, Instruction::IAdd),
            (10, Instruction::Return),
        ],
        "()V",
        vec![],
    );
    let ir =
        build(&frame_invalid).expect("unreachable instructions must not contribute frame facts");
    assert_eq!(ir.source_map.instructions_at(3.into()).count(), 0);
}
