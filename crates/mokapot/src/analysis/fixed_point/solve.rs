use super::{DataflowProblem, FactsMap};

/// Computes the fixed point of `problem` by worklist iteration.
///
/// Seeds the worklist, then repeatedly joins a pending fact into the result map
/// and, when it changed, flows it to the successors. Returns the final fact at
/// every location.
///
/// The map type `M` is inferred from the return type, so it selects the
/// container:
///
/// ```ignore
/// let results: BTreeMap<_, _> = solve(&problem)?; // `Location: Ord`
/// let results: HashMap<_, _> = solve(&problem)?; // `Location: Hash + Eq`
/// ```
///
/// # Errors
///
/// Propagates errors from [`DataflowProblem::flow`].
///
/// # Termination
///
/// Terminates when the lattice has finite height and the flow function is
/// monotonic.
#[instability::unstable(feature = "fixed-point-analyses")]
pub fn solve<P, M>(problem: &mut P) -> Result<M, P::Err>
where
    P: DataflowProblem,
    M: FactsMap<P::Location, P::Fact>,
{
    let mut facts = M::default();
    let mut worklist = M::default();

    for (location, fact) in problem.seeds() {
        worklist.insert_or_join(location, fact);
    }

    while let Some((location, incoming)) = worklist.pop_one() {
        let Some((location, fact)) = facts.insert_or_join(location, incoming) else {
            continue;
        };
        for (successor, propagated) in problem.flow(location, fact)? {
            worklist.insert_or_join(successor, propagated);
        }
    }

    Ok(facts)
}
