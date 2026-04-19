use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use super::{
    absorb,
    indexed::{AtomTable, IndexedCube, LiteralState, absorb_indexed, indexed_cover_cost},
};
use crate::ir::control_flow::path_condition::{PathConditionBudget, cube::Cube};

pub(super) fn heuristic_minimize<P>(
    cubes: &HashSet<Cube<P>>,
    atoms: &AtomTable<P>,
    budget: PathConditionBudget,
) -> HashSet<Cube<P>>
where
    P: Hash + Eq + Clone,
{
    let mut current = absorb_indexed(
        cubes
            .iter()
            .map(|cube| IndexedCube::from_cube(cube, atoms))
            .collect(),
    );
    if current.len() <= 1 {
        return current
            .into_iter()
            .map(|cube| cube.to_cube(atoms))
            .collect::<HashSet<_>>();
    }

    let mut current_cost = indexed_cover_cost(&current);
    let mut stats = HeuristicStats::default();

    for _round in 0..budget.max_heuristic_rounds {
        let expanded = heuristic_expand(&current, budget, &mut stats);
        let candidate = heuristic_irredundant(expanded, budget, &mut stats);
        let candidate_cost = indexed_cover_cost(&candidate);

        if candidate_cost < current_cost {
            current = candidate;
            current_cost = candidate_cost;
        } else {
            break;
        }

        if stats.cover_checks >= budget.max_cover_checks {
            break;
        }
    }

    absorb(
        current
            .into_iter()
            .map(|cube| cube.to_cube(atoms))
            .collect(),
    )
}

fn heuristic_expand(
    cover: &[IndexedCube],
    budget: PathConditionBudget,
    stats: &mut HeuristicStats,
) -> Vec<IndexedCube> {
    let reference = absorb_indexed(cover.to_vec());
    let mut memo = HashMap::new();
    let mut expanded = Vec::with_capacity(reference.len());

    for (cube_index, cube) in reference.iter().enumerate() {
        let mut candidate = cube.clone();
        let specified_indices = candidate.specified_indices().collect::<Vec<_>>();

        for index in specified_indices {
            let generalized = candidate.generalize(index);
            let Some(is_covered) =
                indexed_cover_covers_cube(&reference, &generalized, &mut memo, budget, stats)
            else {
                expanded.push(candidate);
                expanded.extend(reference.iter().skip(cube_index + 1).cloned());
                return absorb_indexed(expanded);
            };

            if is_covered {
                candidate = generalized;
            }
        }

        expanded.push(candidate);
        if stats.cover_checks >= budget.max_cover_checks {
            expanded.extend(reference.iter().skip(cube_index + 1).cloned());
            break;
        }
    }

    absorb_indexed(expanded)
}

fn heuristic_irredundant(
    cover: Vec<IndexedCube>,
    budget: PathConditionBudget,
    stats: &mut HeuristicStats,
) -> Vec<IndexedCube> {
    let cover = absorb_indexed(cover);
    let mut irredundant = Vec::with_capacity(cover.len());

    for (cube_index, cube) in cover.iter().enumerate() {
        let rest = cover
            .iter()
            .enumerate()
            .filter(|(other_index, _)| *other_index != cube_index)
            .map(|(_, other)| other.clone())
            .collect::<Vec<_>>();
        let mut memo = HashMap::new();

        let Some(is_covered) = indexed_cover_covers_cube(&rest, cube, &mut memo, budget, stats)
        else {
            irredundant.extend(cover.iter().skip(cube_index).cloned());
            return absorb_indexed(irredundant);
        };

        if !is_covered {
            irredundant.push(cube.clone());
        }

        if stats.cover_checks >= budget.max_cover_checks {
            irredundant.extend(cover.iter().skip(cube_index + 1).cloned());
            break;
        }
    }

    absorb_indexed(irredundant)
}

fn indexed_cover_covers_cube(
    cover: &[IndexedCube],
    cube: &IndexedCube,
    memo: &mut HashMap<IndexedCube, bool>,
    budget: PathConditionBudget,
    stats: &mut HeuristicStats,
) -> Option<bool> {
    if let Some(result) = memo.get(cube) {
        return Some(*result);
    }
    if !stats.try_take_cover_check(budget) {
        return None;
    }

    let result = if cover.iter().any(|existing| existing.subsumes(cube)) {
        true
    } else {
        let split_index = cover
            .iter()
            .filter(|existing| !existing.conflicts_with(cube))
            .flat_map(IndexedCube::specified_indices)
            .find(|index| cube.literal(*index) == LiteralState::DontCare);
        let Some(split_index) = split_index else {
            return Some(false);
        };

        let positive = indexed_cover_covers_cube(
            cover,
            &cube.with_literal(split_index, LiteralState::Positive),
            memo,
            budget,
            stats,
        )?;
        let negative = indexed_cover_covers_cube(
            cover,
            &cube.with_literal(split_index, LiteralState::Negative),
            memo,
            budget,
            stats,
        )?;
        positive && negative
    };

    memo.insert(cube.clone(), result);
    Some(result)
}

#[derive(Debug, Default)]
struct HeuristicStats {
    cover_checks: usize,
}

impl HeuristicStats {
    const fn try_take_cover_check(&mut self, budget: PathConditionBudget) -> bool {
        if self.cover_checks >= budget.max_cover_checks {
            false
        } else {
            self.cover_checks += 1;
            true
        }
    }
}
