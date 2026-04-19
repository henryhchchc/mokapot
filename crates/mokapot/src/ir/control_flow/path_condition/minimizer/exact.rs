use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;
use std::iter::once;

use itertools::Itertools;

use super::{
    absorb,
    indexed::{AtomTable, IndexedCube, absorb_indexed, cover_cost},
};
use crate::ir::control_flow::path_condition::cube::Cube;

pub(super) fn exact_minimize<P>(cubes: &HashSet<Cube<P>>, atoms: &AtomTable<P>) -> HashSet<Cube<P>>
where
    P: Hash + Eq + Clone,
{
    let mut on_set = cubes
        .iter()
        .flat_map(|cube| IndexedCube::from_cube(cube, atoms).expand_minterms())
        .collect::<Vec<_>>();
    on_set.sort_by_cached_key(IndexedCube::sort_key);
    on_set.dedup();
    let prime_implicants = prime_implicants(on_set.clone());
    let selected = minimum_cover(&prime_implicants, &on_set);

    absorb(
        selected
            .into_iter()
            .map(|index| prime_implicants[index].to_cube(atoms))
            .collect(),
    )
}

fn prime_implicants(on_set: Vec<IndexedCube>) -> Vec<IndexedCube> {
    let mut current = absorb_indexed(on_set);
    let mut prime_implicants = Vec::new();

    while !current.is_empty() {
        let mut combined = vec![false; current.len()];
        let mut next = Vec::new();

        for i in 0..current.len() {
            for j in i + 1..current.len() {
                if let Some(cube) = current[i].combine(&current[j]) {
                    combined[i] = true;
                    combined[j] = true;
                    next.push(cube);
                }
            }
        }

        for (used, cube) in combined.into_iter().zip(current) {
            if !used {
                prime_implicants.push(cube);
            }
        }

        current = absorb_indexed(next);
    }

    prime_implicants.sort_by_cached_key(IndexedCube::sort_key);
    prime_implicants.dedup();
    prime_implicants
}

fn minimum_cover(prime_implicants: &[IndexedCube], on_set: &[IndexedCube]) -> BTreeSet<usize> {
    let coverage = on_set
        .iter()
        .map(|cube| {
            prime_implicants
                .iter()
                .enumerate()
                .filter_map(|(index, implicant)| implicant.subsumes(cube).then_some(index))
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();

    let mut selected: BTreeSet<usize> = coverage
        .iter()
        .filter(|it| it.len() == 1)
        .flatten()
        .copied()
        .collect();

    let mut uncovered: BTreeSet<_> = (0..on_set.len()).collect();
    if !selected.is_empty() {
        uncovered.retain(|&cube_index| {
            !selected
                .iter()
                .any(|prime_index| prime_implicants[*prime_index].subsumes(&on_set[cube_index]))
        });
    }

    if uncovered.is_empty() {
        return selected;
    }

    let products = uncovered
        .into_iter()
        .fold(vec![BTreeSet::new()], |products, cube_index| {
            let options: Vec<_> = coverage[cube_index]
                .iter()
                .filter(|index| !selected.contains(index))
                .copied()
                .collect();
            let next_products = products
                .iter()
                .cartesian_product(&options)
                .map(|(product, option)| product.iter().copied().chain(once(*option)).collect())
                .collect();
            reduce_products(next_products, prime_implicants)
        });

    let best = products
        .into_iter()
        .min_by_key(|product| cover_cost(product, prime_implicants))
        .unwrap_or_default();
    selected.extend(best);
    selected
}

fn reduce_products(
    products: Vec<BTreeSet<usize>>,
    prime_implicants: &[IndexedCube],
) -> Vec<BTreeSet<usize>> {
    products
        .into_iter()
        .sorted_by_key(|product| cover_cost(product, prime_implicants))
        .fold(Vec::new(), |mut reduced, product| {
            if reduced.iter().any(|existing| existing.is_subset(&product)) {
                reduced
            } else {
                reduced.retain(|existing: &BTreeSet<usize>| !product.is_subset(existing));
                reduced.push(product);
                reduced
            }
        })
}
