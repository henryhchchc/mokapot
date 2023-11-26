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
    let mut facts = BTreeMap::new();
    let mut dirty_nodes = VecDeque::new();
    let (entry_pc, entry_fact) = analyzer.entry_fact()?;
    analyzer
        .execute_instruction(entry_pc.clone(), &entry_fact)?
        .into_iter()
        .for_each(|it| dirty_nodes.push_back(it));
    facts.insert(entry_pc, entry_fact);

    while let Some((location, incoming_fact)) = dirty_nodes.pop_front() {
        let (merged_fact, is_fact_updated) = match facts.remove(&location) {
            Some(current_fact) => {
                let new_fact = current_fact.merge(incoming_fact).map_err(A::Err::from)?;
                let is_changed = new_fact != current_fact;
                (new_fact, is_changed)
            }
            None => (incoming_fact, true),
        };

        debug_assert!(
            !facts.contains_key(&location),
            "The fact of the location being analyzed should not be in the facts map.",
        );
        let fact: &_ = facts.entry(location.clone()).or_insert(merged_fact);

        if is_fact_updated {
            let subsequent_nodes = analyzer.execute_instruction(location, &fact)?;
            subsequent_nodes
                .into_iter()
                .for_each(|it| dirty_nodes.push_back(it));
        }
    }

    Ok(facts)
}
