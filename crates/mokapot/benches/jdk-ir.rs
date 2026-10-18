//! IR generation over pre-parsed JDK methods.

use std::{env, hint::black_box, sync::LazyLock};

use divan::{Bencher, counter::ItemsCount};
use mokapot::jvm::{Class, Method};

#[path = "support/jdk_corpus.rs"]
mod jdk_corpus;
#[path = "support/jdk_methods.rs"]
mod jdk_methods;

static CLASSES: LazyLock<Vec<Class>> = LazyLock::new(jdk_methods::parse_classes);
static METHODS: LazyLock<Vec<Method<'static>>> = LazyLock::new(|| {
    let classes: &'static [Class] = CLASSES.as_slice();
    let methods: Vec<_> = classes
        .iter()
        .flat_map(Class::methods)
        .filter(|method| method.body().is_some())
        .collect();
    assert!(
        !methods.is_empty(),
        "JDK shard contains no methods with bodies"
    );
    for &method in &methods {
        drop(jdk_methods::build_ir(method));
    }
    jdk_corpus::record_summary(format!("IR generation: {} methods", methods.len()));
    methods
});

#[divan::bench(ignore = env::var_os("JDK_CLASSES").is_none())]
fn generate(bencher: Bencher) {
    let methods = &*METHODS;
    bencher
        .counter(ItemsCount::new(methods.len()))
        .bench_local(|| {
            for &method in methods {
                drop(black_box(jdk_methods::build_ir(black_box(method))));
            }
        });
}

fn main() {
    divan::main();
    jdk_corpus::print_summary();
}
