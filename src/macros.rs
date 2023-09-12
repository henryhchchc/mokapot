#[macro_export]
macro_rules! fill_once {
    ($container:ident, $with:ident, $what:literal) => {
        if $container.replace($with).is_some() {
            Err(ClassFileParsingError::MalformedClassFile(concat!(
                "There should be at most one ",
                $what
            )))?
        }
    };
}
