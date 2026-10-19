//! Tests for resolving compiled Java fixtures.

#![cfg(all(java_fixture_tests, feature = "petgraph"))]
#![allow(missing_docs, clippy::ignore_without_reason)]

use mokapot::types::class_name::ClassName;

use mokapot::{
    analysis::ResolutionContext,
    jvm::class_loader::class_paths::{DirectoryClassPath, NopClassPath},
};

mod support;

#[test]
fn interfaces_impl() {
    let app_cp = DirectoryClassPath::new(support::classes_dir());
    let ctx = ResolutionContext::new([app_cp], NopClassPath::EMPTY);
    let my_class: ClassName = "org/mokapot/test/MyClass".parse().unwrap();
    let implements = ctx
        .interface_implementations
        .implemented_interfaces(&my_class);
    let closeable: ClassName = "java/io/Closeable".parse().unwrap();
    assert!(implements.iter().any(|it| it == &closeable));
}
