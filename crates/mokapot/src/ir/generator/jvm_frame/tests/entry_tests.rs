use crate::{
    analysis::fixed_point::JoinSemiLattice,
    ir::generator::{OperandState, ReturnAddress, SsaValueId, jvm_frame::entry::Entry},
};

#[test]
fn merge_value_ref() {
    let lhs = Entry::Value(OperandState::Local(SsaValueId::new(0)));
    let rhs = Entry::Value(OperandState::Local(SsaValueId::new(1)));

    let result = lhs.join(rhs);
    assert_eq!(result, Entry::Value(OperandState::Merged));
}

#[test]
fn merge_same_value_ref() {
    let lhs = Entry::Value(OperandState::Local(SsaValueId::new(0)));
    let rhs = Entry::Value(OperandState::Local(SsaValueId::new(0)));

    let result = lhs.join(rhs);
    assert_eq!(
        result,
        Entry::Value(OperandState::Local(SsaValueId::new(0)))
    );
}

#[test]
fn incompatible_legacy_values_become_invalid() {
    let value = Entry::Value(OperandState::Local(SsaValueId::new(0)));
    let address = Entry::Value(OperandState::ReturnAddress(ReturnAddress::for_test(0)));

    assert_eq!(value.join(address), Entry::Value(OperandState::Invalid));
}
