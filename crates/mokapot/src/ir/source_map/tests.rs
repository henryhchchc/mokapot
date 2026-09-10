use super::*;

#[test]
fn source_map_is_a_sparse_many_to_many_relation() {
    let pc0 = ProgramCounter::from(0);
    let pc1 = ProgramCounter::from(100);
    let pc2 = ProgramCounter::from(200);
    let instruction0 = InstructionId::new(0);
    let instruction1 = InstructionId::new(1);
    let instruction2 = InstructionId::new(2);
    let instruction3 = InstructionId::new(3);
    let synthetic = InstructionId::new(4);
    let mut map = SourceMap::default();
    map.insert(pc0, instruction0);
    map.insert(pc0, instruction1);
    map.insert(pc1, instruction1);
    map.insert(pc1, instruction2);
    map.insert(pc2, instruction3);

    assert_eq!(
        map.instructions_at(pc0).collect::<Vec<_>>(),
        [instruction0, instruction1]
    );
    assert_eq!(map.origins_of(instruction1).collect::<Vec<_>>(), [pc0, pc1]);
    assert_eq!(map.instructions_at(pc2).collect::<Vec<_>>(), [instruction3]);
    assert_eq!(map.instructions_at(50.into()).count(), 0);
    assert_eq!(map.origins_of(synthetic).count(), 0);

    let covered_nodes = BTreeSet::from([pc0])
        .into_iter()
        .flat_map(|pc| map.instructions_at(pc))
        .collect::<BTreeSet<_>>();
    assert_eq!(covered_nodes, BTreeSet::from([instruction0, instruction1]));
    assert!(!covered_nodes.contains(&instruction2));
    assert!(!covered_nodes.contains(&synthetic));
}
