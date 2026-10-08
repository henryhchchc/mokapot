use derive_more::Display;

use super::ValueId;

/// JVM invocation dispatch semantics and the required receiver, without target resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
pub enum InvocationKind {
    /// An `invokestatic` invocation without a receiver.
    #[display("static")]
    Static,
    /// An `invokevirtual` invocation.
    #[display("virtual")]
    Virtual {
        /// The instance receiver.
        this: ValueId,
    },
    /// An `invokeinterface` invocation.
    #[display("interface")]
    Interface {
        /// The instance receiver.
        this: ValueId,
    },
    /// An `invokespecial` invocation.
    #[display("special")]
    Special {
        /// The instance receiver.
        this: ValueId,
    },
}

#[cfg(test)]
mod tests {
    use crate::ir::test::prelude::*;

    #[test]
    fn calls_display_dispatch_kind_and_receiver() {
        let [this, first, second] = ids(0);
        let method = method_ref("accept", "(IJ)V");
        let static_call = "call static void java/lang/Object::accept(%1, %2)";
        let virtual_call = "call virtual void %0@java/lang/Object::accept(%1, %2)";
        let interface_call = "call interface void %0@java/lang/Object::accept(%1, %2)";
        let special_call = "call special void %0@java/lang/Object::accept(%1, %2)";
        for (kind, expected) in [
            (InvocationKind::Static, static_call),
            (InvocationKind::Virtual { this }, virtual_call),
            (InvocationKind::Interface { this }, interface_call),
            (InvocationKind::Special { this }, special_call),
        ] {
            let expression = call(kind, method.clone(), [first, second]);
            assert_eq!(expression.to_string(), expected);
        }
    }
}
