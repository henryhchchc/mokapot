use super::*;
use crate::ir::BlockId;

fn value(index: u32) -> SsaValueId {
    SsaValueId::new(index)
}

fn block(index: u32) -> BlockId {
    BlockId::new(index)
}

fn candidate(placement: u32, inputs: Vec<(BlockId, SsaValueId)>) -> PhiCandidate {
    PhiCandidate {
        placement: block(placement),
        inputs,
    }
}

#[test]
fn ignores_self_inputs_when_simplifying() {
    let simplified = simplify_phis(BTreeMap::from([(
        value(1),
        candidate(2, vec![(block(0), value(100)), (block(1), value(1))]),
    )]))
    .unwrap();

    assert_eq!(
        simplified,
        SimplifiedPhis {
            substitutions: BTreeMap::from([(value(1), value(100))]),
            candidates: BTreeMap::new(),
        }
    );
}

#[test]
fn rejects_a_closed_reachable_cycle() {
    let error = simplify_phis(BTreeMap::from([
        (value(1), candidate(2, vec![(block(0), value(2))])),
        (value(2), candidate(3, vec![(block(1), value(1))])),
    ]))
    .unwrap_err();

    assert_eq!(
        error,
        PhiSimplificationError::ClosedCycle {
            representative: value(2)
        }
    );
}

#[test]
fn canonicalizes_chains_before_rewriting_retained_candidates() {
    let simplified = simplify_phis(BTreeMap::from([
        (value(1), candidate(3, vec![(block(0), value(100))])),
        (value(2), candidate(4, vec![(block(0), value(1))])),
        (
            value(3),
            candidate(
                5,
                vec![
                    (block(0), value(2)),
                    (block(1), value(101)),
                    (block(2), value(3)),
                ],
            ),
        ),
    ]))
    .unwrap();

    assert_eq!(
        simplified,
        SimplifiedPhis {
            substitutions: BTreeMap::from([(value(1), value(100)), (value(2), value(100)),]),
            candidates: BTreeMap::from([(
                value(3),
                candidate(
                    5,
                    vec![
                        (block(0), value(100)),
                        (block(1), value(101)),
                        (block(2), value(3)),
                    ],
                ),
            )]),
        }
    );
}
