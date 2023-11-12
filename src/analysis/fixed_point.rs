use std::collections::{HashMap, LinkedList};

pub trait FixedPointAnalyzer {
    type Location: std::hash::Hash + Eq + Copy;
    type Fact: FixedPointFact;
    type Error;
    fn entry_fact(&self) -> Result<(Self::Location, Self::Fact), Self::Error>;
    fn execute_instruction(
        &mut self,
        pc: Self::Location,
        fact: &Self::Fact,
    ) -> Result<HashMap<Self::Location, Self::Fact>, Self::Error>;
}

pub trait FixedPointFact: PartialEq + Sized {
    type MergeError;
    fn merge(&self, other: Self) -> Result<Self, Self::MergeError>;
}

pub fn analyze<A>(analyzer: &mut A) -> Result<HashMap<A::Location, A::Fact>, A::Error>
where
    A: FixedPointAnalyzer,
    <A as FixedPointAnalyzer>::Error:
        From<<<A as FixedPointAnalyzer>::Fact as FixedPointFact>::MergeError>,
{
    let mut facts_before_location = HashMap::new();
    let mut dirty_nodes = LinkedList::new();
    let (entry_pc, entry_fact) = analyzer.entry_fact()?;
    analyzer
        .execute_instruction(entry_pc, &entry_fact)?
        .into_iter()
        .for_each(|it| dirty_nodes.push_back(it));
    facts_before_location.insert(entry_pc, entry_fact);

    while let Some((pc, incoming_fact)) = dirty_nodes.pop_front() {
        let (merged_fact, is_fact_updated) = match facts_before_location.remove(&pc) {
            Some(current_fact) => {
                let new_fact = current_fact.merge(incoming_fact)?;
                let is_changed = new_fact != current_fact;
                (new_fact, is_changed)
            }
            None => (incoming_fact, true),
        };

        facts_before_location.insert(pc, merged_fact);
        let fact = facts_before_location.get(&pc).unwrap();

        if is_fact_updated {
            let subsequent_nodes = analyzer.execute_instruction(pc, &fact)?;
            subsequent_nodes
                .into_iter()
                .for_each(|it| dirty_nodes.push_back(it));
        }
    }

    Ok(facts_before_location)
}
