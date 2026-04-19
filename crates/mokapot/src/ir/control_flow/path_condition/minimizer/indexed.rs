use std::{
    collections::hash_map::DefaultHasher,
    collections::{BTreeSet, HashMap, HashSet},
    hash::{Hash, Hasher},
};

use crate::ir::control_flow::path_condition::{BooleanVariable, cube::Cube};

#[derive(Debug, Clone)]
pub(super) struct AtomTable<P> {
    atoms: Vec<P>,
    indices: HashMap<P, usize>,
}

impl<P> AtomTable<P>
where
    P: Hash + Eq + Clone,
{
    pub(super) fn from_cubes(cubes: &HashSet<Cube<P>>) -> Self {
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

    pub(super) const fn len(&self) -> usize {
        self.atoms.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum LiteralState {
    DontCare,
    Positive,
    Negative,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct IndexedCube {
    literals: Vec<LiteralState>,
}

impl IndexedCube {
    pub(super) fn from_cube<P>(cube: &Cube<P>, atoms: &AtomTable<P>) -> Self
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

    pub(super) fn to_cube<P>(&self, atoms: &AtomTable<P>) -> Cube<P>
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
                    debug_assert_ne!(
                        inserted,
                        crate::ir::control_flow::path_condition::cube::InsertResult::Contradiction
                    );
                }
                LiteralState::Negative => {
                    let inserted = cube.insert(BooleanVariable::Negative(predicate));
                    debug_assert_ne!(
                        inserted,
                        crate::ir::control_flow::path_condition::cube::InsertResult::Contradiction
                    );
                }
            }
        }
        cube
    }

    pub(super) fn combine(&self, other: &Self) -> Option<Self> {
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

    pub(super) fn subsumes(&self, other: &Self) -> bool {
        self.literals
            .iter()
            .zip(&other.literals)
            .all(|(lhs, rhs)| *lhs == LiteralState::DontCare || lhs == rhs)
    }

    pub(super) fn conflicts_with(&self, other: &Self) -> bool {
        self.literals.iter().zip(&other.literals).any(|(lhs, rhs)| {
            matches!(
                (lhs, rhs),
                (LiteralState::Positive, LiteralState::Negative)
                    | (LiteralState::Negative, LiteralState::Positive)
            )
        })
    }

    pub(super) fn literal(&self, index: usize) -> LiteralState {
        self.literals[index]
    }

    pub(super) fn literal_count(&self) -> usize {
        self.literals
            .iter()
            .filter(|literal| **literal != LiteralState::DontCare)
            .count()
    }

    pub(super) fn sort_key(&self) -> (usize, Vec<LiteralState>) {
        (self.literal_count(), self.literals.clone())
    }

    pub(super) fn specified_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.literals
            .iter()
            .enumerate()
            .filter_map(|(index, literal)| (*literal != LiteralState::DontCare).then_some(index))
    }

    pub(super) fn generalize(&self, index: usize) -> Self {
        self.with_literal(index, LiteralState::DontCare)
    }

    pub(super) fn with_literal(&self, index: usize, literal: LiteralState) -> Self {
        let mut literals = self.literals.clone();
        literals[index] = literal;
        Self { literals }
    }

    pub(super) fn expand_minterms(self) -> Vec<Self> {
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

pub(super) fn absorb_indexed(cubes: Vec<IndexedCube>) -> Vec<IndexedCube> {
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

pub(super) fn cover_cost(
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

pub(super) fn indexed_cover_cost(cubes: &[IndexedCube]) -> (usize, usize, Vec<Vec<LiteralState>>) {
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
