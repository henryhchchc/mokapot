use std::collections::{HashMap, LinkedList};

pub trait FixedPointFact: PartialEq + Sized {
    type MergeError;
    fn merge(&self, other: Self) -> Result<Self, Self::MergeError>;
}

pub trait FixedPointAnalyzer {
    type Location: std::hash::Hash + Eq + Copy;
    type Fact: FixedPointFact;
    type Error: From<<Self::Fact as FixedPointFact>::MergeError>;

    fn entry_fact(&self) -> Result<(Self::Location, Self::Fact), Self::Error>;
    fn execute_instruction(
        &mut self,
        location: Self::Location,
        fact: &Self::Fact,
    ) -> Result<HashMap<Self::Location, Self::Fact>, Self::Error>;
}

pub fn analyze<A>(analyzer: &mut A) -> Result<HashMap<A::Location, A::Fact>, A::Error>
where
    A: FixedPointAnalyzer,
{
    let mut facts = HashMap::new();
    let mut dirty_nodes = LinkedList::new();
    let (entry_pc, entry_fact) = analyzer.entry_fact()?;
    analyzer
        .execute_instruction(entry_pc, &entry_fact)?
        .into_iter()
        .for_each(|it| dirty_nodes.push_back(it));
    facts.insert(entry_pc, entry_fact);

    while let Some((location, incoming_fact)) = dirty_nodes.pop_front() {
        let (merged_fact, is_fact_updated) = match facts.remove(&location) {
            Some(current_fact) => {
                let new_fact = current_fact.merge(incoming_fact)?;
                let is_changed = new_fact != current_fact;
                (new_fact, is_changed)
            }
            None => (incoming_fact, true),
        };

        facts.insert(location, merged_fact);

        if is_fact_updated {
            let fact = facts
                .get(&location)
                .expect("BUG: the merged fact is not inserted to the map");
            let subsequent_nodes = analyzer.execute_instruction(location, &fact)?;
            subsequent_nodes
                .into_iter()
                .for_each(|it| dirty_nodes.push_back(it));
        }
    }

    Ok(facts)
}
