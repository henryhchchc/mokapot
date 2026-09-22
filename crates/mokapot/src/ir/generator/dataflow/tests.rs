//! Test-only oracle for the dataflow frame bookkeeping.

use std::collections::HashMap;

use super::{
    analysis::{BlockSolution, DataflowSolver, FrameSource},
    frame::Frame,
};
use crate::{
    ir::{BlockId, generator::controlflow},
    jvm::Method,
};

/// Asserts the frame-flow invariant of `method`'s solutions; methods the phase
/// rejects are left to the tests that cover those failures.
pub(crate) fn verify_method(method: &Method) {
    let Ok(cfg) = controlflow::analyze(method) else {
        return;
    };
    let Ok(solver) = DataflowSolver::new(&cfg) else {
        return;
    };
    let Ok(parts) = solver.solve() else {
        return;
    };
    verify_frames(parts.entry, &parts.blocks);
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
