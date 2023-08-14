use std::collections::BTreeMap;
use std::error::Error;

use misc::Coords;

type Grid33<T> = [[T; 33]; 33];

/// The definition of a hexcells puzzle.
/// Is uses cube coordinates for hexagons: https://www.redblobgames.com/grids/hexagons
/// It is computed by parsing a string: https://github.com/oprypin/sixcells
/// It is passed to the solver for solving.
pub type Defn = BTreeMap<Coords, Cell>;

fn char_grid_of_string(strdefn: &str) -> Result<Grid33<(char, char)>, Box<dyn Error>> {
    let mut grid = [[('_', '_'); 33]; 33];
    let strdefn: Vec<_> = strdefn.trim().split('\n').collect();
    if strdefn.len() != 38 {
        return Err(format!(
            "Wrong number of line in strdefn. Got {}, expected 38",
            strdefn.len()
        )
        .into());
    }
    let strdefn = &strdefn[5..];
    assert_eq!(strdefn.len(), 33);
    for (i, line) in strdefn.iter().enumerate() {
        let line = line.trim();
        if line.len() != 66 {
            return Err(format!(
                "All lines should have len 66, found one with len {}",
                line.len()
            )
            .into());
        }
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

/// `Cell` is the type of a single cell in a Hexcells level definition
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
        _ => Err(format!("Unknown left token:'{}'", c).into()),
    }
}

fn lex_right(c: char) -> Result<TokenRight, Box<dyn Error>> {
    type R = TokenRight;
    match c {
        '.' => Ok(R::Dot),
        '+' => Ok(R::Plus),
        'c' => Ok(R::C),
        'n' => Ok(R::N),
        _ => Err(format!("Unknown right token:'{}'", c).into()),
    }
}

fn parse_modifier(r: TokenRight) -> Modifier {
    type R = TokenRight;
    type M = Modifier;
    match r {
        R::Plus => M::Anywhere,
        R::C => M::Together,
        R::N => M::Separated,
        R::Dot => std::panic::panic_any(0),
    }
}

fn parse_cell(l: TokenLeft, r: TokenRight) -> Result<Cell, Box<dyn Error>> {
    type L = TokenLeft;
    type R = TokenRight;
    type O = Orientation;
    type C = Color;
    match (l, r) {
        (L::Dot, R::Dot) => Ok(Cell::Empty),
        (L::Dot, _right) => Err("Invalid pair A".into()),
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
        (L::SmallX, _right @ (R::C | R::N)) => Err("Invalid pair B".into()),
        (L::BigX, R::Dot) => Ok(Cell::Zone0 {
            revealed: true,
            color: C::Blue,
        }),
        (L::BigX, R::Plus) => Ok(Cell::Zone18 { revealed: true }),
        (L::BigX, _right @ (R::C | R::N)) => Err("Invalid pair C".into()),
        (_left @ (L::Slash | L::Backslash | L::Pipe), R::Dot) => Err("Invalid pair D".into()),
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
            let left = lex_left(*left)?;
            let right = lex_right(*right)?;
            let cell = parse_cell(left, right)?;
            dst[i][j] = cell
        }
    }
    Ok(dst)
}

enum Alignment {
    Odd,
    Even,
}

/// Attempt to turn a grid of `Cell` to a `Defn`. This includes converting from 2d grid coordinates
/// to cube coordinates.
/// In the 2d grid representation, half of the element are void, they are placeholders that lie
/// between two actual puzzle cells. These cells are expected to be `Empty`. `alignment` chooses
/// which subset of the string definition is void.
fn of_cell_grid(grid: Grid33<Cell>, alignment: Alignment) -> Result<Defn, Box<dyn Error>> {
    let (icorrection, jcorrection) = match alignment {
        Alignment::Even => (1, 0),
        Alignment::Odd => (0, 0),
    };
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
                    let (q, r, s) = (q as isize, r as isize, s as isize);
                    let c = Coords::new(q, r, s);
                    assert!(!map.contains_key(&c));
                    map.insert(c, *cell);
                }
                (false, _) => {
                    return Err("Bad alignment in hexcells definition".into());
                }
            }
        }
    }
    Ok(map)
}

/// Takes a string definition as found on reddit and lex/parse/type it to `Defn`. If the result is
/// `Ok` then the grid is a valid Hexcells puzzle.
pub fn of_string(strdefn: &str) -> Result<Defn, Box<dyn Error>> {
    // Step 1: Turn the string into 33x33 array of (char, char).
    let grid = char_grid_of_string(strdefn)?;

    // Step 2: Lex and parse the (char, char) to Cell.
    // - The lexing step is a direct translation of the left/right chars to TokenLeft/TokenRight.
    // - The parsing step is an exhaustive pattern matching of the tokens to a final Cell type.
    let grid = cell_grid_of_char_grid(grid)?;

    // Step 3: Turn the 33x33 Cell array to a Defn.
    match of_cell_grid(grid, Alignment::Even) {
        Err(_) => (),
        Ok(x) => return Ok(x),
    };
    match of_cell_grid(grid, Alignment::Odd) {
        Err(_) => (),
        Ok(x) => return Ok(x),
    };
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
