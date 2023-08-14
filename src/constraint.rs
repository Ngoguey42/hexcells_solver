/// Conversion of game constraints from [Defn] to [Multiverse] ready for solving:
/// [line], [zone6] and [zone18]
use itertools::Itertools;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use defn;
use defn::Color;
use defn::Modifier;
use defn::Orientation;
use misc::Coords;
use multiverse::Layout;
use multiverse::Multiverse;

/// This multiverse constructor is common for Zone6 anywhere, Line anywhere and Zone18
/// The output contains a single layout
fn distribute_anywhere(scope_vec: &Vec<Coords>, blue_count: usize) -> Multiverse {
    if scope_vec.len() == 0 {
        assert!(blue_count == 0);
        return Multiverse::empty();
    }
    assert!(scope_vec.len() > 0);
    assert!(scope_vec.len() >= blue_count);
    let scope_set: BTreeSet<_> = scope_vec.iter().cloned().collect();
    let layout = Layout::new(BTreeMap::from([(scope_set.clone(), blue_count as u16)]));
    let layouts = vec![layout];
    Multiverse::new(scope_set, layouts)
}

/// This multiverse constructor is for Line together
/// The output has one layout per solution
fn distribute_together(scope_vec: &Vec<Coords>, blue_count: usize) -> Multiverse {
    assert!(scope_vec.len() > 0);
    assert!(scope_vec.len() >= blue_count);
    let scope_set: BTreeSet<_> = scope_vec.iter().cloned().collect();
    let solution_count = {
        if blue_count == 0 || blue_count == scope_vec.len() {
            // Without this branch we would create several identical layouts
            // which would not be incorrect but just be noise.
            1
        } else {
            scope_vec.len() - blue_count + 1
        }
    };
    assert!(solution_count >= 1);
    let mut layouts = vec![];
    for i0 in 0..solution_count {
        let mut blues = BTreeSet::new();
        let mut blacks = scope_set.clone();
        for i in i0..(i0 + blue_count) {
            let coords = scope_vec[i];
            assert!(blacks.remove(&coords));
            blues.insert(coords);
        }
        assert_eq!(blues.len(), blue_count);
        assert_eq!(blacks.len() + blues.len(), scope_vec.len());
        let mut map = BTreeMap::new();
        if !blues.is_empty() {
            map.insert(blues, blue_count as u16);
        }
        if !blacks.is_empty() {
            map.insert(blacks, 0);
        }
        layouts.push(Layout::new(map));
    }
    let mv = Multiverse::new(scope_set, layouts);
    assert_eq!(Some(solution_count as u64), mv.solution_count_upper_bound());
    mv
}

/// This multiverse constructor is for Line separated
/// It is the only constructor that creates layouts with overlapping solutions
fn distribute_separated(scope_vec: &Vec<Coords>, blue_count: usize) -> Multiverse {
    assert!(blue_count >= 2);
    assert!(scope_vec.len() >= 3);
    assert!(scope_vec.len() > blue_count);
    let scope_set: BTreeSet<_> = scope_vec.iter().cloned().collect();
    let pivot_position_count = scope_vec.len() - 2;
    let mut layouts = vec![];
    for ipivot in 1..(1 + pivot_position_count) {
        let mut before = BTreeSet::new();
        let pivot = BTreeSet::from([scope_vec[ipivot]]);
        let mut after = BTreeSet::new();
        for i in 0..ipivot {
            before.insert(scope_vec[i]);
        }
        for i in (ipivot + 1)..scope_vec.len() {
            after.insert(scope_vec[i]);
        }
        assert_eq!(before.len() + 1 + after.len(), scope_vec.len());
        for i in 1..blue_count {
            let j = blue_count - i;
            assert!(j >= 1);
            if i > before.len() || j > after.len() {
                continue;
            }
            layouts.push(Layout::new(BTreeMap::from([
                (before.clone(), i as u16),
                (pivot.clone(), 0),
                (after.clone(), j as u16),
            ])));
        }
    }
    Multiverse::new(scope_set, layouts)
}

fn has_compatible_contiguity(
    blues: &BTreeSet<usize>,
    blacks: &BTreeSet<usize>,
    together: bool,
) -> bool {
    let last_blue_idx = blues.last().expect("Can't be empty");
    let first_blue_idx = blues.first().expect("Can't be empty");
    let all_blues_togethers = last_blue_idx - first_blue_idx == blues.len() - 1;
    let last_black_idx = blacks.last().expect("Can't be empty");
    let first_black_idx = blacks.first().expect("Can't be empty");
    let all_blacks_togethers = last_black_idx - first_black_idx == blacks.len() - 1;
    match (all_blues_togethers, all_blacks_togethers) {
        (true, true) => {
            // Colors are groupped together and don't loop over index 0
            assert!(*first_blue_idx == 0 || *first_black_idx == 0);
            assert!(*last_blue_idx == 5 || *last_black_idx == 5);
            if !together {
                return false;
            };
        }
        (true, false) => {
            // Colors are grouped together and blacks loop over index 0
            if !together {
                return false;
            };
        }
        (false, true) => {
            // Colors are grouped together and blues loop over index 0
            if !together {
                return false;
            };
        }
        (false, false) => {
            // Colors are not grouped together
            if together {
                return false;
            };
        }
    };
    true
}

/// This multiverse constructor is for Zone6 together and Zone6 separated
/// The output contains one layout per solution
fn distribute_in_ring(
    scope_arr: &[(Coords, bool); 6],
    blue_count: usize,
    together: bool,
) -> Multiverse {
    if together {
        let scope_vec: Vec<_> = scope_arr
            .iter()
            .filter_map(|(coords, is_gap)| if *is_gap { None } else { Some(*coords) })
            .collect();
        if blue_count <= 1 || blue_count == scope_vec.len() {
            return distribute_anywhere(&scope_vec, blue_count);
        }
    } else {
        assert!(blue_count >= 2);
    }
    let scope_set: BTreeSet<_> = scope_arr
        .iter()
        .filter_map(|(c, is_gap)| if *is_gap { None } else { Some(*c) })
        .collect();
    let mut layouts = vec![];
    let idxs: BTreeSet<_> = (0..6).collect();
    for blues in idxs.iter().combinations(blue_count) {
        let blues: BTreeSet<_> = blues.iter().cloned().cloned().collect();
        let mut a_gap_is_blue = false;
        for i in &blues {
            if scope_arr[*i].1 {
                a_gap_is_blue = true;
                break;
            }
        }
        if a_gap_is_blue {
            continue;
        }
        let blacks: BTreeSet<_> = idxs.difference(&blues).cloned().collect();
        if !has_compatible_contiguity(&blues, &blacks, together) {
            continue;
        }
        let blues: BTreeSet<_> = blues.iter().map(|i| scope_arr[*i].0).collect();
        let blacks: BTreeSet<_> = blacks
            .iter()
            .filter(|i| !scope_arr[**i].1)
            .map(|i| scope_arr[*i].0)
            .collect();
        assert_eq!(scope_set.len(), blues.len() + blacks.len());
        let mut bc = vec![];
        bc.push((blues, blue_count as u16));
        if !blacks.is_empty() {
            bc.push((blacks, 0));
        }
        layouts.push(Layout::new(bc.into_iter().collect()));
    }
    assert!(layouts.len() > 0);
    Multiverse::new(scope_set, layouts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use misc;
    use multiverse::State;

    fn nk(n: u64, k: u64) -> u64 {
        misc::n_choose_k(n, k).unwrap()
    }

    fn mock_zone6_anywhere(center: &Coords, blue_count: usize) -> Multiverse {
        distribute_anywhere(&center.neighbors6().iter().cloned().collect(), blue_count)
    }

    fn mock_line_together(topmost: &Coords, cell_count: usize, blue_count: usize) -> Multiverse {
        // Towards down
        let mut scope_vec = vec![];
        for i in 0..(cell_count as isize) {
            scope_vec.push(Coords::new(
                topmost.q() + 0 * i,
                topmost.r() + 1 * i,
                topmost.s() - 1 * i,
            ))
        }
        distribute_together(&scope_vec, blue_count)
    }

    fn mock_ring_together(center: &Coords, blue_count: usize) -> Multiverse {
        distribute_in_ring(
            &center.neighbors6().map(|coords| (coords, false)),
            blue_count,
            true,
        )
    }

    fn mock_line_separated(topmost: &Coords, cell_count: usize, blue_count: usize) -> Multiverse {
        // Towards down
        let mut scope_vec = vec![];
        for i in 0..(cell_count as isize) {
            scope_vec.push(Coords::new(
                topmost.q() + 0 * i,
                topmost.r() + 1 * i,
                topmost.s() - 1 * i,
            ))
        }
        distribute_separated(&scope_vec, blue_count)
    }

    fn mock_ring_separated(center: &Coords, blue_count: usize) -> Multiverse {
        distribute_in_ring(
            &center.neighbors6().map(|coords| (coords, false)),
            blue_count,
            false,
        )
    }

    fn test_two_zone6_horizontal_neighbors(
        blue_count_left: usize,
        blue_count_right: usize,
        invariant_count: usize,
        solution_count: u64,
    ) {
        // Horizontal neighbors are not direct neighbors. They share 2 direct neighbors.
        let mv0 = mock_zone6_anywhere(&Coords::new(0, 0, 0), blue_count_left);
        let mv1 = mock_zone6_anywhere(&Coords::new(2, -1, -1), blue_count_right);
        let mv = mv0.merge(&mv1);
        let invariants = mv.invariants();
        assert_eq!(
            nk(6, blue_count_left as u64),
            mv0.solution_count_upper_bound().unwrap()
        );
        assert_eq!(
            nk(6, blue_count_right as u64),
            mv1.solution_count_upper_bound().unwrap()
        );
        assert_eq!(solution_count, mv.solution_count_upper_bound().unwrap());
        assert_eq!(invariants.len(), invariant_count);
    }

    #[test]
    pub fn test_zone6() {
        test_two_zone6_horizontal_neighbors(0, 0, 10, 1);
        test_two_zone6_horizontal_neighbors(0, 1, 6, 4);
        test_two_zone6_horizontal_neighbors(1, 1, 0, nk(2, 1) + nk(4, 1) * nk(4, 1));
        test_two_zone6_horizontal_neighbors(
            2,
            2,
            0,
            nk(4, 2) * nk(4, 2) + nk(4, 1) * nk(4, 1) * nk(2, 1) + 1,
        );
        test_two_zone6_horizontal_neighbors(
            4,
            4,
            0,
            nk(4, 2) * nk(4, 2) + nk(4, 1) * nk(4, 1) * nk(2, 1) + 1,
        );
        test_two_zone6_horizontal_neighbors(5, 5, 0, nk(2, 1) + nk(4, 1) * nk(4, 1));
        test_two_zone6_horizontal_neighbors(6, 5, 6, 4);
        test_two_zone6_horizontal_neighbors(6, 6, 10, 1);
    }

    #[test]
    pub fn test_line_together() {
        // A line of len 5 with 3 together blues
        let mv0 = mock_line_together(&Coords::new(0, 0, 0), 5, 3);
        assert_eq!(3, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(1, mv0.invariants().len()); // The one in the middle is always blue

        // A black circle intersecting on the last 2 cells of the line
        let mv1 = mock_zone6_anywhere(&Coords::new(-1, 4, -3), 0);
        let mv = mv0.merge(&mv1);
        assert_eq!(1, mv.solution_count_upper_bound().unwrap());
        assert_eq!(9, mv.invariants().len());

        // A black circle intersecting on the middle cell and the one below
        let mv1 = mock_zone6_anywhere(&Coords::new(-1, 3, -2), 0);
        let mv = mv0.merge(&mv1);
        assert_eq!(0, mv.solution_count_upper_bound().unwrap());

        // A blue circle intersecting on the middle cell and the one below
        let mv1 = mock_zone6_anywhere(&Coords::new(-1, 3, -2), 6);
        let mv = mv0.merge(&mv1);
        assert_eq!(2, mv.solution_count_upper_bound().unwrap());
        assert_eq!(7, mv.invariants().len()); // The 6 of the circle plus the topmost

        // A line of len 5 with 1 (together) blue
        let mv0 = mock_line_together(&Coords::new(0, 0, 0), 5, 1);
        assert_eq!(5, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(0, mv0.invariants().len());

        // A line of len 5 with 0 (together) blue
        let mv0 = mock_line_together(&Coords::new(0, 0, 0), 5, 0);
        assert_eq!(1, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(5, mv0.invariants().len());

        // A line of len 5 with 5 (together) blues
        let mv0 = mock_line_together(&Coords::new(0, 0, 0), 5, 5);
        assert_eq!(1, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(5, mv0.invariants().len());
    }

    #[test]
    pub fn test_line_separated() {
        // A line of len 3 with 2 separated blues (minimal for separated)
        let mv0 = mock_line_separated(&Coords::new(0, 0, 0), 3, 2);
        assert_eq!(1, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(3, mv0.invariants().len());

        // A line of len 4 with 2 separated blues
        let mv0 = mock_line_separated(&Coords::new(0, 0, 0), 4, 2);
        assert_eq!(4, mv0.solution_count_upper_bound().unwrap()); // Reality is 3 but the algorithm produced overlapping layouts
        assert_eq!(0, mv0.invariants().len());

        // A line of len 4 with 3 separated blues
        let mv0 = mock_line_separated(&Coords::new(0, 0, 0), 4, 3);
        assert_eq!(2, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(2, mv0.invariants().len()); // The 2 extermities

        // A line of len 5 with 3 separated blues
        let mv0 = mock_line_separated(&Coords::new(0, 0, 0), 5, 3);
        assert_eq!(10, mv0.solution_count_upper_bound().unwrap()); // Reality is 7 but the algorithm produced overlapping layouts
        assert_eq!(0, mv0.invariants().len());

        // A black circle intersecting on the middle cell and the one below
        let mv1 = mock_zone6_anywhere(&Coords::new(-1, 3, -2), 0);
        let mv = mv0.merge(&mv1);
        assert_eq!(2, mv.solution_count_upper_bound().unwrap()); // Reality is 1 but the algorithm produced overlapping layouts
        assert_eq!(9, mv.invariants().len());

        // A blue circle intersecting on the middle cell and the one below
        let mv1 = mock_zone6_anywhere(&Coords::new(-1, 3, -2), 6);
        let mv = mv0.merge(&mv1);
        assert_eq!(1, mv.solution_count_upper_bound().unwrap());
        assert_eq!(9, mv.invariants().len());
    }

    #[test]
    pub fn test_ring_together() {
        for blue_count in [0, 6] {
            let mv0 = mock_ring_together(&Coords::new(0, 0, 0), blue_count);
            assert_eq!(1, mv0.solution_count_upper_bound().unwrap());
            assert_eq!(6, mv0.invariants().len());
        }
        for blue_count in [1, 2, 3, 4, 5] {
            let mv0 = mock_ring_together(&Coords::new(0, 0, 0), blue_count);
            assert_eq!(6, mv0.solution_count_upper_bound().unwrap());
            assert_eq!(0, mv0.invariants().len());
        }

        // A line of len 5 with 3 together blues
        let mv0 = mock_line_together(&Coords::new(0, 0, 0), 5, 3);
        // A circle intersecting on the middle cell and the one below
        let mv1 = mock_ring_together(&Coords::new(-1, 3, -2), 4);
        let mv = mv0.merge(&mv1);
        assert_eq!(7, mv.solution_count_upper_bound().unwrap());
        assert_eq!(1, mv.invariants().len()); // The leftmost of the ring

        let mv0 = mock_zone6_anywhere(&Coords::new(0, 0, 0), 4);
        let mv1 = mock_ring_together(&Coords::new(0, 0, 0), 4);
        let mv = mv0.merge(&mv1);
        assert_eq!(6, mv.solution_count_upper_bound().unwrap());
        assert_eq!(0, mv.invariants().len());

        // A triangle. Pairwise they have intersections of 2 cells
        let mv0 = mock_zone6_anywhere(&Coords::new(0, 0, 0), 6);
        let mv1 = mock_ring_together(&Coords::new(2, -1, -1), 3);
        let mv2 = mock_ring_together(&Coords::new(1, -2, 1), 3);
        let mv = mv0.merge(&mv1).merge(&mv2);
        assert_eq!(2, mv.solution_count_upper_bound().unwrap());
        assert_eq!(10, mv.invariants().len());
    }

    #[test]
    pub fn test_ring_separated() {
        let mv0 = mock_ring_separated(&Coords::new(0, 0, 0), 2);
        assert_eq!(9, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(0, mv0.invariants().len());

        let mv0 = mock_ring_separated(&Coords::new(0, 0, 0), 3);
        assert_eq!(14, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(0, mv0.invariants().len());

        let mv0 = mock_ring_separated(&Coords::new(0, 0, 0), 4);
        assert_eq!(9, mv0.solution_count_upper_bound().unwrap());
        assert_eq!(0, mv0.invariants().len());

        let mv0 = mock_ring_separated(&Coords::new(0, 0, 0), 3);
        let mv1 = mock_zone6_anywhere(&Coords::new(2, -1, -1), 6);
        let mv = mv0.merge(&mv1);
        assert_eq!(2, mv.solution_count_upper_bound().unwrap());
        assert_eq!(8, mv.invariants().len());

        let mv0 = mock_ring_separated(&Coords::new(0, 0, 0), 2);
        let mv1 = mock_zone6_anywhere(&Coords::new(2, -1, -1), 5);
        let mv = mv0.merge(&mv1);
        assert_eq!(6, mv.solution_count_upper_bound().unwrap());
        assert_eq!(4, mv.invariants().len());
    }

    #[test]
    pub fn test_multiverse_edge_cases() {
        // Flavors of empty
        let empty = Multiverse::empty();
        assert_eq!(empty.state(), State::Empty);
        assert_eq!(0, empty.solution_count_upper_bound().unwrap());
        assert!(empty.invariants().len() == 0);
        let empty = Multiverse::new(BTreeSet::new(), vec![]);
        assert_eq!(empty.state(), State::Empty);
        assert_eq!(0, empty.solution_count_upper_bound().unwrap());
        assert!(empty.invariants().len() == 0);
        let empty = empty.merge(&empty);
        assert_eq!(empty.state(), State::Empty);
        assert_eq!(0, empty.solution_count_upper_bound().unwrap());
        assert!(empty.invariants().len() == 0);

        // Intersection with empty
        let c = Coords::new(0, 0, 0);
        let running = mock_zone6_anywhere(&c, 3);
        let running = empty.merge(&running);
        assert_eq!(running.state(), State::Running);
        assert_eq!(nk(6, 3), running.solution_count_upper_bound().unwrap());
        assert!(running.invariants().len() == 0);

        // Stuck
        let stuck = Multiverse::new(BTreeSet::from([c]), vec![]);
        assert_eq!(stuck.state(), State::Stuck);
        assert_eq!(0, stuck.solution_count_upper_bound().unwrap());
        // (Undefined result in stuck.invariants())

        // Disjoint scopes
        let c2 = Coords::new(10, 0, -10);
        let running2 = mock_zone6_anywhere(&c2, 3);
        let mv = running2.merge(&running);
        assert_eq!(mv.state(), State::Running);
        assert_eq!(nk(6, 3).pow(2), mv.solution_count_upper_bound().unwrap());
        assert!(mv.invariants().len() == 0);
    }
}

pub fn zone6(defn: &defn::Defn, coords: Coords, modifier: Modifier) -> Multiverse {
    let mut blue_count = 0;
    let neighborhood = coords.neighbors6();
    let scope_arr = neighborhood.map(|c| match defn.get(&c).and_then(defn::color_of_cell) {
        None => (c, true),
        Some(Color::Blue) => {
            blue_count += 1;
            (c, false)
        }
        Some(Color::Black) => (c, false),
    });
    match modifier {
        Modifier::Anywhere => {
            let scope = scope_arr
                .iter()
                .filter_map(|(c, is_gap)| if *is_gap { None } else { Some(*c) })
                .collect();
            distribute_anywhere(&scope, blue_count)
        }
        Modifier::Together => distribute_in_ring(&scope_arr, blue_count, true),
        Modifier::Separated => distribute_in_ring(&scope_arr, blue_count, false),
    }
}

pub fn zone18(defn: &defn::Defn, coords: Coords) -> Multiverse {
    let mut scope = Vec::new();
    let mut blue_count = 0;
    for c in coords.neighbors18() {
        match defn.get(&c).and_then(defn::color_of_cell) {
            None => (),
            Some(Color::Blue) => {
                blue_count += 1;
                scope.push(c);
            }
            Some(Color::Black) => {
                scope.push(c);
            }
        }
    }
    distribute_anywhere(&scope, blue_count)
}

pub fn line(
    defn: &defn::Defn,
    coords: Coords,
    orientation: Orientation,
    modifier: Modifier,
) -> Multiverse {
    let (dq, dr, ds) = match orientation {
        Orientation::Bottom => (0, 1, -1),
        Orientation::BottomRight => (1, 0, -1),
        Orientation::BottomLeft => (-1, 1, 0),
    };
    let (q, r, s) = (coords.q(), coords.r(), coords.s());
    let mut scope = Vec::new();
    let mut blue_count = 0;
    for i in 0..33 {
        // 33 is more than the max diagonal len of a grid
        let c = Coords::new(q + dq * i, r + dr * i, s + ds * i);
        match defn.get(&c).and_then(defn::color_of_cell) {
            None => (),
            Some(Color::Blue) => {
                blue_count += 1;
                scope.push(c);
            }
            Some(Color::Black) => {
                scope.push(c);
            }
        }
    }
    match modifier {
        Modifier::Anywhere => distribute_anywhere(&scope, blue_count),
        Modifier::Together => distribute_together(&scope, blue_count),
        Modifier::Separated => distribute_separated(&scope, blue_count),
    }
}

pub fn global_blue_count(defn: &defn::Defn) -> Multiverse {
    let mut scope = Vec::new();
    let mut blue_count = 0;
    for (c, cell) in defn {
        match defn::color_of_cell(cell) {
            None => (),
            Some(Color::Blue) => {
                blue_count += 1;
                scope.push(*c);
            }
            Some(Color::Black) => {
                scope.push(*c);
            }
        }
    }
    distribute_anywhere(&scope, blue_count)
}
