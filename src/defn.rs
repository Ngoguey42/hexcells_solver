use std::collections::BTreeMap;
use std::error::Error;

use misc::err;
use misc::Coords;

type Grid33<T> = [[T; 33]; 33];

// to learn: In OCaml that type would be called [t]. What are the rust
// conventions?
pub type Defn = BTreeMap<Coords, Cell>;

fn char_grid_of_string(defn: &String) -> Result<Grid33<(char, char)>, Box<dyn Error>> {
    let mut grid = [[('_', '_'); 33]; 33];
    let defn: Vec<_> = defn.trim().split("\n").collect();
    if defn.len() != 38 {
        return Err(format!(
            "Wrong number of line in defn. Got {}, expected 38",
            defn.len()
        )
        .into());
    }
    let defn = &defn[5..];
    assert_eq!(defn.len(), 33);
    for (i, line) in defn.iter().enumerate() {
        let line = line.trim();
        if line.len() != 66 {
            return Err(format!(
                "All lines should have len 66, found one with len {}",
                line.len()
            )
            .into());
        }
        // to-learn: I think that [collect] copies. Is there a copy-less way?
        let line: Vec<_> = line.chars().collect();
        for (j, chunk) in line.chunks(2).enumerate() {
            let (left, right) = match chunk {
                [left, right] => (left, right),
                _ => std::panic::panic_any(0),
            };
            grid[i][j] = (*left, *right)
        }
    }
    Ok(grid)
}

enum TokenLeft {
    Dot,
    SmallO,
    BigO,
    SmallX,
    BigX,
    Slash,
    Backslash,
    Pipe,
}

enum TokenRight {
    Dot,
    Plus,
    C,
    N,
}

#[derive(Copy, Clone, Debug)]
pub enum Modifier {
    Anywhere,
    Together,
    Separated,
}

#[derive(Copy, Clone, Debug)]
pub enum Orientation {
    BottomRight,
    Bottom,
    BottomLeft,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Color {
    Black,
    Blue,
}

// to learn: What's the difference between clone and copy? Why do I need both?
/// Cell is the type of a single cell in a Hexcells level definition
#[derive(Copy, Clone, Debug)]
pub enum Cell {
    Empty,
    Zone0 { revealed: bool, color: Color },
    Zone6 { revealed: bool, m: Modifier },
    Zone18 { revealed: bool },
    Line { o: Orientation, m: Modifier },
}

fn lex_left(c: char) -> Result<TokenLeft, Box<dyn Error>> {
    type L = TokenLeft;
    match c {
        '.' => Ok(L::Dot),
        'o' => Ok(L::SmallO),
        'O' => Ok(L::BigO),
        'x' => Ok(L::SmallX),
        'X' => Ok(L::BigX),
        '/' => Ok(L::Slash),
        '\\' => Ok(L::Backslash),
        '|' => Ok(L::Pipe),
        _ => err(&format!("Unknown left token:'{}'", c)),
    }
}

fn lex_right(c: char) -> Result<TokenRight, Box<dyn Error>> {
    type R = TokenRight;
    match c {
        '.' => Ok(R::Dot),
        '+' => Ok(R::Plus),
        'c' => Ok(R::C),
        'n' => Ok(R::N),
        _ => err(&format!("Unknown right token:'{}'", c)),
    }
}

fn parse_modifier(r: TokenRight) -> Modifier {
    type R = TokenRight;
    type M = Modifier;
    match r {
        R::Plus => M::Anywhere,
        R::C => M::Together,
        R::N => M::Separated,
        R::Dot =>
        // to learn: In other langues I would use [assert false]. Is panic
        // alright?
        {
            std::panic::panic_any(0)
        }
    }
}

fn parse_cell(l: TokenLeft, r: TokenRight) -> Result<Cell, Box<dyn Error>> {
    type L = TokenLeft;
    type R = TokenRight;
    type O = Orientation;
    type C = Color;
    match (l, r) {
        (L::Dot, R::Dot) => Ok(Cell::Empty),
        (L::Dot, _right) => err("Invalid pair A"),
        (L::SmallO, right @ (R::Plus | R::C | R::N)) => Ok(Cell::Zone6 {
            revealed: false,
            m: parse_modifier(right),
        }),
        (L::SmallO, R::Dot) => Ok(Cell::Zone0 {
            revealed: false,
            color: C::Black,
        }),
        (L::BigO, right @ (R::Plus | R::C | R::N)) => Ok(Cell::Zone6 {
            revealed: true,
            m: parse_modifier(right),
        }),
        (L::BigO, R::Dot) => Ok(Cell::Zone0 {
            revealed: true,
            color: C::Black,
        }),
        (L::SmallX, R::Dot) => Ok(Cell::Zone0 {
            revealed: false,
            color: C::Blue,
        }),
        (L::SmallX, R::Plus) => Ok(Cell::Zone18 { revealed: false }),
        (L::SmallX, _right @ (R::C | R::N)) => err("Invalid pair B"),
        (L::BigX, R::Dot) => Ok(Cell::Zone0 {
            revealed: true,
            color: C::Blue,
        }),
        (L::BigX, R::Plus) => Ok(Cell::Zone18 { revealed: true }),
        (L::BigX, _right @ (R::C | R::N)) => err("Invalid pair C"),
        (_left @ (L::Slash | L::Backslash | L::Pipe), R::Dot) => err("Invalid pair D"),
        (L::Slash, right @ (R::Plus | R::C | R::N)) => Ok(Cell::Line {
            o: O::BottomLeft,
            m: parse_modifier(right),
        }),
        (L::Backslash, right @ (R::Plus | R::C | R::N)) => Ok(Cell::Line {
            o: O::BottomRight,
            m: parse_modifier(right),
        }),
        (L::Pipe, right @ (R::Plus | R::C | R::N)) => Ok(Cell::Line {
            o: O::Bottom,
            m: parse_modifier(right),
        }),
    }
}

fn cell_grid_of_char_grid(src: Grid33<(char, char)>) -> Result<Grid33<Cell>, Box<dyn Error>> {
    let mut dst = [[Cell::Empty; 33]; 33];
    for (i, row) in src.iter().enumerate() {
        for (j, (left, right)) in row.iter().enumerate() {
            // to learn: I kinda guessed the [*] syntax. I need to make sure
            // that it does that I suspect.
            let left = lex_left(*left)?;
            let right = lex_right(*right)?;
            let cell = parse_cell(left, right)?;
            dst[i][j] = cell
        }
    }
    Ok(dst)
}

fn of_cell_grid(
    grid: Grid33<Cell>,
    icorrection: usize,
    jcorrection: usize,
) -> Result<Defn, Box<dyn Error>> {
    let mut map = BTreeMap::new();
    for (i, row) in grid.iter().enumerate() {
        let i = i + icorrection;
        let i = i as f64;
        for (j, cell) in row.iter().enumerate() {
            let j = j + jcorrection;
            let j = j as f64;
            let q = 0.0 * i + 1.0 * j;
            let r = 0.5 * i - 0.5 * j;
            let s = -0.5 * i - 0.5 * j;
            let whole = q.fract() == 0. && s.fract() == 0.;
            match (whole, cell) {
                (true | false, Cell::Empty) => (),
                (true, _) => {
                    // println!("    i:{}, j:{}, q:{}, r:{}, s:{}, {:?}", i, j, q, r, s, &cell);
                    let (q, r, s) = (q as isize, r as isize, s as isize);
                    let c = Coords::new(q, r, s);
                    assert!(!map.contains_key(&c));
                    map.insert(c, *cell);
                    ()
                }
                (false, _) => {
                    // println!("    i:{}, j:{}, q:{}, r:{}, s:{}, {:?}", i, j, q, r, s, &cell);
                    return err("Bad alignment in hexcells definition");
                }
            }
        }
    }
    Ok(map)
}

/// Takes a string as defined in https://www.redblobgames.com/grids/hexagons/
/// and lex/parse/type to [Cell].
pub fn of_string(defn: &String) -> Result<Defn, Box<dyn Error>> {
    // Step 1: Turn the string into 33x33 array of (char, char).
    let grid = char_grid_of_string(defn)?;

    // Step 2: Lex and parse the (char, char) to Cell.
    // - The lexing step is a direct translation of the left/right chars to
    // TokenLeft/TokenRight.
    // - The parsing step is an exhaustive pattern matching of the tokens to a
    // final Cell type.
    // to learn: Is the grid copied when passed to [cell_grid_of_char_grid]? I
    // don't want that.
    let grid = cell_grid_of_char_grid(grid)?;

    // Step 3: Turn the 33x33 array to a map, which will be the contained used
    // when solving.
    for (ci, cj) in [(0, 0), (1, 0)] {
        // println!("  delta: {} {}", ci, cj);
        match of_cell_grid(grid, ci, cj) {
            Err(_) => (),
            Ok(x) => return Ok(x),
        }
    }
    Err("Input grid is incompatible with cube coordinates. This happens because the level is made of at least 2 zones that are completely disjoint and that don't lie on the same hexagon tiling".into())
}

pub fn color_of_cell(cell: &Cell) -> Option<Color> {
    match cell {
        Cell::Empty => None,
        Cell::Line { .. } => None,
        Cell::Zone0 { color, .. } => Some(*color),
        Cell::Zone6 { .. } => Some(Color::Black),
        Cell::Zone18 { .. } => Some(Color::Blue),
    }
}
