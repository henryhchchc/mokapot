#![cfg(integration_test)]

use mokapot::{
    analysis::ResolutionContext,
    jvm::{class_loader::class_paths::DirectoryClassPath, references::ClassRef},
};

const TEST_CP: &str = concat!(env!("OUT_DIR"), "/mokapot/java_classes");

#[test]
fn load_classes() {
    let app_cp = DirectoryClassPath::new(TEST_CP);
    let ctx = ResolutionContext::new(&[app_cp], &[]);
    assert!(
        ctx.application_classes
            .contains_key(&ClassRef::new("org/mokapot/test/TestAnalysis"))
    );
}

#[test]
fn interfaces_impl() {
    let app_cp = DirectoryClassPath::new(TEST_CP);
    let ctx = ResolutionContext::new(&[app_cp], &[]);
    let implements = ctx
        .interface_implementations
        .implemented_interfaces(&ClassRef::new("org/mokapot/test/MyClass"));
    assert!(
        implements
            .iter()
            .any(|it| it == &ClassRef::new("java/io/Closeable"))
    );
}
