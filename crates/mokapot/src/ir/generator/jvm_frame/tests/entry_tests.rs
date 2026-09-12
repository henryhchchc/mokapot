use crate::{
    analysis::fixed_point::JoinSemiLattice,
    ir::generator::{OperandState, ReturnAddress, SsaValueId, jvm_frame::entry::Entry},
};

#[test]
fn context_free_value_conflict_is_invalid() {
    let lhs = Entry::Value(OperandState::Value(SsaValueId::new(0)));
    let rhs = Entry::Value(OperandState::Value(SsaValueId::new(1)));

    let result = lhs.join(rhs);
    assert_eq!(result, Entry::Value(OperandState::Invalid));
}

#[test]
fn merge_same_value_ref() {
    let lhs = Entry::Value(OperandState::Value(SsaValueId::new(0)));
    let rhs = Entry::Value(OperandState::Value(SsaValueId::new(0)));

    let result = lhs.join(rhs);
    assert_eq!(
        result,
        Entry::Value(OperandState::Value(SsaValueId::new(0)))
    );
}

#[test]
fn incompatible_legacy_values_become_invalid() {
    let value = Entry::Value(OperandState::Value(SsaValueId::new(0)));
    let address = Entry::Value(OperandState::ReturnAddress(ReturnAddress::for_test(0)));

    assert_eq!(value.join(address), Entry::Value(OperandState::Invalid));
}

#[test]
fn a_value_missing_on_one_path_is_unavailable() {
    let value = Entry::Value(OperandState::Value(SsaValueId::new(0)));

    assert_eq!(
        value.clone().join(Entry::UninitializedLocal),
        Entry::UninitializedLocal
    );
    assert_eq!(
        Entry::UninitializedLocal.join(value),
        Entry::UninitializedLocal
    );
}
