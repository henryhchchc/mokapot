use crate::{
    analysis::fixed_point::JoinSemiLattice,
    ir::generator::{DiscoveryValue, ProvisionalValueId, jvm_frame::entry::Entry},
};

#[test]
fn merge_value_ref() {
    let lhs = Entry::Value(DiscoveryValue::Local(ProvisionalValueId::new(0)));
    let rhs = Entry::Value(DiscoveryValue::Local(ProvisionalValueId::new(1)));

    let result = lhs.join(rhs);
    assert_eq!(result, Entry::Value(DiscoveryValue::Merged));
}

#[test]
fn merge_same_value_ref() {
    let lhs = Entry::Value(DiscoveryValue::Local(ProvisionalValueId::new(0)));
    let rhs = Entry::Value(DiscoveryValue::Local(ProvisionalValueId::new(0)));

    let result = lhs.join(rhs);
    assert_eq!(
        result,
        Entry::Value(DiscoveryValue::Local(ProvisionalValueId::new(0)))
    );
}

#[test]
fn incompatible_legacy_values_become_invalid() {
    let value = Entry::Value(DiscoveryValue::Local(ProvisionalValueId::new(0)));
    let address = Entry::Value(DiscoveryValue::ReturnAddress(
        crate::ir::generator::legacy::ReturnAddress::for_test(0),
    ));

    assert_eq!(value.join(address), Entry::Value(DiscoveryValue::Invalid));
}
