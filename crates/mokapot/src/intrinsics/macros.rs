#![deny(meta_variable_misuse)]

macro_rules! see_jvm_spec {
    (__latest_jdk) => { 27 };
    ($sec:literal $(, $sub_sec:literal )*) => {
        concat!(
            "See the [JVM Specification §", $sec, $( ".", $sub_sec, )* "]",
            "(https://docs.oracle.com/javase/specs/jvms/se", see_jvm_spec!(__latest_jdk),
            "/html/jvms-", $sec, ".html#jvms-", $sec, $( ".", $sub_sec, )* ") for more information."
        )
    };
}

pub(crate) use see_jvm_spec;
