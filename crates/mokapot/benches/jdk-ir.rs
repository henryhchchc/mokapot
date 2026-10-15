//! IR generation over pre-parsed JDK methods.

use std::{env, hint::black_box, sync::LazyLock};

use divan::{Bencher, counter::ItemsCount};
use mokapot::jvm::Method;

#[path = "support/jdk_corpus.rs"]
mod jdk_corpus;
#[path = "support/jdk_methods.rs"]
mod jdk_methods;

static METHODS: LazyLock<Vec<Method>> = LazyLock::new(|| {
    let mut methods = Vec::new();
    jdk_methods::for_each_method(|method| {
        drop(jdk_methods::build_ir(&method));
        methods.push(method);
    });
    assert!(
        !methods.is_empty(),
        "JDK shard contains no methods with bodies"
    );
    jdk_corpus::record_summary(format!("IR generation: {} methods", methods.len()));
    methods
});

#[divan::bench(ignore = env::var_os("JDK_CLASSES").is_none())]
fn generate(bencher: Bencher) {
    let methods = &*METHODS;
    bencher
        .counter(ItemsCount::new(methods.len()))
        .bench_local(|| {
            for method in methods {
                drop(black_box(jdk_methods::build_ir(black_box(method))));
            }
        });
}

fn main() {
    divan::main();
    jdk_corpus::print_summary();
}
