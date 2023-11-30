use std::collections::{BTreeMap, VecDeque};

pub trait FixedPointFact: PartialEq + Sized {
    type MergeErr;
    fn merge(&self, other: Self) -> Result<Self, Self::MergeErr>;
}

pub trait FixedPointAnalyzer {
    type Location: Ord + Eq + Clone;
    type Fact: FixedPointFact;
    type Err: From<<Self::Fact as FixedPointFact>::MergeErr>;

    fn entry_fact(&self) -> Result<(Self::Location, Self::Fact), Self::Err>;
    fn execute_instruction(
        &mut self,
        location: Self::Location,
        fact: &Self::Fact,
    ) -> Result<BTreeMap<Self::Location, Self::Fact>, Self::Err>;
}

pub fn analyze<A>(analyzer: &mut A) -> Result<BTreeMap<A::Location, A::Fact>, A::Err>
where
    A: FixedPointAnalyzer,
{
    let mut facts: BTreeMap<A::Location, A::Fact> = BTreeMap::new();
    let entry_node = analyzer.entry_fact()?;
    let mut dirty_nodes = VecDeque::from([entry_node]);

    while let Some((location, incoming_fact)) = dirty_nodes.pop_front() {
        let maybe_updated_fact = match facts.get(&location) {
            Some(current_fact) => {
                let merged_fact = current_fact.merge(incoming_fact).map_err(A::Err::from)?;
                Some(merged_fact).filter(|it| it.ne(current_fact))
            }
            None => Some(incoming_fact),
        };

        if let Some(fact) = maybe_updated_fact {
            dirty_nodes.extend(analyzer.execute_instruction(location.clone(), &fact)?);
            facts.insert(location, fact);
        }
    }

    Ok(facts)
}
