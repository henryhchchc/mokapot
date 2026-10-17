//! Sequential parsing benchmarks over an extracted JDK corpus.

use std::{env, hint::black_box, sync::LazyLock};

use divan::{
    Bencher,
    counter::{BytesCount, ItemsCount},
};
use mokapot::jvm::{Class, benchmark_support};

#[path = "support/jdk_corpus.rs"]
mod jdk_corpus;
use jdk_corpus::Corpus;

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

static CORPUS: LazyLock<Corpus> = LazyLock::new(Corpus::from_env);

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
        Class::from_bytes(&input.bytes)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
    }
    bencher
        .counter(BytesCount::new(corpus.byte_count))
        .counter(ItemsCount::new(corpus.classes.len()))
        .bench_local(|| {
            for input in &corpus.classes {
                let parsed = Class::from_bytes(black_box(&input.bytes))
                    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
                drop(black_box(parsed));
            }
        });
}

fn main() {
    divan::main();
    jdk_corpus::print_summary();
}
