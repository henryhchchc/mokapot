use std::collections::{BTreeMap, LinkedList};

use crate::elements::{
    instruction::{Instruction, MethodBody, ProgramCounter},
    Method,
};

pub trait FixedPointAnalyzer {
    type Fact: FixedPointFact;
    type Error;
    fn entry_fact(&self, method: &Method) -> Self::Fact;
    fn execute_instruction(
        &mut self,
        body: &MethodBody,
        pc: ProgramCounter,
        insn: &Instruction,
        fact: &Self::Fact,
    ) -> Result<BTreeMap<ProgramCounter, Self::Fact>, Self::Error>;
}

pub trait FixedPointFact: PartialEq + Sized {
    type MergeError;
    fn merge(&self, other: &Self) -> Result<Self, Self::MergeError>;
}

pub fn analyze<'a, 'm, A>(
    method: &'m Method,
    analyzer: &'a mut A,
) -> Result<BTreeMap<ProgramCounter, A::Fact>, A::Error>
where
    A: FixedPointAnalyzer,
    <A as FixedPointAnalyzer>::Error:
        From<<<A as FixedPointAnalyzer>::Fact as FixedPointFact>::MergeError>,
{
    let mut facts_before_insn = BTreeMap::new();
    let mut dirty_nodes = LinkedList::new();
    let body = method.body.as_ref().expect("TODO");

    let Some((entry_pc, entry_insn)) = body.instructions.first() else {
        return Ok(facts_before_insn);
    };
    let entry_fact = analyzer.entry_fact(method);
    analyzer
        .execute_instruction(body, *entry_pc, entry_insn, &entry_fact)?
        .into_iter()
        .for_each(|it| dirty_nodes.push_back(it));
    facts_before_insn.insert(*entry_pc, entry_fact);

    while let Some((pc, new_fact)) = dirty_nodes.pop_front() {
        let insn = body.instruction_at(pc).expect("TODO: Raise error");

        let (merged_fact, is_fact_updated) = match facts_before_insn.remove(&pc) {
            Some(current_fact) => {
                let new_fact = current_fact.merge(&new_fact)?;
                let is_changed = new_fact != current_fact;
                (new_fact, is_changed)
            }
            None => (new_fact, true),
        };

        facts_before_insn.insert(pc, merged_fact);
        let fact = facts_before_insn.get(&pc).unwrap();

        if is_fact_updated {
            let subsequent_nodes = analyzer.execute_instruction(body, pc, insn, &fact)?;
            subsequent_nodes
                .into_iter()
                .for_each(|it| dirty_nodes.push_back(it));
        }
    }

    Ok(facts_before_insn)
}
