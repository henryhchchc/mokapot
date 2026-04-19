use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;

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
    let mut selected = BTreeSet::new();
    let mut uncovered = (0..on_set.len()).collect::<BTreeSet<_>>();
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

    for covers in &coverage {
        if covers.len() == 1 {
            selected.extend(covers.iter().copied());
        }
    }

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

    let mut products = vec![BTreeSet::new()];
    for cube_index in uncovered {
        let options = coverage[cube_index]
            .iter()
            .filter(|index| !selected.contains(index))
            .copied()
            .collect::<Vec<_>>();

        let mut next_products = Vec::new();
        for product in &products {
            for option in &options {
                let mut candidate = product.clone();
                candidate.insert(*option);
                next_products.push(candidate);
            }
        }

        products = reduce_products(next_products, prime_implicants);
    }

    let best = products
        .into_iter()
        .min_by_key(|product| cover_cost(product, prime_implicants))
        .unwrap_or_default();
    selected.extend(best);
    selected
}

fn reduce_products(
    mut products: Vec<BTreeSet<usize>>,
    prime_implicants: &[IndexedCube],
) -> Vec<BTreeSet<usize>> {
    products.sort_by_key(|product| cover_cost(product, prime_implicants));
    let mut reduced: Vec<BTreeSet<usize>> = Vec::new();

    'candidate: for product in products {
        for existing in &reduced {
            if existing.is_subset(&product) {
                continue 'candidate;
            }
        }

        reduced.retain(|existing: &BTreeSet<usize>| !product.is_subset(existing));
        reduced.push(product);
    }

    reduced
}
