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
    fn merge(&self, other: Self) -> Result<Self, Self::MergeError>;
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
    let mut to_analyze = LinkedList::new();
    let body = method.body.as_ref().expect("TODO");

    let Some((entry_pc, entry_insn)) = body.instructions.first() else {
        return Ok(facts_before_insn);
    };
    let entry_fact = analyzer.entry_fact(method);
    analyzer
        .execute_instruction(body, *entry_pc, entry_insn, &entry_fact)?
        .into_iter()
        .for_each(|it| to_analyze.push_back(it));
    facts_before_insn.insert(*entry_pc, entry_fact);

    while let Some((pc, to_be_merged)) = to_analyze.pop_front() {
        let insn = body.instruction_at(pc).expect("TODO: Raise error");

        let old_fact = facts_before_insn.remove(&pc);
        let (new_fact, fact_changed) = match (old_fact, to_be_merged) {
            (Some(f), nf) => {
                let new_fact = f.merge(nf)?;
                let changed = new_fact != f;
                (new_fact, changed)
            }
            (None, nf) => (nf, true),
        };

        facts_before_insn.insert(pc, new_fact);
        let fact = facts_before_insn.get(&pc).unwrap();

        if fact_changed {
            let dirty_facts = analyzer.execute_instruction(body, pc, insn, &fact)?;
            dirty_facts
                .into_iter()
                .for_each(|it| to_analyze.push_back(it));
        }
    }

    Ok(facts_before_insn)
}
