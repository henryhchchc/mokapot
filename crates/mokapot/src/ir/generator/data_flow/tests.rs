//! Test-only oracle for the dataflow frame bookkeeping.

use std::collections::HashMap;

use super::{
    analysis::{BlockSolution, DataflowSolver, FrameSource},
    frame::Frame,
};
use crate::{
    ir::{BlockId, generator::control_flow},
    jvm::Method,
};

/// Asserts the frame-flow invariant of `method`'s solutions.
fn verify_method(method: &Method) {
    let cfg = control_flow::analyze(method).expect("the test method has a valid CFG");
    let solver = DataflowSolver::new(&cfg).expect("the test method has a valid entry frame");
    let parts = solver.solve().expect("the test method has valid dataflow");
    verify_frames(parts.entry, &parts.blocks);
}

mod cases {
    use super::*;
    use std::collections::BTreeMap;

    use crate::{
        ir::{generator::data_flow::analyze, test::prelude::*},
        jvm::code::Instruction,
    };

    #[test]
    fn parallel_edges_deliver_their_join_arguments() {
        let switch = Instruction::LookupSwitch {
            default: 8.into(),
            match_targets: BTreeMap::from([(1, 12.into()), (2, 12.into())]),
        };
        let body = [
            (0, Instruction::IConst0),
            (1, Instruction::IStore1),
            (2, Instruction::ILoad0),
            (3, switch),
            (8, Instruction::IConst1),
            (9, Instruction::IStore1),
            (10, Instruction::Goto(12.into())),
            (12, Instruction::ILoad1),
            (13, Instruction::IReturn),
        ];
        let method = method(body, "(I)I", vec![]);
        let cfg = control_flow::analyze(&method).unwrap();
        let parts = analyze(&cfg).unwrap();
        let (&target, block) = parts
            .blocks
            .iter()
            .find(|(_, block)| !block.parameters.is_empty())
            .expect("the local join has a block parameter");
        assert_eq!(block.parameters.len(), 1);
        let incoming = parts
            .blocks
            .values()
            .flat_map(|block| block.terminator.arms())
            .filter(|arm| arm.block_target() == Some(target))
            .collect::<Vec<_>>();
        assert_eq!(incoming.len(), 3);
        assert!(incoming.iter().all(|arm| arm.arguments().len() == 1));
        verify_method(&method);
    }

    #[test]
    fn branch_and_loop_frames_arrive_on_their_edges() {
        let body = [
            (0, Instruction::ILoad0),
            (1, Instruction::IfEq(8.into())),
            (4, Instruction::IInc(0, -1)),
            (7, Instruction::Goto(0.into())),
            (8, Instruction::ILoad0),
            (9, Instruction::IReturn),
        ];
        let method = method(body, "(I)I", vec![]);
        verify_method(&method);
    }

    #[test]
    fn exception_handler_frames_arrive_from_throwing_instructions() {
        let body = [
            (0, Instruction::ALoad0),
            (1, Instruction::ArrayLength),
            (2, Instruction::IReturn),
            (3, Instruction::AStore1),
            (4, Instruction::IConst0),
            (5, Instruction::IReturn),
        ];
        let handlers = vec![crate::jvm::code::ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 3.into(),
            catch_type: None,
        }];
        let method = method(body, "([I)I", handlers);
        verify_method(&method);
    }
}

/// Verifies closure and both directions of the frame-flow relation.
fn verify_frames(entry: BlockId, blocks: &HashMap<BlockId, BlockSolution>) {
    let delivered = delivered_frames(blocks);
    for (&source, solution) in blocks {
        for (frame_source, target, frame) in solution.block.outgoing_frames(source) {
            let Some(target_block) = blocks.get(&target) else {
                panic!("the successor {target:?} of {source:?} has no block solution");
            };
            let Some(recorded) = target_block.incoming_frames.get(&frame_source) else {
                panic!("the frame delivered along {frame_source:?} never reached block {target:?}");
            };
            assert_eq!(
                recorded, frame,
                "the frame delivered along {frame_source:?} differs from the one recorded by {target:?}"
            );
        }
    }
    for (&block, solution) in blocks {
        for (frame_source, frame) in &solution.incoming_frames {
            if *frame_source == FrameSource::Entry && block == entry {
                continue;
            }
            let Some(delivered_frame) = delivered.get(&(block, *frame_source)) else {
                panic!(
                    "block {block:?} recorded an arrival from {frame_source:?} that no arm delivers"
                );
            };
            assert_eq!(
                delivered_frame, frame,
                "block {block:?} recorded a frame from {frame_source:?} that its arm does not carry"
            );
        }
    }
}

/// The frame each arm delivers, keyed by the receiving block and the arm.
fn delivered_frames(
    blocks: &HashMap<BlockId, BlockSolution>,
) -> HashMap<(BlockId, FrameSource), Frame> {
    let mut delivered = HashMap::new();
    for (&source, solution) in blocks {
        for (frame_source, target, frame) in solution.block.outgoing_frames(source) {
            let replaced = delivered.insert((target, frame_source), frame.clone());
            assert!(
                replaced.is_none(),
                "the frame delivered to {target:?} along {frame_source:?} was delivered twice"
            );
        }
    }
    delivered
}
