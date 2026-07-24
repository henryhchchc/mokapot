#[cfg(test)]
use crate::ir::{Identifier, LocalValue, Operand, generator::jvm_frame::entry::Entry};

fn operand(identifiers: impl IntoIterator<Item = Identifier>) -> Operand {
    Operand::try_from_iter(identifiers).expect("test operands must not be empty")
}

#[test]
fn merge_value_ref() {
    let lhs = Entry::Value(Operand::just(Identifier::Local(LocalValue::new(0))));
    let rhs = Entry::Value(Operand::just(Identifier::Local(LocalValue::new(1))));

    let result = Entry::merge(lhs, rhs);
    assert_eq!(
        result,
        Entry::Value(operand([
            Identifier::Local(LocalValue::new(0)),
            Identifier::Local(LocalValue::new(1))
        ]))
    );
}

#[test]
fn merge_same_value_ref() {
    let lhs = Entry::Value(Operand::just(Identifier::Local(LocalValue::new(0))));
    let rhs = Entry::Value(Operand::just(Identifier::Local(LocalValue::new(0))));

    let result = Entry::merge(lhs, rhs);
    assert_eq!(
        result,
        Entry::Value(Operand::just(Identifier::Local(LocalValue::new(0))))
    );
}
