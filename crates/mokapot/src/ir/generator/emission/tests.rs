use super::*;

#[test]
fn allocation_rejects_duplicate_temporary_values() {
    let mut allocation = Allocation::default();

    assert_eq!(
        allocation
            .value(SsaValueId::new(0), ValueDefinition::This)
            .expect("first assignment succeeds"),
        ValueId::new(0)
    );
    assert!(matches!(
        allocation.value(SsaValueId::new(0), ValueDefinition::This),
        Err(Error::MalformedControlFlow)
    ));
}

#[test]
fn allocation_preserves_sparse_temporary_value_gaps() {
    let mut allocation = Allocation::default();
    let assigned = allocation
        .value(SsaValueId::new(3), ValueDefinition::This)
        .expect("sparse assignment succeeds");

    assert_eq!(assigned, ValueId::new(0));
    assert_eq!(allocation.values, vec![None, None, None, Some(assigned)]);
    assert_eq!(
        allocation
            .resolve(SsaValueId::new(3))
            .expect("assigned value resolves"),
        assigned
    );
    assert!(matches!(
        allocation.resolve(SsaValueId::new(2)),
        Err(Error::MalformedControlFlow)
    ));
    assert!(matches!(
        allocation.resolve(SsaValueId::new(4)),
        Err(Error::MalformedControlFlow)
    ));
}
