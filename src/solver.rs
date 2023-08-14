use itertools::Itertools;
use multiverse::Multiverse;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::convert::TryInto;
use std::error::Error;
use std::fmt;

use constraint;
use defn;
use defn::Cell;
use defn::Color;
use defn::Defn;
use env;
use env::Env;
use misc::Coords;
use multiverse::State;

/// Solver progress. Finished when `unknowns` is empty.
struct Progress {
    blues: BTreeSet<Coords>,
    blacks: BTreeSet<Coords>,
    unknowns: BTreeSet<Coords>,
}

#[derive(Debug, Serialize, Deserialize)]
enum Difficulty {
    Global(u32),
    Local(u32),
}

impl Progress {
    fn of_defn(defn: &Defn) -> Progress {
        let mut blues = BTreeSet::new();
        let mut blacks = BTreeSet::new();
        let mut unknowns = BTreeSet::new();
        let mut add = |coords: Coords, revealed: bool, color: Color| {
            let _: bool = match (revealed, color) {
                (false, _) => unknowns.insert(coords),
                (true, Color::Black) => blacks.insert(coords),
                (true, Color::Blue) => blues.insert(coords),
            };
        };
        for (coords, cell) in defn.iter() {
            type C = defn::Cell;
            match cell {
                C::Empty => (),
                C::Line { .. } => (),
                C::Zone0 { revealed, color } => add(*coords, *revealed, *color),
                C::Zone6 { revealed, .. } => add(*coords, *revealed, Color::Black),
                C::Zone18 { revealed, .. } => add(*coords, *revealed, Color::Blue),
            }
        }
        Progress {
            blues,
            blacks,
            unknowns,
        }
    }

    fn is_solved(&self) -> bool {
        self.unknowns.is_empty()
    }

    fn update(&mut self, findings: BTreeMap<Coords, Color>) {
        for (coords, color) in findings {
            self.unknowns.remove(&coords);
            match color {
                Color::Black => {
                    self.blacks.insert(coords);
                }
                Color::Blue => {
                    self.blues.insert(coords);
                }
            }
        }
    }
}

/// Solver constraints. They correspond to the numbers in the actual game.
/// The hidden ones are yet to be revealed by the solver loop.
/// The exhausted ones are revealed but don't carry uncertainty anymore.
/// The visible ones is the active set of constraint for the solver. The the actual puzzle, there
/// are the constraints that the player has to look at in order to discover new cells.
struct Constraints {
    constraints_hidden: BTreeMap<Coords, Multiverse>,
    constraints_visible: BTreeMap<Coords, Multiverse>,
    constraints_exhausted: BTreeSet<Coords>,
}

/// This is used to give a virtual coordinate to the global constraint
static UNIQUE_COORDS: Lazy<Coords> = Lazy::new(|| Coords::new(999, 0, -999));

impl Constraints {
    fn of_defn(defn: &Defn) -> Constraints {
        let mut constraints_hidden = BTreeMap::new();
        let mut constraints_visible = BTreeMap::new();
        let constraints_exhausted = BTreeSet::new();
        for (coords, cell) in defn {
            match cell {
                Cell::Empty => (),
                Cell::Zone0 { .. } => (),
                Cell::Line { m, o } => {
                    constraints_visible.insert(*coords, constraint::line(defn, *coords, *o, *m));
                }
                Cell::Zone6 { m, .. } => {
                    constraints_hidden.insert(*coords, constraint::zone6(defn, *coords, *m));
                }
                Cell::Zone18 { .. } => {
                    constraints_hidden.insert(*coords, constraint::zone18(defn, *coords));
                }
            }
        }
        constraints_visible.insert(*UNIQUE_COORDS, constraint::global_blue_count(defn));
        Constraints {
            constraints_hidden,
            constraints_visible,
            constraints_exhausted,
        }
    }

    fn reveal(&mut self, visible_cells: &BTreeSet<Coords>) {
        for k in self.constraints_hidden.keys().cloned().collect::<Vec<_>>() {
            if visible_cells.contains(&k) {
                let mv = self.constraints_hidden.remove(&k).expect("Unreachable");
                self.constraints_visible.insert(k, mv);
            }
        }
    }

    fn narrow(&mut self, visible_cells: &BTreeSet<Coords>, progress: &Progress) {
        for (_k, mv) in self.constraints_visible.iter_mut() {
            let inter: BTreeSet<_> = mv.scope.intersection(visible_cells).cloned().collect();
            if inter.is_empty() {
                continue;
            }
            for coords in inter.intersection(&progress.blues) {
                *mv = mv.learn(coords, Color::Blue);
            }
            for coords in inter.intersection(&progress.blacks) {
                *mv = mv.learn(coords, Color::Black);
            }
        }
    }

    fn gc(&mut self) {
        for k in self.constraints_visible.keys().cloned().collect::<Vec<_>>() {
            match self.constraints_visible[&k].state() {
                State::Running => (),
                State::Stuck => panic!("The grid is bugged and has no soltions"),
                State::Empty => {
                    self.constraints_visible
                        .remove(&k.clone())
                        .expect("Unreachable");
                    self.constraints_exhausted.insert(k);
                }
            }
        }
    }

    fn is_solved(&self) -> bool {
        self.constraints_visible.is_empty() && self.constraints_hidden.is_empty()
    }

    fn trivial_invariants(&self, defn: &Defn) -> BTreeMap<Coords, Color> {
        let mut invariants = BTreeMap::new();
        for mv in self.constraints_visible.values() {
            for (coords, color) in mv.invariants() {
                if invariants.contains_key(&coords) {
                    assert_eq!(color, invariants[&coords]);
                }
                invariants.insert(coords, color);
                assert_eq!(Some(color), defn::color_of_cell(&defn[&coords]));
            }
        }
        invariants
    }

    fn compound_invariants(
        &self,
        env: &mut Env,
        defn: &Defn,
    ) -> Result<(BTreeMap<Coords, Color>, Difficulty), Box<dyn Error>> {
        // First construct the graph over visible constraints.
        let mut connections: BTreeMap<Coords, BTreeSet<Coords>> = self
            .constraints_visible
            .keys()
            .map(|k| (*k, BTreeSet::new()))
            .collect();
        for pair in self.constraints_visible.keys().combinations(2) {
            let [k0, k1]: [&Coords; 2] = pair.try_into().expect("Unreachable");
            if *k0 == *UNIQUE_COORDS || *k1 == *UNIQUE_COORDS {
                continue;
            }
            let mv0 = &self.constraints_visible[k0];
            let mv1 = &self.constraints_visible[k1];
            if !mv0.scope.is_disjoint(&mv1.scope) {
                connections.get_mut(k0).expect("Unreachable").insert(*k1);
                connections.get_mut(k1).expect("Unreachable").insert(*k0);
            }
        }

        // Then build the set of compound invariants, starting with one visible constraint per
        // group
        let mut constraints_groups: BTreeMap<BTreeSet<Coords>, Multiverse> = self
            .constraints_visible
            .iter()
            .map(|(k, v)| (BTreeSet::from([*k]), v.clone()))
            .collect();
        constraints_groups.remove(&BTreeSet::from([*UNIQUE_COORDS]));
        connections.remove(&*UNIQUE_COORDS);

        // Then escape if there are no visible constraints
        let mut invariants = BTreeMap::new();
        let mut difficulty = 2;
        if constraints_groups.is_empty() {
            return Ok((invariants, Difficulty::Local(difficulty)));
        }

        // Then loop until one or more invariants are found or that all the graph has been collapsed
        loop {
            // One loop consists of increasing the size of constraint groups by one.
            // The first loop starts with `constraints_groups` being one group per node of the graph
            // and ends with `constraints_groups` being one group per edge of the graph.

            // For each group so far, for each neighbor cell in the graph, create a new group that
            // merges the old group with that neighbor.
            for kset_old in constraints_groups.keys().cloned().collect::<Vec<_>>() {
                env.check_timeout()?;
                let mv_old = constraints_groups.remove(&kset_old).unwrap();
                let mut neighbor_contraints = BTreeSet::new();
                for k in &kset_old {
                    for k in &connections[k] {
                        if !kset_old.contains(k) {
                            neighbor_contraints.insert(k);
                        }
                    }
                }
                for k_new in &neighbor_contraints {
                    let mut kset_new = kset_old.clone();
                    kset_new.insert(**k_new);
                    if constraints_groups.contains_key(&kset_new) {
                        // A previous iteration already created that multiverse
                        continue;
                    }
                    let mv_new = &self.constraints_visible[k_new];
                    // `mv_old.merge(mv_new)` is computation intensive
                    constraints_groups.insert(kset_new, mv_old.merge(mv_new));
                }
            }

            // Look for invariants
            for mv in constraints_groups.values() {
                for (coords, color) in mv.invariants() {
                    if invariants.contains_key(&coords) {
                        assert_eq!(color, invariants[&coords]);
                    }
                    invariants.insert(coords, color);
                    assert_eq!(Some(color), defn::color_of_cell(&defn[&coords]));
                }
            }

            // Stop if necessary
            if !invariants.is_empty() {
                break;
            }
            if constraints_groups.is_empty() {
                break;
            }
            difficulty += 1;
        }
        Ok((invariants, Difficulty::Local(difficulty)))
    }

    fn global_invariants(
        &self,
        env: &mut Env,
        defn: &Defn,
    ) -> Result<BTreeMap<Coords, Color>, Box<dyn Error>> {
        let mut invariants = BTreeMap::new();
        // Using rev() here is a quick and dirty hack to make sure that the
        // global constraint is first in the fold. This greatly improves
        // runtime.
        let mut mv = Multiverse::empty();
        for mv2 in self.constraints_visible.values().rev() {
            env.check_timeout()?;
            mv = mv.merge(mv2);
        }
        for (coords, color) in mv.invariants() {
            if invariants.contains_key(&coords) {
                assert_eq!(color, invariants[&coords]);
            }
            invariants.insert(coords, color);
            assert_eq!(Some(color), defn::color_of_cell(&defn[&coords]));
        }
        Ok(invariants)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Findings {
    difficulty: Difficulty,
    cells: BTreeSet<Coords>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Outcome {
    Timeout,
    Unsolvable,
    Solved(Vec<Findings>),
}

pub fn difficulty_of_findings_vec(findings_vec: &Vec<Findings>) -> (Option<u32>, Option<u32>) {
    let mut max_local = None;
    let mut max_global = None;
    for findings in findings_vec {
        match findings.difficulty {
            Difficulty::Global(diff) => {
                max_global = Some(max_global.map_or(diff, |prev_max: u32| prev_max.max(diff)));
            }
            Difficulty::Local(diff) => {
                max_local = Some(max_local.map_or(diff, |prev_max: u32| prev_max.max(diff)));
            }
        }
    }
    (max_local, max_global)
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Unsolvable => write!(f, "Requires additional rules"),
            Outcome::Timeout => write!(f, "Timeout"),
            Outcome::Solved(findings_vec) => {
                let mut steps = 0;
                let mut max_local = None;
                let mut max_global = None;
                for findings in findings_vec {
                    steps += 1;
                    match findings.difficulty {
                        Difficulty::Global(diff) => {
                            max_global =
                                Some(max_global.map_or(diff, |prev_max: u32| prev_max.max(diff)));
                        }
                        Difficulty::Local(diff) => {
                            max_local =
                                Some(max_local.map_or(diff, |prev_max: u32| prev_max.max(diff)));
                        }
                    }
                }
                write!(
                    f,
                    "Solved steps:{} max-local-difficulty:{:?} max-global-difficulty:{:?}",
                    steps, max_local, max_global
                )
            }
        }
    }
}

pub fn solve(env: &mut Env, defn: &Defn, verbose: bool) -> Outcome {
    let mut progress = Progress::of_defn(defn);
    let mut constraints = Constraints::of_defn(defn);
    let mut history = vec![];
    let mut difficulty;
    loop {
        let visible_cells: BTreeSet<_> = progress.blacks.union(&progress.blues).cloned().collect();
        if verbose {
            println!(
                "Solver loop with visibles:{}, unknown:{}",
                visible_cells.len(),
                progress.unknowns.len(),
            );
        }

        // Step 1 - Transfer constraints from hidden to visible in order to reflect the status of
        // `progress`.
        constraints.reveal(&visible_cells);

        // Step 2 - Narrow down each of the visible constraints in order to reflect the status of
        // `progress`.
        constraints.narrow(&visible_cells, &progress);

        // Step 3 - Transfer visible constraints to exhausted if they don't carry uncertainty
        // anymore (i.e. the ones that were narrowed while `progress` knows all they scope).
        constraints.gc();

        // Step 4 - Check if finished
        if progress.is_solved() {
            assert!(constraints.is_solved());
            break;
        } else {
            assert!(!constraints.is_solved());
        }

        // Step 5.1 - Look for trivial invariants (i.e. previously unknown cells that can be infered
        // by looking at a single constraint).
        let mut invariants = constraints.trivial_invariants(defn);
        difficulty = Difficulty::Local(1);

        // Step 5.2 - Look for compound invariants, gradually increasing the level of cognitive load
        // for the player. (global constraint is exclduded here because it is likely to cause
        // combinatorial explosion, see step 5.3 for this)
        if invariants.is_empty() {
            env.reset_timer();
            (invariants, difficulty) = match constraints.compound_invariants(env, defn) {
                Ok(x) => x,
                Err(err) => match err.downcast::<env::Timeout>() {
                    Ok(_) => return Outcome::Timeout,
                    Err(_) => panic!("compound_invariants failed"),
                },
            };
        }

        // Step 5.3 - Look for invariants using the global constraints
        if invariants.is_empty() {
            difficulty =
                Difficulty::Global(constraints.constraints_visible.len().try_into().unwrap());
            invariants = match constraints.global_invariants(env, defn) {
                Ok(x) => x,
                Err(err) => match err.downcast::<env::Timeout>() {
                    Ok(_) => return Outcome::Timeout,
                    Err(_) => panic!("compound_invariants failed"),
                },
            };
            if invariants.is_empty() {
                return Outcome::Unsolvable;
            }
        }
        history.push(Findings {
            difficulty,
            cells: invariants.keys().cloned().collect(),
        });

        // Step 6 - Reflect findings in progress
        progress.update(invariants);
    }
    Outcome::Solved(history)
}
