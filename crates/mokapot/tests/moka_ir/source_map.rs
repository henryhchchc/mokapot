#![cfg(integration_test)]

use super::*;

#[test]
fn origin_mapping_is_consistent_with_instruction_locations() {
    for (label, ir) in corpus_ir() {
        for location in live_locations(&ir) {
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
}
