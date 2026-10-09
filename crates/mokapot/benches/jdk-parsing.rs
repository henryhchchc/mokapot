//! Sequential parsing benchmarks over an extracted JDK corpus.

use std::{env, hint::black_box, path::PathBuf, sync::LazyLock};

use divan::{
    Bencher,
    counter::{BytesCount, ItemsCount},
};
use mokapot::jvm::{Class, bytecode::benchmark_support};

#[path = "support/jdk_corpus.rs"]
mod jdk_corpus;
use jdk_corpus::{Corpus, Shard};

static CORPUS: LazyLock<Corpus> = LazyLock::new(|| {
    let root = env::var_os("JDK_CLASSES").map(PathBuf::from).expect(
        "set JDK_CLASSES to an extracted JDK image (jimage extract --dir=<path> <jdk>/lib/modules)",
    );
    let index = env::var_os("JDK_BENCH_SHARD_INDEX").map(|value| {
        value
            .into_string()
            .expect("JDK_BENCH_SHARD_INDEX must be UTF-8")
    });
    let count = env::var_os("JDK_BENCH_SHARD_COUNT").map(|value| {
        value
            .into_string()
            .expect("JDK_BENCH_SHARD_COUNT must be UTF-8")
    });
    let shard = Shard::new(index.as_deref(), count.as_deref())
        .expect("valid JDK benchmark shard configuration");
    let corpus = Corpus::load(&root, shard).expect("load JDK benchmark corpus");
    eprintln!(
        "JDK corpus: {}; shard {}/{}; {} classes; {} bytes; fingerprint {:016x}",
        root.display(),
        shard.index,
        shard.count,
        corpus.classes.len(),
        corpus.byte_count,
        corpus.fingerprint
    );
    corpus
});

#[divan::bench(ignore = env::var_os("JDK_CLASSES").is_none())]
fn raw_class(bencher: Bencher) {
    let corpus = &*CORPUS;
    for input in &corpus.classes {
        benchmark_support::raw_class(&input.bytes)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
    }
    bencher
        .counter(BytesCount::new(corpus.byte_count))
        .counter(ItemsCount::new(corpus.classes.len()))
        .bench_local(|| {
            for input in &corpus.classes {
                let parsed = benchmark_support::raw_class(black_box(&input.bytes))
                    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
                drop(black_box(parsed));
            }
        });
}

#[divan::bench(ignore = env::var_os("JDK_CLASSES").is_none())]
fn whole_class(bencher: Bencher) {
    let corpus = &*CORPUS;
    for input in &corpus.classes {
        Class::from_reader(&mut input.bytes.as_slice())
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
    }
    bencher
        .counter(BytesCount::new(corpus.byte_count))
        .counter(ItemsCount::new(corpus.classes.len()))
        .bench_local(|| {
            for input in &corpus.classes {
                let parsed = Class::from_reader(&mut black_box(input.bytes.as_slice()))
                    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
                drop(black_box(parsed));
            }
        });
}

fn main() {
    divan::main();
}
