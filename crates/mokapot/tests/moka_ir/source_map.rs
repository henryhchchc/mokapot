use super::*;

pub(super) fn assert_consistent(label: &str, ir: &MokaIRMethod) {
    for location in live_locations(ir) {
        if matches!(location, InstructionLocation::BlockParameter { .. }) {
            assert!(
                ir.source_map.origin_of(location).is_none(),
                "{label}: block parameter {location:?} has a JVM origin"
            );
            continue;
        }
        let Some(pc) = ir.source_map.origin_of(location) else {
            continue;
        };
        assert!(
            ir.source_map
                .instructions_at(pc)
                .any(|candidate| candidate == location),
            "{label}: JVM location {pc} does not list {location:?}"
        );
        for other in ir.source_map.instructions_at(pc) {
            assert_eq!(
                ir.source_map.origin_of(other),
                Some(pc),
                "{label}: {other:?} does not map back to {pc}"
            );
        }
    }
}
