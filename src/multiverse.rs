use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::convert::TryInto;

use defn::Color;
use misc;
use misc::Coords;

/// A Layout is a subset of a Multiverse. It defines a set of unique solutions
/// for the cells covered by the Multiverse.
/// It is a mapping from a set of coords to the number of blues amongst these
/// cells.
/// Examples:
/// {a}: 0     // The `a` cell is black
/// {b}: 1     // The `b` cell is blue
/// {c, d}: 1  // One of `c, d` is blue and the other is black
/// {e, f}: 2  // Both `e, f` are blue
/// n: k       // `k` of the `n` coordinates are blue.
///               (i.e. n.len() choose k combinations)
#[derive(Debug, Clone)]
pub struct Layout {
    pub binomial_coefs: BTreeMap<BTreeSet<Coords>, u16>,
}

impl Layout {
    pub fn new(binomial_coefs: BTreeMap<BTreeSet<Coords>, u16>) -> Layout {
        let mut seen = BTreeSet::new();
        for (coords_set, blue_count) in &binomial_coefs {
            assert_ne!(coords_set.len(), 0, "empty coords_set in input layout");
            assert!((*blue_count) as usize <= coords_set.len());
            for coords in coords_set {
                assert!(!seen.contains(coords), "duplicate coords in input layout");
                seen.insert(coords);
            }
        }
        Layout { binomial_coefs }
    }

    pub fn solution_count(&self) -> Option<u64> {
        let mut i: u64 = 1;
        for (coords_set, blue_count) in &self.binomial_coefs {
            let fact = misc::n_choose_k(coords_set.len().try_into().unwrap(), *blue_count as u64);
            match fact.and_then(|fact| i.checked_mul(fact)) {
                None => return None,
                Some(res) => i = res,
            }
        }
        Some(i)
    }

    /// Test if two Layouts share the same keys on their intersection
    fn aligned_with(&self, other: &Layout) -> bool {
        let mut left_key_per_coords = BTreeMap::new();
        for kleft in self.binomial_coefs.keys() {
            for c in kleft {
                let _ = left_key_per_coords.insert(c, kleft);
            }
        }
        for kright in other.binomial_coefs.keys() {
            for c in kright {
                match left_key_per_coords.get(c) {
                    None => (),
                    Some(kleft) => {
                        if kleft != &kright {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    fn are_aligned(left: &[Layout], right: &[Layout]) -> bool {
        // First: Check that all left Layouts have the same
        let left_layout_opt = match left {
            [] => None,
            [hd, tl @ ..] => {
                let keys: BTreeSet<_> = hd.binomial_coefs.keys().cloned().collect();
                for map in tl {
                    if keys != map.binomial_coefs.keys().cloned().collect() {
                        return false;
                    }
                }
                Some(hd)
            }
        };
        // Then: Check that all right Layouts have the same keys
        let right_layout_opt = match right {
            [] => None,
            [hd, tl @ ..] => {
                let keys: BTreeSet<_> = hd.binomial_coefs.keys().cloned().collect();
                for map in tl {
                    if keys != map.binomial_coefs.keys().cloned().collect() {
                        return false;
                    }
                }
                Some(hd)
            }
        };
        // Finally: Check that left/right are aligned
        match (left_layout_opt, right_layout_opt) {
            (Some(left_layout), Some(right_layout)) => left_layout.aligned_with(right_layout),
            _ => true,
        }
    }

    /// Fork all the same-keyed Layouts in the input Vec<Layout> so that they contain new_key.
    fn split(layouts: &Vec<Layout>, new_key: &BTreeSet<Coords>) -> Vec<Layout> {
        let mut res = vec![];
        for lay in layouts {
            let old_key = lay
                .binomial_coefs
                .keys()
                .find(|coords_set| coords_set.is_superset(new_key))
                .expect("Unexpected parameters to split");
            if new_key == old_key {
                // This means that a previous call to `split` already chunked as wished
                res.push(lay.clone());
                continue;
            }
            let new_key2: BTreeSet<_> = old_key.difference(new_key).cloned().collect();
            assert!(!new_key2.is_empty());
            let mut bc = lay.binomial_coefs.clone();
            let blue_count = bc.remove(old_key).expect("Unreachable");
            let mut pushed = 0;
            for i in 0..=blue_count {
                let j = blue_count - i;
                if i as usize <= new_key.len() && j as usize <= new_key2.len() {
                    let mut bc = bc.clone();
                    bc.insert(new_key.clone(), i);
                    bc.insert(new_key2.clone(), j);
                    res.push(Layout::new(bc));
                    pushed += 1;
                }
            }
            assert!(pushed != 0);
        }
        res
    }

    /// Fork a layout to make it compatible with the keys of another Layout. That other Layout will
    /// need to undergo the symmetrical operation.
    fn align_with_keys(&self, right_keys: &BTreeSet<BTreeSet<Coords>>) -> Vec<Layout> {
        let mut res = vec![(*self).clone()];
        for left_key in self.binomial_coefs.keys() {
            for right_key in right_keys {
                if left_key.is_disjoint(right_key) {
                    continue;
                }
                let inter = left_key.intersection(right_key).cloned().collect();
                if left_key == &inter {
                    continue;
                }
                res = Self::split(&res, &inter);
            }
        }
        res
    }

    /// Reshape two layouts to give them the same keys on their intersection.
    /// Such a reshaping implies forking each layout into multiple layouts, hence the `Vec` return type.
    /// In `(va, vb) = align(a, b)`:
    /// - `a` and `va` encode the exact same set of solutions (the same goes for `b` with `vb`).
    /// - If `a` and `b` are already aligned, `va = vec![a]` and `vb = vec![vb]`.
    /// - All the Layouts in `va` have the same keys (the same goes for `vb`).
    /// - The number of solutions is identical in `a` and `va` (the same foes for `b` and `vb`).
    fn align(&self, other: &Layout) -> (Vec<Layout>, Vec<Layout>) {
        let left_keys: BTreeSet<_> = self.binomial_coefs.keys().cloned().collect();
        let right_keys: BTreeSet<_> = other.binomial_coefs.keys().cloned().collect();
        let left = self.align_with_keys(&right_keys);
        let right = other.align_with_keys(&left_keys);
        assert!(Self::are_aligned(&left, &right));

        // The following assert crashes because of https://www.reddit.com/r/hexcellslevels/comments/pnhjef/level_divided_easy/
        // assert_eq!(
        //     self.solution_count(),
        //     left.iter()
        //         .map(|lay| lay.solution_count())
        //         .fold(Some(0), |acc, b| acc
        //             .and_then(|a: u64| b.and_then(|b: u64| a.checked_add(b))))
        // );
        // assert_eq!(
        //     other.solution_count(),
        //     right
        //         .iter()
        //         .map(|lay| lay.solution_count())
        //         .fold(Some(0), |acc, b| acc
        //             .and_then(|a: u64| b.and_then(|b: u64| a.checked_add(b))))
        // );

        (left, right)
    }

    fn merge(&self, other: &Layout) -> Vec<Layout> {
        let mut res = vec![];
        let (left_lays, right_lays) = self.align(other);
        let left_keys: BTreeSet<_> = left_lays
            .get(0)
            .expect("Left can't be empty here")
            .binomial_coefs
            .keys()
            .collect();
        let right_keys: BTreeSet<_> = right_lays
            .get(0)
            .expect("Right can't be empty here")
            .binomial_coefs
            .keys()
            .collect();
        let inter_keys: Vec<_> = left_keys.intersection(&right_keys).collect();
        for left_lay in &left_lays {
            for right_lay in &right_lays {
                if inter_keys
                    .iter()
                    .all(|key| left_lay.binomial_coefs[key] == right_lay.binomial_coefs[key])
                {
                    let mut bc = left_lay.binomial_coefs.clone();
                    for (k, v) in &right_lay.binomial_coefs {
                        bc.insert(k.clone(), *v);
                    }
                    res.push(Layout::new(bc))
                }
            }
        }
        res
    }
}

#[derive(PartialEq, Debug)]
pub enum State {
    Running,
    Stuck,
    Empty,
}

/// A Multiverse gathers all the possible permutations that a given set of coords (i.e. scope) may take.
/// If `mv.solution_count_upper_bound() == 1`, there is no uncertainty within `mv`.
/// If `mv.invariants().is_empty()`, there is no certainty within `mv`.
/// Two differents layout in a multiverse are two ways to describe permutations of the same set of coords (i.e. the scope).
/// Two layouts in a multiverse may describe overlapping sets of results, hence the fact that [solution_count_upper_bound] doesn't give the exact number of solutions.
/// A multiverse may have no solutions (i.e. `State::Stuck`)
#[derive(Debug, Clone)]
pub struct Multiverse {
    pub scope: BTreeSet<Coords>,
    pub layouts: Vec<Layout>,
}

impl Multiverse {
    pub fn new(scope: BTreeSet<Coords>, layouts: Vec<Layout>) -> Multiverse {
        for lay in &layouts {
            let lay_coords = lay.binomial_coefs.keys().fold(BTreeSet::new(), |acc, set| {
                acc.union(set).cloned().collect()
            });
            assert_eq!(lay_coords, scope);
        }
        Multiverse { scope, layouts }
    }

    pub fn empty() -> Multiverse {
        Multiverse::new(BTreeSet::new(), vec![])
    }

    pub fn solution_count_upper_bound(&self) -> Option<u64> {
        let mut i: u64 = 0;
        for lay in &self.layouts {
            match lay.solution_count().and_then(|x| i.checked_add(x)) {
                None => return None,
                Some(res) => {
                    i = res;
                }
            }
        }
        Some(i)
    }

    pub fn state(&self) -> State {
        match (self.scope.is_empty(), self.layouts.is_empty()) {
            (true, true) => State::Empty,
            (false, false) => State::Running,
            (false, true) => State::Stuck,
            (true, false) => panic!("Corrupted multiverse"),
        }
    }

    /// The invariants of the Multiverse are the coords that have a constant
    /// color across all the solutions of the Multiverse.
    /// The result is undefined if the multiverse is stuck (i.e. empty layouts)
    pub fn invariants(&self) -> BTreeMap<Coords, Color> {
        let mut blue_for_sure = self.scope.clone();
        let mut black_for_sure = self.scope.clone();
        // Start with full `blue_for_sure` and `black_for_sure` and gradually purge them.
        // If both become empty. All cells in the scope are uncertain.
        for lay in &self.layouts {
            for (coords_set, blue_count) in &lay.binomial_coefs {
                if *blue_count == 0 {
                    // All in `coords_set` are black
                    for coords in coords_set {
                        blue_for_sure.remove(coords);
                    }
                } else if *blue_count as usize == coords_set.len() {
                    // All in `coords_set` are blue
                    for coords in coords_set {
                        black_for_sure.remove(coords);
                    }
                } else {
                    // All in `coords_set` are unknown
                    for coords in coords_set {
                        blue_for_sure.remove(coords);
                        black_for_sure.remove(coords);
                    }
                }
                if blue_for_sure.is_empty() && black_for_sure.is_empty() {
                    // Early stop
                    break;
                }
            }
            if blue_for_sure.is_empty() && black_for_sure.is_empty() {
                // Early stop
                break;
            }
        }
        let mut result: BTreeMap<_, _> = BTreeMap::new();
        for coords in blue_for_sure {
            result.insert(coords, Color::Blue);
        }
        for coords in black_for_sure {
            result.insert(coords, Color::Black);
        }
        result
    }

    pub fn merge(&self, other: &Multiverse) -> Multiverse {
        let scope = self.scope.union(&other.scope).cloned().collect();
        match (self.state(), other.state()) {
            (State::Empty, _) => return other.clone(),
            (_, State::Empty) => return self.clone(),
            (State::Stuck, _) | (_, State::Stuck) => return Multiverse::new(scope, vec![]),
            (State::Running, State::Running) => (),
        }
        let mut layouts = vec![];
        for left_lay in &self.layouts {
            for right_lay in &other.layouts {
                layouts.append(&mut left_lay.merge(right_lay));
            }
        }
        Multiverse::new(scope, layouts)
    }

    pub fn learn(&self, coords: &Coords, color: Color) -> Multiverse {
        let mut scope = self.scope.clone();
        let key = BTreeSet::from([*coords]);
        if scope == key {
            return Multiverse::empty();
        }
        assert!(scope.remove(coords));
        let layouts = Layout::split(&self.layouts, &key);
        let layouts = layouts
            .iter()
            .filter_map(|lay| {
                match (color, lay.binomial_coefs[&key.clone()]) {
                    (_, 2..=u16::MAX) => panic!("Unreachable"),
                    (Color::Blue, 1) | (Color::Black, 0) => {
                        // Keep that layout and remove that coords
                        let mut bc = lay.binomial_coefs.clone();
                        bc.remove(&key);
                        Some(Layout::new(bc))
                    }
                    (Color::Blue, 0) | (Color::Black, 1) => {
                        // The layout assumed the other color than the one currently
                        // learned
                        None
                    }
                }
            })
            .collect();
        Multiverse::new(scope, layouts)
    }
}
