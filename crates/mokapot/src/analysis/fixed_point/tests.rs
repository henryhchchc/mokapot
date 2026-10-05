use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
};

use proptest::prelude::*;

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
fn cyclic_flow_reaches_the_same_fixed_point_with_each_facts_map() {
    let expected = TestSet::from([0, 1]);
    for facts in [
        solve::<_, BTreeMap<u8, TestSet>>(&mut CyclicProblem)
            .unwrap()
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
        solve::<_, HashMap<u8, TestSet>>(&mut CyclicProblem)
            .unwrap()
            .into_iter()
            .collect(),
        solve::<_, QueuedFactsMap<u8, TestSet>>(&mut CyclicProblem)
            .unwrap()
            .into_iter()
            .collect(),
    ] {
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[&0], expected);
        assert_eq!(facts[&1], expected);
    }
}

proptest! {
   #[test]
   fn option_join_ordering(
       lhs in any::<Option<TestSet>>(),
       rhs in any::<Option<TestSet>>(),
   ) {
       let mut joined = lhs.clone();
       let changed = joined.join_assign(rhs.clone());
       prop_assert!(joined >= lhs);
       prop_assert!(joined >= rhs);
       prop_assert_eq!(changed, joined != lhs);

       // The join is commutative and idempotent.
       let mut commuted = rhs.clone();
       commuted.join_assign(lhs.clone());
       prop_assert_eq!(&joined, &commuted);
       prop_assert!(!joined.clone().join_assign(joined.clone()));
   }
}
