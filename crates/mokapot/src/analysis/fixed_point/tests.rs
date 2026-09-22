use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
};

use proptest::prelude::*;

use crate::analysis::fixed_point::{DataflowProblem, JoinSemiLattice, solve};

#[derive(Debug, Clone, PartialEq, Eq, proptest_derive::Arbitrary)]
struct TestSet(BTreeSet<u8>);

impl PartialOrd for TestSet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            Some(std::cmp::Ordering::Equal)
        } else if self.0.is_subset(&other.0) {
            Some(std::cmp::Ordering::Less)
        } else if self.0.is_superset(&other.0) {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for TestSet {
    fn join_assign(&mut self, other: Self) -> bool {
        let old_len = self.0.len();
        self.0.extend(other.0);
        self.0.len() != old_len
    }
}

struct RepeatedSuccessors {
    one_calls: usize,
}

impl DataflowProblem for RepeatedSuccessors {
    type Location = u8;
    type Fact = TestSet;
    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(0, TestSet(BTreeSet::new()))]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        _fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        Ok(match location {
            0 => vec![
                (1, TestSet(BTreeSet::from([1]))),
                (1, TestSet(BTreeSet::from([2]))),
            ],
            1 => {
                self.one_calls += 1;
                Vec::new()
            }
            _ => unreachable!("the test problem only names locations 0 and 1"),
        })
    }
}

#[test]
fn worklist_coalesces_repeated_successors() {
    let mut problem = RepeatedSuccessors { one_calls: 0 };

    let facts: BTreeMap<_, _> = solve(&mut problem).expect("infallible analysis");

    assert_eq!(facts[&1], TestSet(BTreeSet::from([1, 2])));
    assert_eq!(problem.one_calls, 1);
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
