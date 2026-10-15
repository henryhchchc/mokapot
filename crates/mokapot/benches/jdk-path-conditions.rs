//! Path analysis over pre-generated JDK IR, grouped by predicate occurrences.

use std::{env, hint::black_box, sync::LazyLock};

use divan::{Bencher, counter::ItemsCount};
use mokapot::ir::path_condition::PathCondition;

#[path = "support/jdk_corpus.rs"]
mod jdk_corpus;
#[path = "support/jdk_methods.rs"]
mod jdk_methods;
#[path = "support/path_condition_corpus.rs"]
mod path_condition_corpus;
use path_condition_corpus::{PathCorpus, predicate_limit};

static CORPUS: LazyLock<PathCorpus> = LazyLock::new(|| {
    let value = env::var("JDK_BENCH_MAX_PREDICATES");
    let limit = predicate_limit(match &value {
        Ok(value) => Some(value.as_str()),
        Err(env::VarError::NotPresent) => None,
        Err(error) => panic!("invalid JDK_BENCH_MAX_PREDICATES: {error}"),
    })
    .expect("valid predicate limit");
    let mut corpus = PathCorpus::default();
    jdk_methods::for_each_method(|method| corpus.insert(jdk_methods::build_ir(&method), limit));
    let included: usize = corpus.bins.values().map(Vec::len).sum();
    jdk_corpus::record_summary(format!(
        "Path conditions: {included} methods included; {} excluded; maximum {limit} predicate occurrences",
        corpus.excluded
    ));
    for (count, methods) in &corpus.bins {
        jdk_corpus::record_summary(format!(
            "  {count} predicate occurrences: {} methods",
            methods.len()
        ));
    }
    assert!(included != 0, "JDK shard contains no eligible methods");
    corpus
});

fn predicate_counts() -> Vec<usize> {
    if env::var_os("JDK_CLASSES").is_none() {
        return Vec::new();
    }
    CORPUS.bins.keys().copied().collect()
}

#[divan::bench(args = predicate_counts(), ignore = env::var_os("JDK_CLASSES").is_none())]
fn analyze(bencher: Bencher, predicate_occurrences: usize) {
    let methods = &CORPUS.bins[&predicate_occurrences];
    bencher
        .counter(ItemsCount::new(methods.len()))
        .bench_local(|| {
            for method in methods {
                drop(black_box(PathCondition::analyze(black_box(method))));
            }
        });
}

fn main() {
    divan::main();
    jdk_corpus::print_summary();
}
