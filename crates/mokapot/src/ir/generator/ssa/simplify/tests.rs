use super::*;

fn value(index: u32) -> SsaValueId {
    SsaValueId::new(index)
}

fn block(index: u32) -> BlockId {
    BlockId::new(index)
}

#[test]
fn canonicalizes_trivial_phi_chains() {
    let simplified = simplify_phis(BTreeMap::from([
        (value(1), vec![(block(0), value(100))]),
        (value(2), vec![(block(1), value(1))]),
    ]))
    .unwrap();

    assert!(simplified.candidates.is_empty());
    assert_eq!(
        simplified.substitutions,
        BTreeMap::from([(value(1), value(100)), (value(2), value(100))])
    );
}

#[test]
fn ignores_self_inputs_when_simplifying() {
    let simplified = simplify_phis(BTreeMap::from([(
        value(1),
        vec![(block(0), value(100)), (block(1), value(1))],
    )]))
    .unwrap();

    assert!(simplified.candidates.is_empty());
    assert_eq!(simplified.substitutions[&value(1)], value(100));
}

#[test]
fn collapses_mutually_recursive_trivial_phis() {
    let simplified = simplify_phis(BTreeMap::from([
        (value(1), vec![(block(0), value(100)), (block(1), value(2))]),
        (value(2), vec![(block(0), value(100)), (block(1), value(1))]),
    ]))
    .unwrap();

    assert!(simplified.candidates.is_empty());
    assert_eq!(simplified.substitutions[&value(1)], value(100));
    assert_eq!(simplified.substitutions[&value(2)], value(100));
}

#[test]
fn rejects_a_closed_reachable_cycle() {
    let error = simplify_phis(BTreeMap::from([
        (value(1), vec![(block(0), value(2))]),
        (value(2), vec![(block(1), value(1))]),
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
fn retains_a_cycle_with_distinct_external_values() {
    let candidates = BTreeMap::from([
        (value(1), vec![(block(0), value(100)), (block(1), value(2))]),
        (value(2), vec![(block(0), value(101)), (block(1), value(1))]),
    ]);
    let simplified = simplify_phis(candidates.clone()).unwrap();

    assert!(simplified.substitutions.is_empty());
    assert_eq!(simplified.candidates, candidates);
}

#[test]
fn rewrites_inputs_of_retained_candidates_to_canonical_values() {
    let simplified = simplify_phis(BTreeMap::from([
        (value(1), vec![(block(0), value(100))]),
        (
            value(2),
            vec![
                (block(0), value(1)),
                (block(1), value(101)),
                (block(2), value(2)),
            ],
        ),
    ]))
    .unwrap();

    assert_eq!(simplified.substitutions[&value(1)], value(100));
    assert_eq!(
        simplified.candidates[&value(2)],
        [
            (block(0), value(100)),
            (block(1), value(101)),
            (block(2), value(2))
        ]
    );
}
