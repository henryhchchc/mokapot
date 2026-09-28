use mokapot::ir::BlockKind;

use super::*;

#[test]
fn ssa_definitions_and_block_arguments_are_well_formed() {
    for (label, ir) in corpus_ir() {
        let mut definitions = HashSet::new();
        for value in ir.this.iter().chain(&ir.parameters) {
            assert!(definitions.insert(*value), "{label}: repeated method input");
        }
        assert_eq!(
            ir.entry.arguments.len(),
            ir.block(ir.entry.block).unwrap().parameters.len(),
            "{label}: entry argument count differs from its parameter count",
        );
        for (_, block) in reachable_blocks(&ir) {
            if let BlockKind::LandingPad { exception } = block.kind {
                assert!(
                    definitions.insert(exception),
                    "{label}: repeated caught exception"
                );
            }
            for parameter in &block.parameters {
                assert!(
                    definitions.insert(parameter.value),
                    "{label}: repeated block parameter"
                );
            }
            let operations = block.operations.iter().chain(block.terminator.operation());
            for operation in operations {
                if let Operation::Definition { value, .. } = operation {
                    assert!(
                        definitions.insert(*value),
                        "{label}: repeated operation result"
                    );
                }
            }
            for successor in block.terminator.successors() {
                if let Some(target) = successor.block_target() {
                    assert_eq!(
                        successor.arguments().len(),
                        ir.block(target).unwrap().parameters.len(),
                        "{label}: successor argument count differs from its parameter count",
                    );
                }
            }
        }
    }
}
