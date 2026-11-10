use std::sync::LazyLock;

use mokapot::{
    ir::MokaIRMethod,
    jvm::{Class, Method},
    types::Descriptor,
};

use crate::jdk_corpus::Corpus;

pub fn methods() -> impl Iterator<Item = Method<'static>> {
    static CLASSES: LazyLock<Vec<Class>> = LazyLock::new(|| {
        Corpus::from_env()
            .classes
            .into_iter()
            .map(|input| {
                Class::from_bytes(&input.bytes)
                    .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name))
            })
            .collect()
    });
    CLASSES
        .iter()
        .flat_map(Class::methods)
        .filter(|method| method.body().is_some())
}

pub fn build_ir(method: Method<'_>) -> MokaIRMethod<'_> {
    MokaIRMethod::from_method(method).unwrap_or_else(|error| {
        panic!(
            "cannot build {}::{}{}: {error}",
            method.owner().name(),
            method.name(),
            method.descriptor().descriptor()
        )
    })
}
