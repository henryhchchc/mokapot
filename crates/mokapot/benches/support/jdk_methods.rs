use mokapot::{
    ir::MokaIRMethod,
    jvm::{Class, Method},
    types::Descriptor,
};

use crate::jdk_corpus::Corpus;

pub fn for_each_method(mut visit: impl FnMut(Method)) {
    // Consume inputs so only one parsed class and the retained benchmark inputs remain live.
    for input in Corpus::from_env().classes {
        let class = Class::from_bytes(&input.bytes)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", input.name));
        for method in class.methods {
            if method.body.is_some() {
                visit(method);
            }
        }
    }
}

pub fn build_ir(method: &Method) -> MokaIRMethod {
    MokaIRMethod::from_method(method).unwrap_or_else(|error| {
        panic!(
            "cannot build {}::{}{}: {error}",
            method.owner,
            method.name,
            method.descriptor.descriptor()
        )
    })
}
