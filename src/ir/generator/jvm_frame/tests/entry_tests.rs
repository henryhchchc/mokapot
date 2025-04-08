#[cfg(test)]
use crate::ir::{generator::jvm_frame::entry::Entry, Identifier, LocalValue, Operand};
use std::collections::BTreeSet;

#[test]
fn merge_value_ref() {
    let lhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))));
    let rhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(1))));

    let result = Entry::merge(lhs, rhs);
    assert_eq!(
        result,
        Entry::Value(Operand::Phi(BTreeSet::from([
            Identifier::Local(LocalValue::new(0)),
            Identifier::Local(LocalValue::new(1))
        ])))
    );
}

#[test]
fn merge_same_value_ref() {
    let lhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))));
    let rhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))));

    let result = Entry::merge(lhs, rhs);
    assert_eq!(
        result,
        Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))))
    );
}