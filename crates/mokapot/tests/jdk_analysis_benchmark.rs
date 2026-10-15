//! Benchmark selection tests independent of Java fixtures and extracted JDKs.

use mokapot::{
    ir::MokaIRMethod,
    jvm::{
        Annotations, Method,
        code::{Instruction, MethodBody},
        method::AccessFlags,
    },
};

#[path = "../benches/support/path_condition_corpus.rs"]
mod path_condition_corpus;
use path_condition_corpus::{PathCorpus, predicate_limit};

fn ir(instructions: impl IntoIterator<Item = (u16, Instruction)>) -> MokaIRMethod {
    let method = Method {
        access_flags: AccessFlags::STATIC,
        name: "test".to_owned(),
        descriptor: "(I)V".parse().unwrap(),
        owner: "BenchmarkTest".parse().unwrap(),
        body: Some(MethodBody {
            max_stack: 1,
            max_locals: 1,
            instructions: instructions
                .into_iter()
                .map(|(pc, instruction)| (pc.into(), instruction))
                .collect(),
            exception_table: vec![],
            line_number_table: None,
            local_variable_table: None,
            stack_map_table: None,
            type_annotations: Annotations::default(),
            other_attributes: vec![],
        }),
        exceptions: vec![],
        annotations: Annotations::default(),
        type_annotations: Annotations::default(),
        parameter_annotations: Annotations::default(),
        annotation_default: None,
        parameters: vec![],
        is_synthetic: false,
        is_deprecated: false,
        signature: None,
        other_attributes: vec![],
    };
    MokaIRMethod::from_method(&method).unwrap()
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
    let straight_line = ir([(0, Instruction::Return)]);
    let loop_method = ir([
        (0, Instruction::ILoad0),
        (1, Instruction::IfEq(7.into())),
        (4, Instruction::Goto(0.into())),
        (7, Instruction::Return),
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
