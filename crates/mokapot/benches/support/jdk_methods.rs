use mokapot::{
    ir::MokaIRMethod,
    jvm::{Class, Method},
    types::Descriptor,
};

use crate::jdk_corpus::Corpus;

#[allow(dead_code, reason = "Used by the jdk-ir benchmark target")]
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

#[allow(dead_code, reason = "Used by the jdk-path-conditions benchmark target")]
pub fn for_each_method(mut visit: impl FnMut(Method<'_>)) {
    for input in Corpus::from_env().classes {
        let class = Class::from_bytes(&input.bytes)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
        for method in class.methods().filter(|method| method.body().is_some()) {
            visit(method);
        }
    }
}

pub fn build_ir(method: Method<'_>) -> MokaIRMethod {
    MokaIRMethod::from_method(method).unwrap_or_else(|error| {
        panic!(
            "cannot build {}::{}{}: {error}",
            method.owner().name(),
            method.name(),
            method.descriptor().descriptor()
        )
    })
}
