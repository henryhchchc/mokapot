#![deny(meta_variable_misuse)]

macro_rules! extract_attributes {
    (for $attrs: ident in $env:literal {
         $( let $var: ident: $attr: ident $(as $uw: ident)?, )*
         $( if let $var_true: ident: $attr_true: ident, )*
         $( match $attr_custom: pat => $var_custom: block, )*
         else let $unrecognized:ident
    }) => {
        use crate::jvm::class_file::attribute::Attribute;
        $( let mut $var = None; )*
        $( let mut $var_true = false; )*
        let mut $unrecognized = Vec::new();
        {
            for attr in $attrs {
                match attr? {
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

pub(crate) use extract_attributes;

#[cfg(test)]
mod tests {
    use crate::jvm::{
        class_file::attribute::Attribute,
        errors::{ParseError, ParseErrorKind},
    };

    fn extract_markers(
        attributes: impl IntoIterator<Item = Result<Attribute, ParseError>>,
    ) -> Result<(bool, bool), ParseError> {
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
    fn extraction_rejects_duplicates_unexpected_attributes_and_resolution_errors() {
        use Attribute::{Deprecated, Signature, Synthetic};
        assert_eq!(
            extract_markers([Ok(Synthetic), Ok(Deprecated)]).unwrap(),
            (true, true)
        );
        for attributes in [
            vec![Ok(Synthetic), Ok(Synthetic)],
            vec![Ok(Signature(String::new()))],
            vec![Err(ParseError::malform("invalid attribute"))],
        ] {
            assert_eq!(
                extract_markers(attributes).unwrap_err().kind(),
                ParseErrorKind::Malformed
            );
        }
    }
}
