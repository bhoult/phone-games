//! Designs every level after the first day and writes them to `src/later.rs`.
//!
//!   cargo run -p rufus-core --release --example mklevels                    all 1000
//!   cargo run -p rufus-core --release --example mklevels 60 100 /tmp/x.rs   a quick look
//!
//! Nobody is going to draw a thousand boards by hand, so this does what the
//! hand-made levels went through, a thousand times over. For each level it:
//!
//! 1. picks what the level is about from its chapter's place (bones at the
//!    park, sticks and ponds at the water, mud, parcels indoors, a bit of
//!    everything at night), then draws a board, obstacles and goals;
//! 2. plays it with the casual stand-in and scales the goals until the level
//!    takes about as many moves as it is meant to;
//! 3. throws the design away and draws another if it plays badly: needing
//!    reshuffles, swinging wildly from game to game, a goal that is only
//!    decoration, a board that can jam;
//! 4. tunes the Easy, Normal and Hard move budgets exactly as
//!    `tools/apply_tune.py` does for the hand-made levels.
//!
//! The result is ordinary level data in a source file. The game never runs
//! any of this.

use rufus_core::board::{Step, COLS, ROWS};
use rufus_core::bot::{self, BotCfg};
use rufus_core::level::*;
use rufus_core::tune::{self, par_map, pct, Tuned, LOTS};
use rufus_core::Rng;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

type Mask = [[bool; COLS]; ROWS];
type Grid = [[u8; COLS]; ROWS];

const MID: i32 = COLS as i32 / 2;

// ---- what a level is about ---------------------------------------------------

/// A second thing to do on a level that is mainly about an obstacle.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Extra {
    None,
    Score,
    Kind,
    Kinds2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Recipe {
    Score,
    Kinds(u8),
    KindScore,
    Mud(Extra),
    Boxes(Extra),
    Bones(Extra),
    MudBoxes,
    BonesMud,
}

impl Recipe {
    /// Roughly how much there is to think about. Chapters run simple to busy.
    fn weight(self) -> u32 {
        use Recipe::*;
        match self {
            Score | Kinds(1) => 1,
            Kinds(2) | KindScore | Mud(Extra::None) | Boxes(Extra::None) | Bones(Extra::None) => 2,
            Kinds(_) | Mud(Extra::Score | Extra::Kind) | Boxes(Extra::Score | Extra::Kind) | Bones(Extra::Score | Extra::Kind) => 3,
            _ => 4,
        }
    }

    fn has_bones(self) -> bool {
        matches!(self, Recipe::Bones(_) | Recipe::BonesMud)
    }
}

/// What each place serves, and how often.
fn menu(place: Place) -> &'static [(Recipe, u32)] {
    use Extra as X;
    use Recipe::*;
    match place {
        Place::Yard => &[(Kinds(1), 3), (Kinds(2), 4), (Kinds(3), 3), (KindScore, 3), (Score, 3), (Bones(X::None), 1), (Mud(X::Score), 1), (Boxes(X::Kind), 1)],
        Place::Park => &[(Bones(X::None), 4), (Bones(X::Kind), 4), (Bones(X::Kinds2), 2), (Kinds(2), 3), (Kinds(3), 2), (Score, 2), (KindScore, 2), (BonesMud, 1), (Boxes(X::None), 1)],
        Place::Lake => &[(Kinds(1), 3), (Kinds(2), 4), (Kinds(3), 3), (KindScore, 3), (Score, 3), (Bones(X::Kind), 2), (Mud(X::Kind), 1)],
        Place::Mud => &[(Mud(X::None), 4), (Mud(X::Score), 4), (Mud(X::Kind), 4), (Mud(X::Kinds2), 2), (BonesMud, 2), (MudBoxes, 1), (Kinds(2), 1), (Score, 1)],
        Place::Home => &[(Boxes(X::None), 4), (Boxes(X::Score), 4), (Boxes(X::Kind), 4), (Boxes(X::Kinds2), 2), (Bones(X::None), 1), (Kinds(2), 2), (MudBoxes, 1), (KindScore, 1)],
        Place::Night => &[(Kinds(3), 3), (Score, 2), (Mud(X::Kind), 2), (Boxes(X::Kind), 2), (Bones(X::Kinds2), 2), (MudBoxes, 3), (BonesMud, 2), (Kinds(2), 2), (Mud(X::None), 1), (Bones(X::Kind), 2), (KindScore, 1)],
    }
}

/// The six recipes of a chapter: drawn from its place's menu, nothing more
/// than twice, simplest first.
fn chapter_plan(chapter: usize) -> Vec<Recipe> {
    let mut rng = Rng::new(0x5EED_C4A7 ^ (chapter as u64).wrapping_mul(0x9E37_79B9));
    let menu = menu(CHAPTERS[chapter].place);
    let total: u32 = menu.iter().map(|m| m.1).sum();
    let mut plan: Vec<Recipe> = Vec::new();
    while plan.len() < CHAPTER_LEN {
        let mut roll = rng.below(total);
        let pick = menu.iter().find(|m| if roll < m.1 { true } else { roll -= m.1; false }).unwrap().0;
        if plan.iter().filter(|&&r| r == pick).count() < 2 {
            plan.push(pick);
        }
    }
    let mut keyed: Vec<(u32, Recipe)> = plan.into_iter().map(|r| (r.weight() * 10 + rng.below(14), r)).collect();
    keyed.sort();
    let mut plan: Vec<Recipe> = keyed.into_iter().map(|k| k.1).collect();
    // the same thing twice in a row reads as a repeat: pull them apart
    for i in 1..plan.len() {
        if plan[i] == plan[i - 1] {
            if let Some(j) = (i + 1..plan.len()).find(|&j| plan[j] != plan[i]) {
                plan.swap(i, j);
            }
        }
    }
    plan
}

/// How many moves the casual stand-in should need. Chapters open with short
/// levels and close with long ones.
fn length(n: usize) -> u32 {
    let mut rng = Rng::new(0x1E46 + n as u64);
    [15, 17, 18, 19, 21, 23][n % CHAPTER_LEN] + rng.below(5)
}

// ---- boards ---------------------------------------------------------------------

fn block(top: usize, bottom: usize) -> Mask {
    let mut m = [[false; COLS]; ROWS];
    for (y, row) in m.iter_mut().enumerate() {
        if y >= top && y < ROWS - bottom {
            *row = [true; COLS];
        }
    }
    m
}

/// A board from how far each row is cut in from both sides.
fn profile(insets: [u8; ROWS]) -> Mask {
    let mut m = [[false; COLS]; ROWS];
    for y in 0..ROWS {
        let inset = insets[y] as usize;
        for x in inset..COLS.saturating_sub(inset) {
            m[y][x] = true;
        }
    }
    m
}

#[rustfmt::skip]
const PROFILES: [[u8; ROWS]; 20] = [
    [3, 2, 1, 0, 0, 0, 1, 2, 3], // diamond
    [2, 1, 0, 0, 0, 0, 0, 1, 2], // octagon
    [1, 0, 0, 0, 0, 0, 0, 0, 1], // rounded
    [0, 0, 1, 1, 2, 1, 1, 0, 0], // hourglass
    [1, 1, 0, 0, 0, 0, 0, 1, 1], // tabs
    [3, 2, 2, 1, 1, 0, 0, 0, 0], // pyramid
    [2, 2, 0, 0, 0, 0, 0, 2, 2], // plus
    [2, 2, 1, 1, 0, 0, 0, 0, 0], // steps
    [1, 0, 0, 1, 1, 1, 0, 0, 1], // dumbbell
    [0, 1, 0, 1, 0, 1, 0, 1, 0], // battlements
    [4, 1, 0, 0, 0, 0, 0, 1, 4], // short and rounded
    [2, 1, 0, 0, 0, 0, 0, 0, 0], // dome
    [0, 0, 0, 1, 2, 1, 0, 0, 0], // pinch
    [1, 0, 1, 0, 1, 0, 1, 0, 1], // saw
    [4, 0, 0, 1, 1, 1, 0, 0, 4], // short waist
    [3, 2, 1, 0, 0, 0, 0, 0, 0], // spire
    [2, 1, 1, 0, 0, 0, 1, 1, 2], // barrel
    [1, 1, 1, 0, 0, 0, 1, 1, 1], // wide middle
    [0, 0, 0, 1, 1, 1, 0, 0, 0], // waist
    [0, 0, 2, 2, 0, 2, 2, 0, 0], // windows
];

/// Holes in the middle of a board, as (column, rows below the first row).
#[rustfmt::skip]
const PONDS: [&[(usize, usize)]; 17] = [
    &[(2, 0), (3, 0), (4, 0), (2, 1), (3, 1), (4, 1)],
    &[(3, 0)],
    &[(3, 0), (2, 1), (3, 1), (4, 1), (3, 2)],
    &[(1, 0), (5, 0)],
    &[(1, 0), (5, 0), (1, 1), (5, 1)],
    &[(1, 0), (2, 0), (4, 0), (5, 0), (1, 1), (2, 1), (4, 1), (5, 1)],
    &[(3, 0), (3, 1), (3, 2)],
    &[(1, 0), (5, 0), (1, 3), (5, 3)],
    &[(2, 0), (3, 0), (4, 0)],
    &[(2, 0), (4, 0), (3, 1)],
    &[(3, 0), (3, 3)],
    &[(2, 0), (4, 0)],
    &[(2, 0), (4, 0), (2, 2), (4, 2)],
    &[(3, 0), (2, 1), (4, 1), (3, 2)],
    &[(1, 0), (2, 0), (4, 0), (5, 0)],
    &[(3, 0), (3, 1)],
    &[(2, 0), (3, 0), (4, 0), (3, 1)],
];

fn cells(m: &Mask) -> usize {
    m.iter().flatten().filter(|&&c| c).count()
}

fn column_height(m: &Mask, x: usize) -> usize {
    (0..ROWS).filter(|&y| m[y][x]).count()
}

fn rows_used(m: &Mask) -> (usize, usize) {
    let used: Vec<usize> = (0..ROWS).filter(|&y| m[y].iter().any(|&c| c)).collect();
    (used[0], *used.last().unwrap())
}

fn flip(m: Mask) -> Mask {
    let mut out = m;
    out.reverse();
    out
}

/// Somewhere a match-3 can actually be played. Bones need tall columns: a
/// short one would hand them over for nothing.
fn fits(m: &Mask, tall: bool) -> bool {
    let heights: Vec<usize> = (0..COLS).map(|x| column_height(m, x)).collect();
    let widths: Vec<usize> = (0..ROWS).map(|y| m[y].iter().filter(|&&c| c).count()).collect();
    cells(m) >= 38
        && heights.iter().filter(|&&h| h > 0).count() >= 6
        && heights.iter().all(|&h| h == 0 || h >= if tall { 6 } else { 4 })
        && widths.iter().all(|&w| w == 0 || w >= 3)
        && widths.iter().filter(|&&w| w > 0).count() >= 6
        && (!tall || heights.iter().all(|&h| h > 0))
}

fn pond(rng: &mut Rng) -> Mask {
    let mut m = match rng.below(10) {
        0 | 1 | 2 => block(0, 0),
        3 => block(1, 0),
        4 => block(0, 1),
        5 => block(1, 1),
        6 => profile(PROFILES[2]),
        7 => profile(PROFILES[1]),
        8 => profile(PROFILES[4]),
        _ => profile(PROFILES[11]),
    };
    let holes = PONDS[rng.below(PONDS.len() as u32) as usize];
    let tall = holes.iter().map(|h| h.1).max().unwrap();
    let (top, bottom) = rows_used(&m);
    // not in the board's first or last row: that would just be a notch
    let room = (bottom - 1).saturating_sub(top + 1 + tall);
    let y0 = top + 1 + rng.below(room as u32 + 1) as usize;
    for &(x, dy) in holes {
        if y0 + dy < ROWS {
            m[y0 + dy][x] = false;
        }
    }
    m
}

fn corners(rng: &mut Rng) -> Mask {
    let k = 1 + rng.below(3) as usize;
    let which = rng.below(4);
    let mut insets = [0u8; ROWS];
    for r in 0..k {
        if which != 2 {
            insets[r] = (k - r) as u8;
        }
        if which != 3 {
            insets[ROWS - 1 - r] = (k - r) as u8;
        }
    }
    profile(insets)
}

fn notches(rng: &mut Rng) -> Mask {
    let mut m = block(rng.below(2) as usize, 0);
    let (rows, depth): (&[usize], usize) = match rng.below(6) {
        0 => (&[3, 4, 5], 1),
        1 => (&[2, 6], 1),
        2 => (&[1, 4, 7], 1),
        3 => (&[4], 2),
        4 => (&[2, 3, 5, 6], 1),
        _ => (&[3, 5], 2),
    };
    for &y in rows {
        for d in 0..depth {
            m[y][d] = false;
            m[y][COLS - 1 - d] = false;
        }
    }
    m
}

fn split(rng: &mut Rng) -> Mask {
    let mut m = block(0, 0);
    match rng.below(7) {
        0 => m[3 + rng.below(3) as usize] = [false; COLS],
        1 => {
            let y = 3 + rng.below(3) as usize;
            for x in 1..COLS - 1 {
                m[y][x] = false;
            }
        }
        2 => {
            for row in m.iter_mut().take(4) {
                row[3] = false;
            }
        }
        3 => {
            for row in m.iter_mut().skip(5) {
                row[3] = false;
            }
        }
        4 => {
            for row in m.iter_mut() {
                row[3] = false;
            }
        }
        5 => {
            for y in [2, 6] {
                for x in 2..=4 {
                    m[y][x] = false;
                }
            }
        }
        _ => {
            for y in 2..=6 {
                m[y][3] = false;
            }
        }
    }
    m
}

fn shape(place: Place, tall: bool, rng: &mut Rng) -> Mask {
    // the water is where the ponds are
    let ponds = if place == Place::Lake { 55 } else { 16 };
    for _ in 0..400 {
        let m = if rng.below(100) < ponds {
            pond(rng)
        } else {
            match rng.below(20) {
                0 | 1 => block(rng.below(3) as usize, rng.below(2) as usize),
                2..=10 => {
                    let m = profile(PROFILES[rng.below(PROFILES.len() as u32) as usize]);
                    if rng.chance(0.5) {
                        flip(m)
                    } else {
                        m
                    }
                }
                11..=13 => corners(rng),
                14..=16 => notches(rng),
                _ => split(rng),
            }
        };
        if fits(&m, tall) {
            return m;
        }
    }
    block(0, 0)
}

// ---- obstacles --------------------------------------------------------------------

/// A cell and how many layers of whatever goes on it.
type Spot = (usize, usize, u8);

fn every(m: &Mask, keep: impl Fn(usize, usize) -> bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for y in 0..ROWS {
        for x in 0..COLS {
            if m[y][x] && keep(x, y) {
                out.push((x, y));
            }
        }
    }
    out
}

fn on(m: &Mask, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as usize) < COLS && (y as usize) < ROWS && m[y as usize][x as usize]
}

fn dx(x: usize) -> i32 {
    (x as i32 - MID).abs()
}

fn mud_layout(m: &Mask, rng: &mut Rng) -> Vec<Spot> {
    let (top, bottom) = rows_used(m);
    let cy = ((top + bottom) / 2) as i32;
    let dy = |y: usize| (y as i32 - cy).abs();
    let span = bottom - top + 1;
    let mut where_: Vec<(usize, usize)> = Vec::new();
    for _ in 0..60 {
        let run = 3 + rng.below(4) as usize;
        let from = top + rng.below((span.saturating_sub(run) + 1) as u32) as usize;
        let k = 2 + rng.below(3) as usize;
        let odd = rng.below(2) as usize;
        let p = rng.range(0.25, 0.5);
        where_ = match rng.below(15) {
            0 => every(m, |_, y| y >= from && y < from + run),
            1 => every(m, |_, _| true),
            2 => every(m, |x, y| [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().any(|&(a, b)| !on(m, x as i32 + a, y as i32 + b))),
            3 => every(m, |x, y| dx(x) <= 1 + odd as i32 && y >= from && y < from + run),
            4 => every(m, |x, y| (x + y) % 2 == odd),
            5 => every(m, |x, y| (dx(x) - dy(y)).abs() <= odd as i32),
            6 => every(m, |x, _| dx(x) >= 3 - odd as i32),
            7 => every(m, |x, _| dx(x) <= odd as i32),
            8 => every(m, |_, y| y + k > bottom),
            9 => {
                let mut picked = Vec::new();
                for y in top..=bottom {
                    for x in 0..=MID as usize {
                        if rng.chance(p) {
                            picked.push((x, y));
                            picked.push((COLS - 1 - x, y));
                        }
                    }
                }
                every(m, |x, y| picked.contains(&(x, y)))
            }
            10 => every(m, |_, y| (y - top) % 2 == odd),
            11 => every(m, |x, y| dx(x) >= 4 - k as i32 && (y < top + k - 1 || y + k - 1 > bottom)),
            12 => every(m, |_, y| y < top + k),
            13 => every(m, |x, y| dx(x) + dy(y) <= k as i32),
            _ => every(m, |x, y| dx(x).max(dy(y)) == 2),
        };
        if (8..=54).contains(&where_.len()) {
            break;
        }
    }
    let is_mud = |x: i32, y: i32| x >= 0 && y >= 0 && where_.contains(&(x as usize, y as usize));
    let style = rng.below(100);
    let few = where_.len() <= 28;
    where_
        .iter()
        .map(|&(x, y)| {
            let (xi, yi) = (x as i32, y as i32);
            let deep = match style {
                0..=39 => false,
                40..=54 => few,
                55..=74 => is_mud(xi - 1, yi) && is_mud(xi + 1, yi) && is_mud(xi, yi - 1) && is_mud(xi, yi + 1),
                75..=89 => y as i32 > cy,
                _ => (x + y) % 2 == 0,
            };
            (x, y, 1 + deep as u8)
        })
        .collect()
}

fn box_layout(m: &Mask, rng: &mut Rng, most: usize) -> Vec<Spot> {
    let (top, bottom) = rows_used(m);
    let cy = ((top + bottom) / 2) as i32;
    let mut where_: Vec<(usize, usize)> = Vec::new();
    for _ in 0..80 {
        // never in the board's first row: nothing could get past to open them
        let y0 = top + 1 + rng.below((bottom - top) as u32) as usize;
        let pairs = 2 + rng.below(4);
        where_ = match rng.below(12) {
            0 => every(m, |x, y| y == y0 && x % 2 == 1),
            1 => every(m, |x, y| y == y0 && x % 2 == 0),
            2 => every(m, |x, y| y == y0 && dx(x) <= 2),
            3 => every(m, |x, y| (y == y0 || y == y0 + 2) && dx(x) <= 1),
            4 => {
                let mut picked = Vec::new();
                for _ in 0..pairs {
                    let (x, y) = (rng.below(MID as u32 + 1) as usize, top + 1 + rng.below((bottom - top) as u32) as usize);
                    picked.push((x, y));
                    picked.push((COLS - 1 - x, y));
                }
                every(m, |x, y| picked.contains(&(x, y)))
            }
            5 => every(m, |x, y| y == bottom || (y + 1 == bottom && x % 2 == 0 && pairs > 3)),
            6 => every(m, |x, y| (y == y0 || y == y0 + 1) && dx(x) <= 1),
            7 => every(m, |x, y| dx(x) + (y as i32 - cy).abs() == 2),
            8 => every(m, |x, y| dx(x) == 2 && y >= y0 && y <= y0 + 2),
            9 => every(m, |x, y| (y == y0 && x % 2 == 1) || (y == y0 + 2 && x % 2 == 0)),
            10 => every(m, |x, y| y as i32 == y0 as i32 + 3 - dx(x)),
            _ => every(m, |x, y| dx(x) == 3 && y >= y0 && y <= y0 + 3),
        };
        if (3..=most).contains(&where_.len()) {
            break;
        }
    }
    if !(3..=most).contains(&where_.len()) {
        where_ = every(m, |x, y| y == bottom && x % 2 == 1);
    }
    let style = rng.below(100);
    let few = where_.len() <= 8;
    where_
        .iter()
        .map(|&(x, y)| {
            let taped = match style {
                0..=44 => false,
                45..=64 => few,
                65..=84 => x % 2 == 0,
                _ => dx(x) <= 1,
            };
            (x, y, 1 + taped as u8)
        })
        .collect()
}

/// Bones waiting at the top of some columns. Returns how many.
fn place_bones(grid: &mut Grid, m: &Mask, rng: &mut Rng) -> u32 {
    const SETS: [&[usize]; 7] = [&[1, 5], &[2, 4], &[0, 6], &[1, 3, 5], &[0, 3, 6], &[3], &[2, 3, 4]];
    let set = SETS[rng.below(SETS.len() as u32) as usize];
    let mut placed = 0;
    for &x in set {
        if let Some(y) = (0..ROWS).find(|&y| m[y][x]) {
            grid[y][x] = b'o';
            placed += 1;
        }
    }
    placed
}

fn put(grid: &mut Grid, spots: &[Spot], one: u8, two: u8) {
    for &(x, y, layers) in spots {
        if grid[y][x] == b'.' {
            grid[y][x] = if layers >= 2 { two } else { one };
        }
    }
}

// ---- pieces and goals ---------------------------------------------------------------

fn palette(place: Place, rng: &mut Rng, finale: bool) -> Vec<u8> {
    let roll = if finale { 0 } else { rng.below(100) };
    let size = match place {
        Place::Night => {
            if roll < 65 {
                6
            } else {
                5
            }
        }
        // four kinds is a cascade party: an occasional treat, not a habit
        _ => match roll {
            0..=5 => 4,
            6..=81 => 5,
            _ => 6,
        },
    };
    // every place has its regulars
    let regulars: &[u8] = match place {
        Place::Yard => &[BALL, SQUIRREL],
        Place::Park => &[POODLE],
        Place::Lake => &[STICK],
        Place::Home => &[SOCK, TOY],
        _ => &[],
    };
    let mut rest: Vec<u8> = (0..rufus_core::KINDS as u8).filter(|k| !regulars.contains(k)).collect();
    rng.shuffle(&mut rest);
    let mut kinds = regulars.to_vec();
    kinds.extend(rest.into_iter().take(size - regulars.len()));
    kinds.sort();
    kinds
}

/// Which kinds a level asks for: the place's favourites first.
fn goal_kinds(place: Place, kinds: &[u8], rng: &mut Rng, n: usize) -> Vec<u8> {
    let favourites: &[u8] = match place {
        Place::Yard => &[BALL, SQUIRREL, CAT],
        Place::Park => &[POODLE, BALL, STICK],
        Place::Lake => &[STICK, BALL, TOY],
        Place::Home => &[SOCK, TOY, CAT],
        _ => &[],
    };
    let mut first: Vec<u8> = favourites.iter().copied().filter(|k| kinds.contains(k)).collect();
    let mut rest: Vec<u8> = kinds.iter().copied().filter(|k| !first.contains(k)).collect();
    // the place's own thing is usually asked for first
    if rng.chance(0.45) {
        rng.shuffle(&mut first);
    } else if first.len() > 1 {
        rng.shuffle(&mut first[1..]);
    }
    rng.shuffle(&mut rest);
    first.extend(rest);
    first.truncate(n);
    first
}

/// Nobody wants to be asked for 340 socks. A goal that would have to be
/// bigger than this to fill the level leaves it short, and the design is
/// thrown away for that instead.
const MOST_PIECES: f32 = 150.0;
const MOST_POINTS: f32 = 20_000.0;

fn nice_kind(x: f32) -> u32 {
    let x = x.clamp(8.0, MOST_PIECES);
    if x < 40.0 {
        (x / 2.0).round() as u32 * 2
    } else {
        (x / 5.0).round() as u32 * 5
    }
}

fn nice_score(x: f32) -> u32 {
    let x = x.clamp(1500.0, MOST_POINTS);
    if x < 4000.0 {
        (x / 250.0).round() as u32 * 250
    } else {
        (x / 500.0).round() as u32 * 500
    }
}

fn counted(g: &Goal) -> bool {
    matches!(g, Goal::Score(_) | Goal::Kind(..))
}

#[derive(Clone)]
struct Design {
    kinds: Vec<u8>,
    goals: Vec<Goal>,
    grid: Grid,
    recipe: Recipe,
}

/// One attempt at designing level `n` (counting from 0). The same `n` and
/// `variant` always give the same design.
fn design(n: usize, variant: u32, plain: bool, finale: bool) -> Design {
    let place = place_at(n);
    let mut rng = Rng::new(0x00D0_6000 + n as u64 * 1_000_003 + variant as u64 * 7_919);
    let recipe = if plain {
        Recipe::Kinds(2)
    } else if finale {
        // the very last level: one last job, like the last level of day one
        Recipe::Bones(Extra::Kinds2)
    } else {
        chapter_plan(n / CHAPTER_LEN)[n % CHAPTER_LEN]
    };
    let mask = if finale {
        block(0, 0)
    } else if plain {
        // boards that always play well; more than one, so that two fallbacks
        // near each other need not look alike
        match rng.below(6) {
            0 => block(0, 0),
            1 => block(1, 0),
            2 => block(0, 1),
            3 => profile(PROFILES[2]),
            4 => profile([1, 0, 0, 0, 0, 0, 0, 0, 0]),
            _ => profile([0, 0, 0, 0, 0, 0, 0, 0, 1]),
        }
    } else {
        shape(place, recipe.has_bones(), &mut rng)
    };
    let kinds = palette(place, &mut rng, finale);
    let mut grid = [[b' '; COLS]; ROWS];
    for y in 0..ROWS {
        for x in 0..COLS {
            if mask[y][x] {
                grid[y][x] = b'.';
            }
        }
    }
    let want = length(n) as f32;
    let score = || Goal::Score(nice_score(260.0 * want));
    let kind = |k: u8, share: f32| Goal::Kind(k, nice_kind(2.6 * want * share));
    let mut goals: Vec<Goal> = Vec::new();
    let extra = |goals: &mut Vec<Goal>, rng: &mut Rng, e: Extra| match e {
        Extra::None => {}
        Extra::Score => goals.push(score()),
        Extra::Kind => goals.push(kind(goal_kinds(place, &kinds, rng, 1)[0], 1.0)),
        Extra::Kinds2 => goals.extend(goal_kinds(place, &kinds, rng, 2).into_iter().map(|k| kind(k, 0.8))),
    };
    match recipe {
        Recipe::Score => goals.push(score()),
        Recipe::Kinds(k) => {
            let share = [1.0, 0.8, 0.7][k as usize - 1];
            goals.extend(goal_kinds(place, &kinds, &mut rng, k as usize).into_iter().map(|k| kind(k, share)));
        }
        Recipe::KindScore => {
            goals.push(kind(goal_kinds(place, &kinds, &mut rng, 1)[0], 1.0));
            goals.push(score());
        }
        Recipe::Mud(e) => {
            put(&mut grid, &mud_layout(&mask, &mut rng), b'm', b'M');
            goals.push(Goal::Mud);
            extra(&mut goals, &mut rng, e);
        }
        Recipe::Boxes(e) => {
            put(&mut grid, &box_layout(&mask, &mut rng, 14), b'b', b'B');
            goals.push(Goal::Boxes);
            extra(&mut goals, &mut rng, e);
        }
        Recipe::Bones(e) => {
            let waiting = place_bones(&mut grid, &mask, &mut rng);
            let more = if e == Extra::None { 1 + rng.below(3) } else { rng.below(2) };
            goals.push(Goal::Bones((waiting + more).clamp(3, 6)));
            extra(&mut goals, &mut rng, e);
        }
        Recipe::MudBoxes => {
            put(&mut grid, &box_layout(&mask, &mut rng, 8), b'b', b'B');
            put(&mut grid, &mud_layout(&mask, &mut rng), b'm', b'M');
            goals.push(Goal::Boxes);
            goals.push(Goal::Mud);
        }
        Recipe::BonesMud => {
            let waiting = place_bones(&mut grid, &mask, &mut rng);
            put(&mut grid, &mud_layout(&mask, &mut rng), b'm', b'M');
            goals.push(Goal::Mud);
            goals.push(Goal::Bones((waiting + rng.below(2)).clamp(3, 5)));
        }
    }
    Design { kinds, goals, grid, recipe }
}

fn place_at(n: usize) -> Place {
    CHAPTERS[n / CHAPTER_LEN].place
}

fn row_text(row: &[u8; COLS]) -> String {
    row.iter().map(|&b| b as char).collect()
}

/// The stand-ins play `&'static` level data, so a design being tried out is
/// simply never freed. This is a tool that runs once and exits.
fn leak(d: &Design) -> &'static LevelDef {
    let rows: [&'static str; ROWS] = std::array::from_fn(|y| &*Box::leak(row_text(&d.grid[y]).into_boxed_str()));
    Box::leak(Box::new(LevelDef {
        title: "",
        tip: "",
        moves: [LOTS; 3],
        kinds: Box::leak(d.kinds.clone().into_boxed_slice()),
        goals: Box::leak(d.goals.clone().into_boxed_slice()),
        rows,
        paws: [0, 1],
    }))
}

// ---- trying a design out ---------------------------------------------------------------

/// One casual game with moves to spare, noting when each goal was finished.
struct Trace {
    won: bool,
    moves: u32,
    done: Vec<u32>,
    shuffles: u32,
}

fn trace(def: &'static LevelDef, seed: u64) -> Trace {
    let cfg = BotCfg::CASUAL;
    let mut s = Session::of(def, 0, seed, LOTS);
    let mut rng = Rng::new(seed ^ 0xB07);
    let mut done: Vec<Option<u32>> = vec![None; def.goals.len()];
    let count = |steps: &[Step]| steps.iter().filter(|s| matches!(s, Step::Shuffle(_))).count() as u32;
    let mut shuffles = count(&s.board.settle());
    let mut moves = 0;
    loop {
        for (i, g) in s.goals().iter().enumerate() {
            if g.done() && done[i].is_none() {
                done[i] = Some(moves);
            }
        }
        if s.won() || s.moves_left == 0 {
            break;
        }
        if s.board.zoomies_ready() {
            s.board.zoomies();
            shuffles += count(&s.board.settle());
            continue;
        }
        let Some(m) = bot::choose(&s, &mut rng, &cfg) else { break };
        s.swap(m.a, m.b);
        moves += 1;
        shuffles += count(&s.board.settle());
    }
    Trace { won: s.won(), moves, done: done.into_iter().map(|d| d.unwrap_or(LOTS)).collect(), shuffles }
}

struct Measured {
    win: f32,
    shuffles: f32,
    /// Moves to finish: 25th, 50th, 75th and 90th percentile.
    t: [u32; 4],
    /// When each goal is typically finished.
    done: Vec<u32>,
}

fn measure(d: &Design, seed: u64, games: u64) -> Measured {
    let def = leak(d);
    let traces: Vec<Trace> = (0..games).map(|s| trace(def, seed.wrapping_add(s * 13 + 1))).collect();
    let sorted = |f: &dyn Fn(&Trace) -> u32| {
        let mut v: Vec<u32> = traces.iter().map(f).collect();
        v.sort();
        v
    };
    let total = sorted(&|t| if t.won { t.moves } else { LOTS });
    Measured {
        win: traces.iter().filter(|t| t.won).count() as f32 / games as f32,
        shuffles: traces.iter().map(|t| t.shuffles).sum::<u32>() as f32 / games as f32,
        t: [pct(&total, 0.25), pct(&total, 0.5), pct(&total, 0.75), pct(&total, 0.9)],
        done: (0..d.goals.len()).map(|i| pct(&sorted(&|t| t.done[i]), 0.5)).collect(),
    }
}

/// Scales the counted goals (points, pieces of a kind) until the level takes
/// about `want` moves, or, where an obstacle sets the pace, until they come
/// in a little before the obstacle does.
fn calibrate(d: &mut Design, want: u32, seed: u64) {
    let counted_goals = d.goals.iter().filter(|g| counted(g)).count();
    if counted_goals == 0 {
        return;
    }
    for _ in 0..7 {
        let m = measure(d, seed, 64);
        let pace = d.goals.iter().zip(&m.done).filter(|(g, _)| !counted(g)).map(|(_, &t)| t).max().unwrap_or(0) as f32;
        let aim = if pace >= want as f32 * 0.9 { pace * 0.9 } else { want as f32 * [1.0, 0.94, 0.9][counted_goals.min(3) - 1] };
        let mut settled = true;
        for (g, &t) in d.goals.iter_mut().zip(&m.done) {
            let r = (aim / t.max(1) as f32).clamp(0.55, 1.8);
            let before = *g;
            match g {
                Goal::Score(n) => *n = nice_score(*n as f32 * r),
                Goal::Kind(_, n) => *n = nice_kind(*n as f32 * r),
                _ => continue,
            }
            if (r - 1.0).abs() > 0.06 && *g != before {
                settled = false;
            }
        }
        if settled {
            break;
        }
    }
}

/// Why a design is not worth keeping, if it isn't.
fn fault(d: &Design, m: &Measured, want: u32) -> Option<&'static str> {
    let t50 = m.t[1] as f32;
    let paced_by_goals = d.goals.iter().all(counted);
    if m.win < 0.985 {
        return Some("can jam");
    }
    if m.shuffles > 0.10 {
        return Some("needs reshuffles");
    }
    if !(12..=30).contains(&m.t[1]) {
        return Some("too short or too long");
    }
    if paced_by_goals && (t50 - want as f32).abs() > 3.0 {
        return Some("missed its length");
    }
    // bones are luckier than everything else: allow them a wider spread
    let spread = if d.recipe.has_bones() { 1.52 } else { 1.42 };
    if m.t[2] as f32 > t50 * spread + 0.5 {
        return Some("swings too much");
    }
    if m.t[3] as f32 > t50 * 1.9 {
        return Some("long tail");
    }
    for (g, &t) in d.goals.iter().zip(&m.done) {
        // finished long before the level is: decoration, not a goal
        if (t as f32) < t50 * if counted(g) { 0.45 } else { 0.3 } {
            return Some("a goal is only decoration");
        }
    }
    None
}

struct Made {
    design: Design,
    tuned: Tuned,
    /// Designs drawn, counting the one that was kept.
    tries: u32,
    /// Whether it took so many that this is the plain fallback.
    plain: bool,
    t50: u32,
}

static FAULTS: Mutex<BTreeMap<&'static str, u32>> = Mutex::new(BTreeMap::new());

fn make(n: usize, games: u64, from: u32, finale: bool) -> Made {
    let want = length(n);
    for variant in from.. {
        let seed = 0xCA11_0000 + n as u64 * 977 + variant as u64 * 131;
        // After enough failed attempts, stop being clever: a full board and
        // two kinds to collect always plays well.
        let plain = variant >= from + 14;
        let mut d = design(n, variant, plain, finale);
        calibrate(&mut d, want, seed);
        let m = measure(&d, seed ^ 0x55AA, 160);
        let last_resort = variant >= from + 40;
        if let Some(why) = fault(&d, &m, want) {
            *FAULTS.lock().unwrap().entry(why).or_default() += 1;
            if !last_resort {
                continue;
            }
        }
        let tuned = tune::tune_level(leak(&d), n, games);
        // a Hard that is over in a blink, or an Easy that is a slog
        if (tuned.moves[2] < 9 || tuned.moves[0] > 60) && !last_resort {
            *FAULTS.lock().unwrap().entry("budget out of range").or_default() += 1;
            continue;
        }
        return Made { design: d, tuned, tries: variant + 1, plain, t50: m.t[1] };
    }
    unreachable!()
}

// ---- words ------------------------------------------------------------------------------

/// What a level is about, for naming it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Subject {
    Kind(u8),
    Mud,
    Box,
    Bone,
    Rufus,
}

fn subject(d: &Design, rng: &mut Rng) -> Subject {
    let of = |g: &Goal| match *g {
        Goal::Kind(k, _) => Subject::Kind(k),
        Goal::Mud => Subject::Mud,
        Goal::Boxes => Subject::Box,
        Goal::Bones(_) => Subject::Bone,
        Goal::Score(_) => Subject::Rufus,
    };
    // usually the first goal, sometimes one of the others
    if d.goals.len() > 1 && rng.chance(0.35) {
        of(&d.goals[1 + rng.below(d.goals.len() as u32 - 1) as usize])
    } else {
        of(&d.goals[0])
    }
}

/// Singular and plural.
fn things(s: Subject) -> &'static [(&'static str, &'static str)] {
    match s {
        Subject::Kind(BALL) => &[("Ball", "Balls"), ("Tennis Ball", "Tennis Balls"), ("Bouncy Ball", "Bouncy Balls"), ("Lost Ball", "Lost Balls")],
        Subject::Kind(SQUIRREL) => &[("Squirrel", "Squirrels"), ("Fence Squirrel", "Fence Squirrels"), ("Tree Squirrel", "Tree Squirrels"), ("Bushy Tail", "Bushy Tails")],
        Subject::Kind(CAT) => &[("Cat", "Cats"), ("Kitten", "Kittens"), ("Neighbour Cat", "Neighbour Cats"), ("Whisker", "Whiskers")],
        Subject::Kind(SOCK) => &[("Sock", "Socks"), ("Left Sock", "Left Socks"), ("Gym Sock", "Gym Socks"), ("Slipper", "Slippers")],
        Subject::Kind(POODLE) => &[("Poodle", "Poodles"), ("Fancy Dog", "Fancy Dogs"), ("New Friend", "New Friends"), ("Haircut", "Haircuts")],
        Subject::Kind(STICK) => &[("Stick", "Sticks"), ("Big Stick", "Big Sticks"), ("Branch", "Branches"), ("Log", "Logs")],
        Subject::Kind(_) => &[("Squeaky Toy", "Squeaky Toys"), ("Squeaker", "Squeakers"), ("Chew Toy", "Chew Toys"), ("Plush Duck", "Plush Ducks")],
        Subject::Mud => &[("Puddle", "Puddles"), ("Paw Print", "Paw Prints"), ("Mud Pie", "Mud Pies"), ("Bath", "Baths"), ("Towel", "Towels"), ("Carpet", "Carpets")],
        Subject::Box => &[("Parcel", "Parcels"), ("Box", "Boxes"), ("Delivery", "Deliveries"), ("Doorbell", "Doorbells"), ("Mailman", "Mailmen")],
        Subject::Bone => &[("Bone", "Bones"), ("Buried Bone", "Buried Bones"), ("Treat", "Treats"), ("Biscuit", "Biscuits")],
        Subject::Rufus => &[("Nap", "Naps"), ("Snack", "Snacks"), ("Walk", "Walks"), ("Belly Rub", "Belly Rubs"), ("Zoomie", "Zoomies"), ("Tail Wag", "Tail Wags"), ("Bark", "Barks"), ("Sniff", "Sniffs")],
    }
}

/// `{s}` is the singular, `{p}` the plural.
const TITLES: [&str; 40] = [
    "Operation: {s}",
    "The {s} Situation",
    "{s} Emergency",
    "Suspicious {s}",
    "The Great {s} Chase",
    "Return of the {s}",
    "{s} O'Clock",
    "Unauthorised {s}",
    "Too Many {p}",
    "No {s} Left Behind",
    "One More {s}",
    "{s} Inspector",
    "A Short History of {p}",
    "{s} Appreciation Hour",
    "All {p} Are Mine",
    "The {s} Did It",
    "Regarding the {s}",
    "{s} Patrol",
    "The Last {s}",
    "{p}, Probably",
    "Not Without My {s}",
    "{s} Protocol",
    "The {s} Problem",
    "Extra {p}",
    "{s}: The Sequel",
    "A {s} Too Far",
    "The {s} Question",
    "{p} Everywhere",
    "Where Is the {s}",
    "Midnight {s}",
    "The {s} Incident",
    "{s} Season",
    "Dances With {p}",
    "{s} Diplomacy",
    "The {s} Heist",
    "Emergency {s}",
    "{s} Day",
    "The Forbidden {s}",
    "Advanced {s} Studies",
    "{s} Management",
];

fn tips(s: Subject) -> &'static [&'static str] {
    match s {
        Subject::Kind(BALL) => &[
            "The ball is life. This is not a metaphor.",
            "Rufus has one ball. He would like all of the balls.",
            "Throw it. No, don't take it. Just throw it.",
            "It rolled under the couch. This is now an emergency.",
            "Somewhere a tennis ball is not in Rufus's mouth. Fix this.",
            "A ball in the mouth is worth two in the bush. Get the two in the bush.",
            "He can hear a ball bounce three streets away.",
            "It's slobbery. That's how you know it's his.",
        ],
        Subject::Kind(SQUIRREL) => &[
            "The squirrel is back. It is doing it on purpose.",
            "Rufus has never caught one. He likes his chances today.",
            "They live in the tree. The tree has not been dealt with either.",
            "It flicked its tail. That's squirrel for something rude.",
            "Rufus only wants to talk. Loudly. At speed.",
            "Every squirrel is the same squirrel. Rufus is sure of it.",
            "It's on the fence. THE fence. HIS fence.",
            "The squirrels have meetings. Rufus is never invited.",
        ],
        Subject::Kind(CAT) => &[
            "The cat is on the high shelf. She is always on the high shelf.",
            "Rufus respects the cat. From here. Where it's safe.",
            "She blinked slowly. Rufus doesn't know what it means and he's worried.",
            "The cat has a bed. The cat sleeps in Rufus's bed.",
            "Approach the cat sideways. No eye contact. Good luck.",
            "She knocked it off the table and looked at him. He's being framed.",
            "The cat was here first. She mentions it often.",
            "One paw. That's all it took. Rufus remembers.",
        ],
        Subject::Kind(SOCK) => &[
            "A sock alone is a sock for Rufus. Those are the rules.",
            "It smells like you. That's why it has to be eaten.",
            "He isn't stealing it. He's relocating it.",
            "The left ones taste better. Nobody knows why.",
            "Clean laundry is just socks that haven't met Rufus yet.",
            "Trade offer: you get the sock back, he gets a treat.",
            "There's a pile behind the couch. Nobody is to look behind the couch.",
            "He walked past with it very slowly so you'd notice.",
        ],
        Subject::Kind(POODLE) => &[
            "She has a haircut. Rufus has questions about the haircut.",
            "The poodle said hello. Rufus has forgotten how legs work.",
            "She's fancy. Rufus is muddy. It could work.",
            "Play bow first. Then chaos. There's an etiquette.",
            "Rufus would like to be friends. Rufus would like that with everyone.",
            "She can do tricks. Rufus can sit, sometimes.",
            "They've met forty times. Each one is the first.",
            "Her name is Duchess. His name is mud. Usually literally.",
        ],
        Subject::Kind(STICK) => &[
            "It's a good stick. It might be the best stick.",
            "The stick does not fit through the door. Rufus will keep trying.",
            "Bigger. No, bigger than that. That one, the tree.",
            "He found it, so it's his. Stick law.",
            "A stick is a ball that grows on trees.",
            "It was a stick. Now it's four smaller sticks. Value added.",
            "Carry it sideways and hit every shin on the path.",
            "He'll drop it for a better stick. There's never a better stick.",
        ],
        Subject::Kind(_) => &[
            "The squeaker must be found and it must be stopped.",
            "It squeaks, so it's alive, so it has to be shaken.",
            "This one's his favourite. They're all his favourite.",
            "The stuffing comes out. It doesn't go back in. Rufus checked.",
            "It has no ears left. It's still perfect.",
            "3 a.m. is squeaky toy time. Rufus doesn't make the rules.",
            "He brings it to guests. Not to give. Just to show.",
            "New toy smell lasts about nine seconds.",
        ],
        Subject::Mud => &[
            "Clean dogs are just dogs who haven't tried.",
            "The mud was right there. What was he supposed to do?",
            "Paw prints lead to the couch. They do not lead away.",
            "It's not dirt, it's outdoors he brought in for later.",
            "Dark mud takes two goes. Rufus put in the effort.",
            "He found the only puddle for a mile. It's a gift.",
            "The towel is by the door. Rufus is already past the door.",
            "Shake first, ask questions never.",
            "White carpet was always a risky choice.",
            "He rolled in it. Then he rolled in it properly.",
        ],
        Subject::Box => &[
            "Whatever's inside, the box is the good part.",
            "The doorbell rang. Everyone stay calm. Rufus will not be.",
            "It says FRAGILE. Rufus can't read.",
            "Taped boxes take two hits. Rufus brought both.",
            "The mailman keeps coming back. Some people never learn.",
            "Every parcel might be treats. None have been. Hope remains.",
            "He signed for it. With teeth.",
            "Delivered at noon. Cardboard confetti by one.",
            "Somebody ordered something. Rufus is handling it.",
            "The van is outside. THE VAN IS OUTSIDE.",
        ],
        Subject::Bone => &[
            "Bones go down. Walk them to the bottom of the board.",
            "He buried it somewhere safe. He can't remember where.",
            "A bone is forever. Or until dinner.",
            "Clear what's underneath and the bone drops.",
            "The good bones are always at the top. Typical.",
            "Rufus doesn't share bones. He'll show you it, though.",
            "It was under the cushion the whole time.",
            "Dig first. Think later. Bone eventually.",
        ],
        Subject::Rufus => &[
            "No reason. Rufus just feels like running.",
            "Points are treats you can't eat. Get them anyway.",
            "Big matches fill the zoomies meter faster.",
            "Four in a line. You know what happens.",
            "Rufus believes in you. Rufus believes in everyone.",
            "He's been good all day. Since about four minutes ago.",
            "Somebody said 'walk'. Nobody can take it back now.",
            "Dinner is in two hours. Rufus has started the countdown.",
            "Tail's going. Good things are about to happen.",
            "Rufus has a plan. The plan is zoomies.",
            "It's a perfectly normal day. Rufus is thrilled.",
            "He heard the treat jar. He was asleep. He's here.",
        ],
    }
}

// ---- writing it down --------------------------------------------------------------------------

const KIND_NAMES: [&str; 7] = ["BALL", "SQUIRREL", "CAT", "SOCK", "POODLE", "TOY", "STICK"];

fn goal_text(g: &Goal) -> String {
    match *g {
        Goal::Score(n) => format!("Score({n})"),
        Goal::Kind(k, n) => format!("Kind({}, {n})", KIND_NAMES[k as usize]),
        Goal::Mud => "Mud".into(),
        Goal::Bones(n) => format!("Bones({n})"),
        Goal::Boxes => "Boxes".into(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let count: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(1000);
    let games: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let out = args.get(2).cloned().unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/src/later.rs").to_string());
    assert!((HAND_MADE + count).div_ceil(CHAPTER_LEN) <= CHAPTERS.len(), "not enough chapters named for {count} more levels");
    let finale = |n: usize| count == 1000 && n + 1 == HAND_MADE + count;

    let progress = AtomicUsize::new(0);
    let step = |_: ()| {
        let done = progress.fetch_add(1, Ordering::Relaxed) + 1;
        if done % 50 == 0 {
            eprintln!("  {done} designed");
        }
    };
    let mut made: Vec<Made> = par_map(count, |j| {
        let m = make(HAND_MADE + j, games, 0, finale(HAND_MADE + j));
        step(());
        m
    });

    // The same board twice within a few levels reads as a repeat: redo the later one.
    for round in 0..30 {
        let repeats: Vec<usize> = (1..count).filter(|&j| (j.saturating_sub(4)..j).any(|k| made[k].design.grid == made[j].design.grid)).collect();
        if repeats.is_empty() {
            break;
        }
        eprintln!("  round {round}: {} levels repeat a recent board, redoing them", repeats.len());
        let redone = par_map(repeats.len(), |r| {
            let j = repeats[r];
            make(HAND_MADE + j, games, made[j].tries, finale(HAND_MADE + j))
        });
        for (j, m) in repeats.into_iter().zip(redone) {
            made[j] = m;
        }
    }

    // Names and one-liners: no title twice, and no one-liner again until the
    // others on its subject have had their turn.
    let mut used: HashSet<String> = LEVELS.iter().take(HAND_MADE).map(|l| l.title.to_string()).collect();
    let mut tip_turn: BTreeMap<Subject, usize> = BTreeMap::new();
    let mut words: Vec<(String, &'static str)> = Vec::new();
    for (j, m) in made.iter().enumerate() {
        let n = HAND_MADE + j;
        let mut rng = Rng::new(0x7171 + n as u64 * 31);
        let about = subject(&m.design, &mut rng);
        let (title, tip) = if j == 0 {
            ("Good Morning. Again.".to_string(), "Rufus slept eleven hours. He is ready to do all of it again.")
        } else if finale(n) {
            ("Goodnight, Rufus".to_string(), "One last job, then bed. He means it this time.")
        } else {
            let title = loop {
                let (s, p) = rng.pick(things(about));
                let t = rng.pick(&TITLES).replace("{s}", s).replace("{p}", p);
                if t.chars().count() <= 24 && !used.contains(&t) {
                    break t;
                }
            };
            let pool = tips(about);
            let turn = tip_turn.entry(about).or_default();
            let has = |ch: u8| m.design.grid.iter().flatten().any(|&b| b == ch);
            let tip = loop {
                // a stride that visits every line before repeating any
                let tip = pool[(*turn * 3 + 1) % pool.len()];
                *turn += 1;
                // the two that explain a rule only where the rule applies
                if (tip.starts_with("Dark mud") && !has(b'M')) || (tip.starts_with("Taped boxes") && !has(b'B')) {
                    continue;
                }
                break tip;
            };
            (title, tip)
        };
        used.insert(title.clone());
        words.push((title, tip));
    }

    let mut text = String::new();
    writeln!(text, "//! Levels {} to {}: every day after the first.", HAND_MADE + 1, HAND_MADE + count).unwrap();
    text.push_str(
        "//!\n\
         //! Written by `cargo run -p rufus-core --release --example mklevels`, which\n\
         //! designs each board, plays it a few hundred times and sizes its goals. Do\n\
         //! not edit the boards by hand: change the designer and run it again. The\n\
         //! move budgets and paw thresholds are kept up to date by\n\
         //! `tools/apply_tune.py`, the same as the hand-made levels'.\n\n\
         use crate::level::Goal::*;\n\
         use crate::level::{LevelDef, BALL, CAT, POODLE, SOCK, SQUIRREL, STICK, TOY};\n\n",
    );
    writeln!(text, "pub const COUNT: usize = {count};\n").unwrap();
    text.push_str("#[rustfmt::skip]\npub(crate) const LATER: [LevelDef; COUNT] = [\n");
    for (m, (title, tip)) in made.iter().zip(&words) {
        let d = &m.design;
        let kinds: Vec<&str> = d.kinds.iter().map(|&k| KIND_NAMES[k as usize]).collect();
        let goals: Vec<String> = d.goals.iter().map(goal_text).collect();
        let rows: Vec<String> = d.grid.iter().map(|r| format!("{:?}", row_text(r))).collect();
        let [easy, normal, hard] = m.tuned.moves;
        let [two, three] = m.tuned.paws;
        writeln!(
            text,
            "    LevelDef {{ title: {title:?}, tip: {tip:?}, moves: [{easy}, {normal}, {hard}], kinds: &[{}], goals: &[{}], rows: [{}], paws: [{two}, {three}] }},",
            kinds.join(", "),
            goals.join(", "),
            rows.join(", "),
        )
        .unwrap();
    }
    text.push_str("];\n");
    std::fs::write(&out, text).expect("could not write the levels");

    // What came out.
    let mut recipes: BTreeMap<String, u32> = BTreeMap::new();
    let mut sizes = [0u32; 7];
    let mut shapes: HashSet<Vec<String>> = HashSet::new();
    let mut boards: HashSet<Grid> = HashSet::new();
    for m in &made {
        *recipes.entry(format!("{:?}", m.design.recipe)).or_default() += 1;
        sizes[m.design.kinds.len()] += 1;
        shapes.insert(m.design.grid.iter().map(|r| r.iter().map(|&b| if b == b' ' { ' ' } else { '.' }).collect()).collect());
        boards.insert(m.design.grid);
    }
    let tries: Vec<u32> = made.iter().map(|m| m.tries).collect();
    let mut t50: Vec<u32> = made.iter().map(|m| m.t50).collect();
    t50.sort();
    eprintln!("wrote {count} levels to {out}");
    eprintln!("  recipes: {recipes:?}");
    eprintln!("  kinds in play: 4 on {}, 5 on {}, 6 on {}", sizes[4], sizes[5], sizes[6]);
    eprintln!("  {} different board shapes, {} different boards counting obstacles", shapes.len(), boards.len());
    eprintln!(
        "  attempts per level: mean {:.1}, most {}, {} levels fell back to a plain board",
        tries.iter().sum::<u32>() as f32 / count as f32,
        tries.iter().max().unwrap(),
        made.iter().filter(|m| m.plain).count()
    );
    eprintln!("  thrown away because: {:?}", FAULTS.lock().unwrap());
    eprintln!("  moves the casual stand-in needs: {} to {}, median {}", t50[0], t50[count - 1], t50[count / 2]);
    for k in 0..3 {
        let mut b: Vec<u32> = made.iter().map(|m| m.tuned.moves[k]).collect();
        b.sort();
        eprintln!("  {:?} budgets: {} to {}, median {}", Difficulty::ALL[k], b[0], b[count - 1], b[count / 2]);
    }
}
