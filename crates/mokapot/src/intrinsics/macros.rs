#![deny(meta_variable_misuse)]

macro_rules! extract_attributes {
    (for $attrs: ident in $env:literal {
         $( let $var: ident: $attr: ident $(as $uw: ident)?, )*
         $( if let $var_true: ident: $attr_true: ident, )*
         $( match $attr_custom: pat => $var_custom: block, )*
         else let $unrecognized:ident
    }) => {
        use crate::jvm::bytecode::attribute::Attribute;
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
                        Err(ParseError::malform(message))?;
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
                        Err(ParseError::malform(format!("Unexpected attribute. Expected: {}, but got: {}",
                            unexpected.name(),
                            $env
                        )))?;
                    }
                }
            }
        }
        $( $(let $var = $var.$uw();)? )*
    };
}

macro_rules! see_jvm_spec {
    (__latest_jdk) => { 24 };
    ($sec:literal $(, $sub_sec:literal )*) => {
        concat!(
            "See the [JVM Specification §", $sec, $( ".", $sub_sec, )* "]",
            "(https://docs.oracle.com/javase/specs/jvms/se", see_jvm_spec!(__latest_jdk),
            "/html/jvms-", $sec, ".html#jvms-", $sec, $( ".", $sub_sec, )* ") for more information."
        )
    };
}

macro_rules! attributes_into_iter {
    ($val: expr) => {
        [
            Some($val.runtime_visible_annotations)
                .filter(|it| !it.is_empty())
                .map(Attribute::RuntimeVisibleAnnotations),
            Some($val.runtime_invisible_annotations)
                .filter(|it| !it.is_empty())
                .map(Attribute::RuntimeInvisibleAnnotations),
            Some($val.runtime_visible_type_annotations)
                .filter(|it| !it.is_empty())
                .map(Attribute::RuntimeVisibleTypeAnnotations),
            Some($val.runtime_invisible_type_annotations)
                .filter(|it| !it.is_empty())
                .map(Attribute::RuntimeInvisibleTypeAnnotations),
        ]
        .into_iter()
        .flatten()
        .chain(
            $val.other_attributes
                .into_iter()
                .map(|(name, data)| Attribute::Unrecognized(name, data)),
        )
    };
}

pub(crate) use attributes_into_iter;
pub(crate) use extract_attributes;
pub(crate) use see_jvm_spec;
