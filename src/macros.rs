#![deny(meta_variable_misuse)]

macro_rules! extract_attributes {
    (for $attrs: ident in $env:literal {
         $( let $var: ident: $attr: ident $(as $uw: ident)?, )*
         $( if let $var_true: ident: $attr_true: ident, )*
         $( match $attr_custom: pat => $var_custom: block, )*
         else let $unrecognized:ident
    }) => {
        use crate::jvm::parsing::attribute::Attribute;
        $( let mut $var = None; )*
        $( let mut $var_true = false; )*
        let mut $unrecognized = Vec::new();
        {
            for attr in $attrs {
                match attr {
                $(
                    Attribute::$attr(it) => if $var.replace(it).is_some() {
                        let message = concat!(
                            "There should be at most one ",
                            stringify!($attr),
                            " in a ",
                            $env
                        );
                        Err(Error::Other(message))?;
                    },
                )*
                $(
                    Attribute::$attr_true => {
                        $var_true = true;
                    },
                )*
                $($attr_custom => $var_custom,)*
                    Attribute::Unrecognized(name, bytes) => {
                        $unrecognized.push((name, bytes));
                    }
                    unexpected => {
                        Err(Error::UnexpectedAttribute(
                            unexpected.name().to_owned(),
                            $env.to_owned()
                        ))?;
                    }
                }
            }
        }
        $( $(let $var = $var.$uw();)? )*
    };
}

macro_rules! malform {
    ($msg:expr_2021) => {
        Err(Error::Other($msg))?
    };
}

macro_rules! see_jvm_spec {
    (__latest_jdk) => { 23 };
    ($sec:literal $(, $sub_sec:literal )*) => {
        concat!(
            "See the [JVM Specification §", $sec, $( ".", $sub_sec, )* "]",
            "(https://docs.oracle.com/javase/specs/jvms/se", see_jvm_spec!(__latest_jdk),
            "/html/jvms-", $sec, ".html#jvms-", $sec, $( ".", $sub_sec, )* ") for more information."
        )
    };
}

pub(crate) use extract_attributes;
pub(crate) use malform;
pub(crate) use see_jvm_spec;
