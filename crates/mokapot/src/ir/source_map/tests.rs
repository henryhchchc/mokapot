use super::*;
use crate::ir::BlockId;

#[test]
fn source_map_is_sparse_and_one_to_many() {
    let pc0 = ProgramCounter::from(0);
    let pc1 = ProgramCounter::from(100);
    let pc2 = ProgramCounter::from(200);
    let instruction0 = InstructionLocation::Operation {
        block: BlockId::new(0),
        index: 0,
    };
    let instruction1 = InstructionLocation::Operation {
        block: BlockId::new(0),
        index: 1,
    };
    let instruction2 = InstructionLocation::Operation {
        block: BlockId::new(1),
        index: 0,
    };
    let instruction3 = InstructionLocation::Terminator {
        block: BlockId::new(1),
    };
    let synthetic = InstructionLocation::BlockParameter {
        block: BlockId::new(1),
        index: 0,
    };
    let mut map = SourceMap::default();
    map.record_operation(pc0, BlockId::new(0), 0);
    map.record_operation(pc0, BlockId::new(0), 1);
    map.record_operation(pc1, BlockId::new(1), 0);
    map.record_terminator(pc2, BlockId::new(1));

    assert_eq!(
        map.instructions_at(pc0).collect::<Vec<_>>(),
        [instruction0, instruction1]
    );
    assert_eq!(map.origin_of(instruction0), Some(pc0));
    assert_eq!(map.origin_of(instruction1), Some(pc0));
    assert_eq!(map.origin_of(instruction2), Some(pc1));
    assert_eq!(map.origin_of(instruction3), Some(pc2));
    assert_eq!(map.instructions_at(pc2).collect::<Vec<_>>(), [instruction3]);
    assert_eq!(map.instructions_at(50.into()).count(), 0);
    assert_eq!(map.origin_of(synthetic), None);

    let covered_nodes = BTreeSet::from([pc0])
        .into_iter()
        .flat_map(|pc| map.instructions_at(pc))
        .collect::<BTreeSet<_>>();
    assert_eq!(covered_nodes, BTreeSet::from([instruction0, instruction1]));
    assert!(!covered_nodes.contains(&instruction2));
    assert!(!covered_nodes.contains(&synthetic));
}
