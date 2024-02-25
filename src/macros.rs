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
                        Err(Error::MalformedClassFile(message))?;
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
    ($msg:expr) => {
        Err(Error::MalformedClassFile($msg))?
    };
}

macro_rules! see_jvm_spec {
    ($l1:literal $(, $l2:literal $(, $l3:literal $(, $l4:literal )? )? )?) => {
        concat!(
            "See the [JVM Specification §", $l1,
            $( ".", $l2, $( ".", $l3, $( ".", $l4, )? )? )?
            "](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-",
            $l1,
            ".html#jvms-", $l1,
            $( ".", $l2, $( ".", $l3, $( ".", $l4, )? )? )?
            ") for more information."
        )
    };
}

pub(crate) use extract_attributes;
pub(crate) use malform;
pub(crate) use see_jvm_spec;
