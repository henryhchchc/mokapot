use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
};

use super::*;
use crate::ir::test::prelude::TestSet;

struct RepeatedSuccessors {
    one_calls: usize,
}

impl DataflowProblem for RepeatedSuccessors {
    type Location = u8;
    type Fact = TestSet;
    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(0, TestSet::default())]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        _fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        Ok(match location {
            0 => vec![(1, TestSet::from([1])), (1, TestSet::from([2]))],
            1 => {
                self.one_calls += 1;
                Vec::new()
            }
            _ => unreachable!("the test problem only names locations 0 and 1"),
        })
    }
}

#[test]
fn miri_worklist_coalesces_repeated_successors() {
    assert_repeated_successors::<BTreeMap<u8, TestSet>>();
    assert_repeated_successors::<HashMap<u8, TestSet>>();
    assert_repeated_successors::<QueuedFactsMap<u8, TestSet>>();
}

fn assert_repeated_successors<M>()
where
    M: FactsMap<u8, TestSet> + IntoIterator<Item = (u8, TestSet)>,
{
    let mut problem = RepeatedSuccessors { one_calls: 0 };

    let facts: M = solve(&mut problem).expect("infallible analysis");
    let facts: BTreeMap<_, _> = facts.into_iter().collect();

    assert_eq!(facts[&1], TestSet::from([1, 2]));
    assert_eq!(problem.one_calls, 1);
}

struct CyclicProblem;

impl DataflowProblem for CyclicProblem {
    type Location = u8;
    type Fact = TestSet;
    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(0, TestSet::default())]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        let successor = 1 - *location;
        let mut propagated = fact.clone();
        propagated.0.insert(*location);
        Ok([(successor, propagated)])
    }
}

#[test]
fn cyclic_flow_reaches_a_fixed_point() {
    let expected = TestSet::from([0, 1]);
    let facts: BTreeMap<_, _> = solve(&mut CyclicProblem).unwrap();
    assert_eq!(facts.len(), 2);
    assert_eq!(facts[&0], expected);
    assert_eq!(facts[&1], expected);
}

#[test]
fn option_join_handles_bottom_and_concrete_facts() {
    let mut bottom = None;
    assert!(!bottom.join_assign(None));
    assert!(bottom.join_assign(Some(TestSet::from([1]))));
    assert_eq!(bottom, Some(TestSet::from([1])));

    let mut fact = Some(TestSet::from([1]));
    assert!(!fact.join_assign(None));
    assert!(fact.join_assign(Some(TestSet::from([2]))));
    assert_eq!(fact, Some(TestSet::from([1, 2])));
    assert!(!fact.clone().join_assign(fact.clone()));
}
