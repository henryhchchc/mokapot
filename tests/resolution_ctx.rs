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
    let ctx = ResolutionContext::new([app_cp], NopClassPath::EMPRY);
    assert!(
        ctx.application_classes
            .contains_key(&ClassRef::new("org/mokapot/test/TestAnalysis"))
    );
}

#[test]
#[cfg_attr(not(integration_test), ignore)]
fn interfaces_impl() {
    let app_cp = DirectoryClassPath::new(TEST_CP);
    let ctx = ResolutionContext::new([app_cp], NopClassPath::EMPRY);
    let implements = ctx
        .interface_implementations
        .implemented_interfaces(&ClassRef::new("org/mokapot/test/MyClass"));
    assert!(
        implements
            .iter()
            .any(|it| it == &ClassRef::new("java/io/Closeable"))
    );
}
