use super::*;

#[test]
fn builds_ir_blocks_and_provenance() {
    let class = get_test_class();
    let method = class
        .methods()
        .find(|method| method.name() == "test")
        .unwrap();
    let ir = MokaIRMethod::from_method(method).unwrap();
    // The `ldc` at `0x0000` may fail, so lifting folds it into the block's
    // `Try` terminator instead of an ordinary operation.
    let first = ir
        .source_map
        .instructions_at(ProgramCounter::from(0x0000))
        .find_map(|id| terminator(&ir, id))
        .and_then(Terminator::operation)
        .unwrap();
    assert!(matches!(
        first,
        Operation::Definition(_, Expression::Const(ConstantVal::String(value))) if value.to_str().as_deref() == Ok("233")
    ));

    assert_eq!(
        ir.source_map
            .instructions_at(ProgramCounter::from(0x007B))
            .count(),
        0
    );

    let returned = ir
        .source_map
        .instructions_at(ProgramCounter::from(0x00F7))
        .find_map(|id| terminator(&ir, id))
        .unwrap();
    // Exiting a method may unwind, so the return is a fallible terminator.
    assert!(matches!(
        returned,
        Terminator::Return { value: Some(value), .. } if value == &ir.parameters[1]
    ));
}
