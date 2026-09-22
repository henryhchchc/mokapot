use super::*;

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn builds_ir_blocks_and_provenance() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();

    // The `ldc` at `0x0000` may fail, so lifting folds it into the block's
    // `Try` terminator instead of an ordinary operation.
    let first = ir
        .source_map()
        .instructions_at(ProgramCounter::from(0x0000))
        .find_map(|id| terminator(&ir, id))
        .and_then(Terminator::operation)
        .unwrap();
    assert!(matches!(
        first,
        Operation::Definition {
            expr: Expression::Const(ConstantValue::String(JavaString::Utf8(value))),
            ..
        } if value == "233"
    ));

    assert_eq!(
        ir.source_map()
            .instructions_at(ProgramCounter::from(0x007B))
            .count(),
        0
    );

    let returned = ir
        .source_map()
        .instructions_at(ProgramCounter::from(0x00F7))
        .find_map(|id| terminator(&ir, id))
        .unwrap();
    // Exiting a method may unwind, so the return is a fallible terminator.
    assert!(matches!(
        returned,
        Terminator::Return { value: Some(value), .. } if value == &ir.parameter_values()[1]
    ));
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn block_parameters_have_no_origin() {
    let ir = MokaIRMethod::from_method(&get_test_method()).unwrap();
    for (block, basic_block) in reachable_blocks(&ir) {
        for (index, _) in basic_block.parameters.iter().enumerate() {
            let parameter = InstructionLocation::BlockParameter { block, index };
            assert!(
                ir.source_map().origin_of(parameter).is_none(),
                "block parameter {parameter:?} has a JVM origin"
            );
        }
    }
}
