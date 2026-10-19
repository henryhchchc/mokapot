//! Benchmark selection tests independent of Java fixtures and extracted JDKs.

use crate::ir::test::prelude::{InstructionData, MokaIRMethod, method};

#[path = "../../../benches/support/path_condition_corpus.rs"]
mod path_condition_corpus;
use path_condition_corpus::{PathCorpus, predicate_limit};

fn ir(instructions: impl IntoIterator<Item = (u16, InstructionData)>) -> MokaIRMethod {
    let fixture = method(instructions, "(I)V", Vec::new());
    MokaIRMethod::from_method(fixture.as_method()).unwrap()
}

#[test]
fn predicate_limit_configuration() {
    assert_eq!(predicate_limit(None).unwrap(), 16);
    assert_eq!(predicate_limit(Some("0")).unwrap(), 0);
    assert_eq!(
        predicate_limit(Some("-1")).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[test]
fn path_benchmarks_group_methods_and_apply_the_inclusive_limit() {
    let straight_line = ir([(0, InstructionData::Return)]);
    let loop_method = ir([
        (0, InstructionData::ILoad0),
        (1, InstructionData::IfEq(7.into())),
        (4, InstructionData::Goto(0.into())),
        (7, InstructionData::Return),
    ]);

    for (limit, excluded) in [(0, 1), (1, 1), (2, 0)] {
        let mut corpus = PathCorpus::default();
        corpus.insert(straight_line.clone(), limit);
        corpus.insert(loop_method.clone(), limit);
        assert_eq!(corpus.bins[&0].len(), 1);
        assert_eq!(corpus.bins.get(&2).map_or(0, Vec::len), 1 - excluded);
        assert_eq!(corpus.excluded, excluded);
    }
}
