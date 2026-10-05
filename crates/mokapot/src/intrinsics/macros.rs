#![deny(meta_variable_misuse)]

macro_rules! extract_attributes {
    (for $attrs: ident in $env:literal {
         $( let $var: ident: $attr: ident $(as $uw: ident)?, )*
         $( if let $var_true: ident: $attr_true: ident, )*
         $( match $attr_custom: pat => $var_custom: block, )*
         else let $unrecognized:ident
    }) => {
        use crate::jvm::bytecode::Attribute;
        $( let mut $var = None; )*
        $( let mut $var_true = false; )*
        let mut $unrecognized = Vec::new();
        {
            for attr in $attrs {
                match attr {
                $(
                    Attribute::$attr(it) => if $var.is_none() {
                        $var = Some(it);
                    } else {
                        let message = concat!(
                            "There should be at most one ",
                            stringify!($attr),
                            " in a ",
                            $env
                        );
                        Err($crate::jvm::errors::ParseError::malform(message))?;
                    },
                )*
                $(
                    Attribute::$attr_true => {
                        if $var_true {
                            let message = concat!(
                                "There should be at most one ",
                                stringify!($attr_true),
                                " in a ",
                                $env
                            );
                            Err($crate::jvm::errors::ParseError::malform(message))?;
                        }
                        $var_true = true;
                    },
                )*
                $($attr_custom => $var_custom,)*
                    Attribute::Unrecognized(name, bytes) => {
                        $unrecognized.push((name, bytes));
                    }
                    unexpected => {
                        Err($crate::jvm::errors::ParseError::malform(format!("Unexpected attribute. Expected: {}, but got: {}",
                            $env,
                            unexpected.name()
                        )))?;
                    }
                }
            }
        }
        $( $(let $var = $var.$uw();)? )*
    };
}

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

macro_rules! attributes_into_iter {
    ($val: expr) => {
        [
            Some($val.annotations.runtime_visible)
                .filter(|it| !it.is_empty())
                .map(Attribute::RuntimeVisibleAnnotations),
            Some($val.annotations.runtime_invisible)
                .filter(|it| !it.is_empty())
                .map(Attribute::RuntimeInvisibleAnnotations),
            Some($val.type_annotations.runtime_visible)
                .filter(|it| !it.is_empty())
                .map(Attribute::RuntimeVisibleTypeAnnotations),
            Some($val.type_annotations.runtime_invisible)
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

#[cfg(test)]
mod tests {
    use crate::jvm::bytecode::{Attribute, ParseError, ParseErrorKind};

    fn extract_markers(attributes: Vec<Attribute>) -> Result<(bool, bool), ParseError> {
        extract_attributes! {
            for attributes in "test_attributes" {
                if let is_synthetic: Synthetic,
                if let is_deprecated: Deprecated,
                else let _other_attributes
            }
        }
        Ok((is_synthetic, is_deprecated))
    }

    #[test]
    fn rejects_duplicate_marker_attributes() {
        use crate::jvm::bytecode::Attribute::{Deprecated, Synthetic};
        let error = extract_markers(vec![Synthetic, Synthetic])
            .expect_err("duplicate marker attributes must be rejected");
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
        let error = extract_markers(vec![Deprecated, Deprecated])
            .expect_err("duplicate marker attributes must be rejected");
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }

    #[test]
    fn rejects_unexpected_attributes() {
        let signature = Attribute::Signature(String::new());
        extract_markers(vec![signature]).expect_err("an unexpected attribute must be rejected");
    }
}
