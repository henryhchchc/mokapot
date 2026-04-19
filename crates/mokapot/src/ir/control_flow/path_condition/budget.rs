/// Budget knobs for path-condition minimization.
///
/// The exact reducer is only used while the estimated on-set stays below
/// [`max_exact_on_set_size`](Self::max_exact_on_set_size). Larger covers fall
/// back to a bounded heuristic reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathConditionBudget {
    /// Maximum estimated on-set size for exact minimization.
    pub max_exact_on_set_size: usize,
    /// Maximum number of heuristic improvement rounds.
    pub max_heuristic_rounds: usize,
    /// Maximum number of semantic cover checks during one reduction.
    pub max_cover_checks: usize,
}

impl Default for PathConditionBudget {
    fn default() -> Self {
        Self {
            max_exact_on_set_size: 128,
            max_heuristic_rounds: 2,
            max_cover_checks: 8_192,
        }
    }
}
