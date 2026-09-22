/// Budget knobs for path-condition minimization.
///
/// The exact reducer is only used while the estimated on-set stays below
/// [`on_set_size`](Self::on_set_size). Larger covers fall back to a bounded heuristic reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolvingBudget {
    /// Maximum estimated on-set size for exact minimization.
    pub on_set_size: usize,
    /// Maximum number of heuristic improvement rounds.
    pub heuristic_rounds: usize,
    /// Maximum number of semantic cover checks during one reduction.
    pub cover_checks: usize,
}

impl Default for SolvingBudget {
    fn default() -> Self {
        Self {
            on_set_size: 128,
            heuristic_rounds: 2,
            cover_checks: 8_192,
        }
    }
}
