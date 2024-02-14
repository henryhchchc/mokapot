#![deny(meta_variable_misuse)]

macro_rules! extract_attributes {
    (for $attrs: ident in $env:literal by {
         $( let $var: ident: $attr: ident $($uw: ident)?, )*
         $( if let $var_true: ident: $attr_true: ident, )*
         $( match $attr_custom: pat => $var_custom: block, )*
    }) => {
        use crate::jvm::parsing::attribute::Attribute;
        $( let mut $var = None; )*
        $( let mut $var_true = false; )*
        {
            for attr in $attrs {
                match attr {
                $(
                    Attribute::$attr(it) => if $var.replace(it).is_some() {
                        Err(ClassFileParsingError::MalformedClassFile(concat!(
                            "There should be at most one ",
                            stringify!($attr),
                            " in a ",
                            $env
                        )))?;
                    },
                )*
                $(
                    Attribute::$attr_true => {
                        $var_true = true;
                    },
                )*
                $($attr_custom => $var_custom,)*
                    unexpected => {
                        Err(ClassFileParsingError::UnexpectedAttribute(unexpected.name(), $env))?;
                    }
                }
            }
        }
        $( $(let $var = $var.$uw();)? )*
    };
}

pub(crate) use extract_attributes;
