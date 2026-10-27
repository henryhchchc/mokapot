//! Test-only oracle for the dataflow frame bookkeeping.

use super::{
    analysis::{BlockSolution, DataflowSolver, FrameSource},
    *,
};
use crate::{ir::generator::control_flow, tests::TestMethod};

/// Asserts the frame-flow invariant of `method`'s solutions.
fn verify_method(method: &TestMethod) {
    let cfg = control_flow::analyze(method.as_method()).expect("the test method has a valid CFG");
    let solver = DataflowSolver::new(&cfg).expect("the test method has a valid entry frame");
    let parts = solver.solve().expect("the test method has valid dataflow");
    verify_frames(parts.entry, &parts.blocks);
}

mod cases {
    use super::*;
    use crate::{
        ir::{generator::data_flow::analyze, test::prelude::*},
        jvm::code::InstructionData,
    };

    #[test]
    fn parallel_edges_deliver_their_join_arguments() {
        let switch = InstructionData::LookupSwitch {
            default: 8.into(),
            match_targets: Box::new([(1, 12.into()), (2, 12.into())]),
        };
        let body = [
            (0, InstructionData::IConst0),
            (1, InstructionData::IStore1),
            (2, InstructionData::ILoad0),
            (3, switch),
            (8, InstructionData::IConst1),
            (9, InstructionData::IStore1),
            (10, InstructionData::Goto(12.into())),
            (12, InstructionData::ILoad1),
            (13, InstructionData::IReturn),
        ];
        let method = method(body, "(I)I", vec![]);
        let cfg = control_flow::analyze(method.as_method()).unwrap();
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
