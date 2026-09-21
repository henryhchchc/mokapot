#![allow(missing_docs, clippy::ignore_without_reason)]

use mokapot::{
    analysis::ResolutionContext,
    jvm::{
        class_loader::class_paths::{DirectoryClassPath, NopClassPath},
        references::ClassRef,
    },
};

const TEST_CP: &str = concat!(env!("OUT_DIR"), "/mokapot/java_classes");

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn load_classes() {
    let app_cp = DirectoryClassPath::new(TEST_CP);
    let ctx = ResolutionContext::new([app_cp], NopClassPath::EMPTY);
    let test_analysis: ClassRef = "org/mokapot/test/TestAnalysis".parse().unwrap();
    assert!(ctx.application_classes.contains_key(&test_analysis));
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn interfaces_impl() {
    let app_cp = DirectoryClassPath::new(TEST_CP);
    let ctx = ResolutionContext::new([app_cp], NopClassPath::EMPTY);
    let my_class: ClassRef = "org/mokapot/test/MyClass".parse().unwrap();
    let implements = ctx
        .interface_implementations
        .implemented_interfaces(&my_class);
    let closeable: ClassRef = "java/io/Closeable".parse().unwrap();
    assert!(implements.iter().any(|it| it == &closeable));
}
