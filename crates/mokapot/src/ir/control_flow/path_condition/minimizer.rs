use std::{
    collections::hash_map::DefaultHasher,
    collections::{BTreeSet, HashMap, HashSet},
    hash::{Hash, Hasher},
};

use super::{BooleanVariable, PathConditionBudget, cube::Cube};

/// A reduction strategy for boolean covers.
pub(super) trait Minimizer<P> {
    /// Returns an equivalent set of cubes with redundant terms removed.
    fn minimize(&self, cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
    where
        P: Hash + Eq + Clone;
}

/// A bounded boolean minimizer for path-condition covers.
#[derive(Debug, Clone, Copy)]
pub(super) struct BoundedMinimizer {
    budget: PathConditionBudget,
}

impl BoundedMinimizer {
    /// Creates a new minimizer with the given resource budget.
    #[must_use]
    pub const fn new(budget: PathConditionBudget) -> Self {
        Self { budget }
    }
}

impl<P> Minimizer<P> for BoundedMinimizer {
    fn minimize(&self, cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
    where
        P: Hash + Eq + Clone,
    {
        let cubes = absorb(cubes);
        if cubes.len() <= 1 {
            return cubes;
        }

        let atoms = AtomTable::from_cubes(&cubes);
        let on_set_upper_bound = exact_on_set_upper_bound(&cubes, atoms.atoms.len());

        match on_set_upper_bound {
            Some(upper_bound) if upper_bound <= self.budget.max_exact_on_set_size => {
                exact_minimize(&cubes, &atoms)
            }
            _ => heuristic_minimize(&cubes, &atoms, self.budget),
        }
    }
}

fn exact_minimize<P>(cubes: &HashSet<Cube<P>>, atoms: &AtomTable<P>) -> HashSet<Cube<P>>
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

fn heuristic_minimize<P>(
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
            .find(|index| cube.literals[*index] == LiteralState::DontCare);
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

fn exact_on_set_upper_bound<P>(cubes: &HashSet<Cube<P>>, atom_count: usize) -> Option<usize>
where
    P: Hash + Eq,
{
    cubes.iter().try_fold(0usize, |upper_bound, cube| {
        let dont_care_count = atom_count.saturating_sub(cube.predicates().count());
        if dont_care_count >= usize::BITS as usize {
            return None;
        }

        let cube_expansion = 1usize << dont_care_count;
        upper_bound.checked_add(cube_expansion)
    })
}

fn absorb<P>(cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
where
    P: Hash + Eq,
{
    let mut reduced = HashSet::new();
    for cube in cubes {
        if reduced
            .iter()
            .any(|existing: &Cube<P>| existing.subsumes(&cube))
        {
            continue;
        }

        reduced.retain(|existing| !cube.subsumes(existing));
        reduced.insert(cube);
    }
    reduced
}

#[derive(Debug, Clone)]
struct AtomTable<P> {
    atoms: Vec<P>,
    indices: HashMap<P, usize>,
}

impl<P> AtomTable<P>
where
    P: Hash + Eq + Clone,
{
    fn from_cubes(cubes: &HashSet<Cube<P>>) -> Self {
        let mut atom_set = HashSet::new();
        for cube in cubes {
            atom_set.extend(cube.predicates().cloned());
        }

        let mut atoms = atom_set.into_iter().collect::<Vec<_>>();
        atoms.sort_by_cached_key(predicate_key::<P>);
        let indices = atoms
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, predicate)| (predicate, index))
            .collect();
        Self { atoms, indices }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum LiteralState {
    DontCare,
    Positive,
    Negative,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IndexedCube {
    literals: Vec<LiteralState>,
}

impl IndexedCube {
    fn from_cube<P>(cube: &Cube<P>, atoms: &AtomTable<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let mut literals = vec![LiteralState::DontCare; atoms.atoms.len()];
        for predicate in cube.positive() {
            literals[*atoms.indices.get(predicate).expect("missing positive atom")] =
                LiteralState::Positive;
        }
        for predicate in cube.negative() {
            literals[*atoms.indices.get(predicate).expect("missing negative atom")] =
                LiteralState::Negative;
        }
        Self { literals }
    }

    fn to_cube<P>(&self, atoms: &AtomTable<P>) -> Cube<P>
    where
        P: Hash + Eq + Clone,
    {
        let mut cube = Cube::one();
        for (index, literal) in self.literals.iter().enumerate() {
            let predicate = atoms.atoms[index].clone();
            match literal {
                LiteralState::DontCare => {}
                LiteralState::Positive => {
                    let inserted = cube.insert(BooleanVariable::Positive(predicate));
                    debug_assert_ne!(inserted, super::cube::InsertResult::Contradiction);
                }
                LiteralState::Negative => {
                    let inserted = cube.insert(BooleanVariable::Negative(predicate));
                    debug_assert_ne!(inserted, super::cube::InsertResult::Contradiction);
                }
            }
        }
        cube
    }

    fn combine(&self, other: &Self) -> Option<Self> {
        let mut difference = None;
        let mut literals = Vec::with_capacity(self.literals.len());

        for (index, (lhs, rhs)) in self.literals.iter().zip(&other.literals).enumerate() {
            match (lhs, rhs) {
                (LiteralState::Positive, LiteralState::Negative)
                | (LiteralState::Negative, LiteralState::Positive) => {
                    if difference.is_some() {
                        return None;
                    }
                    difference = Some(index);
                    literals.push(LiteralState::DontCare);
                }
                (lhs, rhs) if lhs == rhs => literals.push(*lhs),
                _ => return None,
            }
        }

        difference.map(|_| Self { literals })
    }

    fn subsumes(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(lhs, rhs)| *lhs == LiteralState::DontCare || lhs == rhs)
    }

    fn conflicts_with(&self, other: &Self) -> bool {
        self.literals.iter().zip(&other.literals).any(|(lhs, rhs)| {
            matches!(
                (lhs, rhs),
                (LiteralState::Positive, LiteralState::Negative)
                    | (LiteralState::Negative, LiteralState::Positive)
            )
        })
    }

    fn literal_count(&self) -> usize {
        self.literals
            .iter()
            .filter(|literal| **literal != LiteralState::DontCare)
            .count()
    }

    fn sort_key(&self) -> (usize, Vec<LiteralState>) {
        (self.literal_count(), self.literals.clone())
    }

    fn specified_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.literals
            .iter()
            .enumerate()
            .filter_map(|(index, literal)| (*literal != LiteralState::DontCare).then_some(index))
    }

    fn generalize(&self, index: usize) -> Self {
        self.with_literal(index, LiteralState::DontCare)
    }

    fn with_literal(&self, index: usize, literal: LiteralState) -> Self {
        let mut literals = self.literals.clone();
        literals[index] = literal;
        Self { literals }
    }

    fn expand_minterms(self) -> Vec<Self> {
        self.literals.into_iter().fold(
            vec![Self {
                literals: Vec::new(),
            }],
            |minterms, literal| match literal {
                LiteralState::DontCare => minterms
                    .into_iter()
                    .flat_map(|minterm| {
                        let mut positive = minterm.clone();
                        positive.literals.push(LiteralState::Positive);

                        let mut negative = minterm;
                        negative.literals.push(LiteralState::Negative);

                        [positive, negative]
                    })
                    .collect(),
                specified => minterms
                    .into_iter()
                    .map(|mut minterm| {
                        minterm.literals.push(specified);
                        minterm
                    })
                    .collect(),
            },
        )
    }
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

fn absorb_indexed(cubes: Vec<IndexedCube>) -> Vec<IndexedCube> {
    let mut reduced = Vec::new();

    'candidate: for cube in cubes {
        if reduced
            .iter()
            .any(|existing: &IndexedCube| existing.subsumes(&cube))
        {
            continue;
        }

        reduced.retain(|existing| !cube.subsumes(existing));
        for existing in &reduced {
            if existing == &cube {
                continue 'candidate;
            }
        }
        reduced.push(cube);
    }

    reduced.sort_by_cached_key(IndexedCube::sort_key);
    reduced.dedup();
    reduced
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

fn cover_cost(
    product: &BTreeSet<usize>,
    prime_implicants: &[IndexedCube],
) -> (usize, usize, Vec<usize>) {
    (
        product
            .iter()
            .map(|index| prime_implicants[*index].literal_count())
            .sum(),
        product.len(),
        product.iter().copied().collect(),
    )
}

fn indexed_cover_cost(cubes: &[IndexedCube]) -> (usize, usize, Vec<Vec<LiteralState>>) {
    (
        cubes.len(),
        cubes.iter().map(IndexedCube::literal_count).sum(),
        cubes.iter().map(|cube| cube.literals.clone()).collect(),
    )
}

fn predicate_key<P>(predicate: &P) -> (u64, u64)
where
    P: Hash,
{
    let mut default = DefaultHasher::new();
    predicate.hash(&mut default);

    let mut fnv = FnvHasher::default();
    predicate.hash(&mut fnv);

    (default.finish(), fnv.finish())
}

#[derive(Debug, Clone)]
struct FnvHasher(u64);

impl Default for FnvHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_minimizer_reduces_complementary_terms_exactly() {
        let cubes = HashSet::from([
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Positive(2_u8))
                .unwrap(),
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Negative(2_u8))
                .unwrap(),
        ]);

        let reduced = BoundedMinimizer::new(PathConditionBudget::default()).minimize(cubes);
        assert_eq!(
            reduced,
            HashSet::from([Cube::of(BooleanVariable::Positive(1_u8))])
        );
    }

    #[test]
    fn bounded_minimizer_reduces_complementary_terms_heuristically() {
        let cubes = HashSet::from([
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Positive(2_u8))
                .unwrap(),
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Negative(2_u8))
                .unwrap(),
        ]);

        let reduced = BoundedMinimizer::new(PathConditionBudget {
            max_exact_on_set_size: 0,
            max_heuristic_rounds: 2,
            max_cover_checks: 128,
        })
        .minimize(cubes);
        assert_eq!(
            reduced,
            HashSet::from([Cube::of(BooleanVariable::Positive(1_u8))])
        );
    }

    #[test]
    fn exact_on_set_upper_bound_detects_large_expansions() {
        let cubes = (0_u8..10)
            .map(|predicate| Cube::of(BooleanVariable::Positive(predicate)))
            .collect::<HashSet<_>>();

        assert_eq!(exact_on_set_upper_bound(&cubes, 10), Some(10 * (1 << 9)));
    }
}
