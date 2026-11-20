use mokapot::{
    ir::MokaIRMethod,
    jvm::{Class, Method},
    types::Descriptor,
};

use crate::jdk_corpus::Corpus;

pub fn parse_classes() -> Vec<Class> {
    Corpus::from_env()
        .classes
        .into_iter()
        .map(|input| {
            Class::from_bytes(&input.bytes)
                .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name))
        })
        .collect()
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
