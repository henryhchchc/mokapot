use std::collections::{BTreeMap, LinkedList};

use crate::elements::instruction::{Instruction, MethodBody, ProgramCounter};

pub trait FixedPointAnalyzer<F> {
    fn new_fact(&self, body: &MethodBody) -> F;
    fn execute_instruction(
        &mut self,
        body: &MethodBody,
        pc: ProgramCounter,
        insn: &Instruction,
        fact: &F,
    ) -> BTreeMap<ProgramCounter, F>;
}

pub trait FixedPointFact: PartialEq {
    fn merge(&self, other: Self) -> Self;
}

pub fn analyze<'a, 'b, A, F>(
    body: &'b MethodBody,
    analyzer: &'a mut A,
) -> BTreeMap<ProgramCounter, F>
where
    A: FixedPointAnalyzer<F>,
    F: FixedPointFact,
{
    let mut facts_before_insn = BTreeMap::new();
    let mut to_analyze = LinkedList::new();

    let Some((entry_pc, entry_insn)) = body.instructions.first() else {
        return facts_before_insn;
    };
    let entry_fact = analyzer.new_fact(body);
    analyzer
        .execute_instruction(body, *entry_pc, entry_insn, &entry_fact)
        .into_iter()
        .for_each(|it| to_analyze.push_back(it));
    facts_before_insn.insert(*entry_pc, entry_fact);

    while let Some((pc, to_be_merged)) = to_analyze.pop_front() {
        let insn = body.instruction_at(pc).expect("TODO: Raise error");

        let mut fact_changed = false;
        let old_fact = facts_before_insn.remove(&pc).unwrap_or_else(|| {
            fact_changed = true;
            analyzer.new_fact(body)
        });
        let new_fact = old_fact.merge(to_be_merged);
        fact_changed |= new_fact != old_fact;

        facts_before_insn.insert(pc, new_fact);
        let fact = facts_before_insn.get(&pc).unwrap();

        if fact_changed {
            let dirty_facts = analyzer.execute_instruction(body, pc, insn, &fact);
            dirty_facts
                .into_iter()
                .for_each(|it| to_analyze.push_back(it));
        }
    }

    facts_before_insn
}
