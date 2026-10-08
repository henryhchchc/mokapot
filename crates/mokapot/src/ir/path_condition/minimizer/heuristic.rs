use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use super::{
    Cube, SolvingBudget, absorb,
    indexed::{AtomTable, IndexedCube, LiteralState, absorb_indexed, indexed_cover_cost},
};

pub(super) fn heuristic_minimize<P>(
    cubes: &HashSet<Cube<P>>,
    atoms: &AtomTable<P>,
    budget: SolvingBudget,
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

    let mut stats = HeuristicStats::default();

    for _round in 0..budget.heuristic_rounds {
        let expanded = heuristic_expand(&current, budget, &mut stats);
        let candidate = heuristic_irredundant(expanded, budget, &mut stats);

        if indexed_cover_cost(&candidate) < indexed_cover_cost(&current) {
            current = candidate;
        } else {
            break;
        }

        if stats.cover_checks >= budget.cover_checks {
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
    budget: SolvingBudget,
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
                indexed_cover_covers_cube(&reference, None, &generalized, &mut memo, budget, stats)
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
        if stats.cover_checks >= budget.cover_checks {
            expanded.extend(reference.iter().skip(cube_index + 1).cloned());
            break;
        }
    }

    absorb_indexed(expanded)
}

fn heuristic_irredundant(
    cover: Vec<IndexedCube>,
    budget: SolvingBudget,
    stats: &mut HeuristicStats,
) -> Vec<IndexedCube> {
    let cover = absorb_indexed(cover);
    let mut irredundant = Vec::with_capacity(cover.len());

    for (cube_index, cube) in cover.iter().enumerate() {
        let mut memo = HashMap::new();

        let Some(is_covered) =
            indexed_cover_covers_cube(&cover, Some(cube_index), cube, &mut memo, budget, stats)
        else {
            irredundant.extend(cover.iter().skip(cube_index).cloned());
            return absorb_indexed(irredundant);
        };

        if !is_covered {
            irredundant.push(cube.clone());
        }

        if stats.cover_checks >= budget.cover_checks {
            irredundant.extend(cover.iter().skip(cube_index + 1).cloned());
            break;
        }
    }

    absorb_indexed(irredundant)
}

fn indexed_cover_covers_cube(
    cover: &[IndexedCube],
    excluded_index: Option<usize>,
    cube: &IndexedCube,
    memo: &mut HashMap<IndexedCube, bool>,
    budget: SolvingBudget,
    stats: &mut HeuristicStats,
) -> Option<bool> {
    if let Some(result) = memo.get(cube) {
        return Some(*result);
    }
    if !stats.try_take_cover_check(budget) {
        return None;
    }

    let included = cover
        .iter()
        .enumerate()
        .filter(|(index, _)| Some(*index) != excluded_index)
        .map(|(_, existing)| existing);
    let result = if included.clone().any(|existing| existing.subsumes(cube)) {
        true
    } else {
        let split_index = included
            .filter(|existing| !existing.conflicts_with(cube))
            .flat_map(IndexedCube::specified_indices)
            .find(|index| cube.literal(*index) == LiteralState::DontCare);
        let Some(split_index) = split_index else {
            return Some(false);
        };

        let positive = indexed_cover_covers_cube(
            cover,
            excluded_index,
            &cube.with_literal(split_index, LiteralState::Positive),
            memo,
            budget,
            stats,
        )?;
        let negative = indexed_cover_covers_cube(
            cover,
            excluded_index,
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
    const fn try_take_cover_check(&mut self, budget: SolvingBudget) -> bool {
        if self.cover_checks >= budget.cover_checks {
            false
        } else {
            self.cover_checks += 1;
            true
        }
    }
}
