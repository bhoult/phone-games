//! The rules of the board, with no rendering attached.
//!
//! The board never animates anything. Every change it makes is described in a
//! report ([`ClearReport`], [`FallReport`], ...) carrying piece ids and times,
//! and the game layer plays those reports back. The loop is:
//!
//! ```text
//! swap() / zoomies()  ->  advance() until it returns Step::Idle
//! ```
//!
//! Times are milliseconds from the start of a clear step. The board is the
//! timing authority for chain reactions: a blast that reaches a special piece
//! sets that piece off at the moment it is reached.

use crate::rng::Rng;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub const COLS: usize = 7;
pub const ROWS: usize = 9;
pub const N: usize = COLS * ROWS;
/// How many kinds of piece there are in all.
pub const KINDS: usize = 7;
/// How many of them one board can use. More than this and matches dry up.
pub const MAX_IN_PLAY: usize = 6;

/// Charge needed before Rufus can be sent on a lap of the board.
pub const ZOOMIES_FULL: u32 = 50;

// Clear choreography, in ms.
pub const RUN_LEAD: u32 = 90;
pub const RUN_STEP: u32 = 55;
const BOMB_LEAD: u32 = 110;
const BOMB_RING: u32 = 45;
const FETCH_LEAD: u32 = 260;
const FETCH_STEP: u32 = 65;
const ALL_LEAD: u32 = 300;
const ALL_RING: u32 = 55;
const MORPH_LEAD: u32 = 200;
const MORPH_STEP: u32 = 55;
const TAIL: u32 = 230;

// Scoring.
const PTS_MATCH: u32 = 10;
const PTS_BLAST: u32 = 15;
const PTS_LINE: u32 = 40;
const PTS_BOMB: u32 = 80;
const PTS_SUPER: u32 = 150;
const PTS_BOX: u32 = 30;
const PTS_MUD: u32 = 20;
pub const PTS_BONE: u32 = 200;
pub const PTS_BONUS: u32 = 100;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Pos {
    pub x: u8,
    pub y: u8,
}

impl Pos {
    pub const fn new(x: usize, y: usize) -> Pos {
        Pos { x: x as u8, y: y as u8 }
    }
    pub const fn i(self) -> usize {
        self.y as usize * COLS + self.x as usize
    }
    pub const fn from_i(i: usize) -> Pos {
        Pos::new(i % COLS, i / COLS)
    }
    pub fn adjacent(self, o: Pos) -> bool {
        (self.x as i32 - o.x as i32).abs() + (self.y as i32 - o.y as i32).abs() == 1
    }
    pub fn offset(self, dx: i32, dy: i32) -> Option<Pos> {
        let (x, y) = (self.x as i32 + dx, self.y as i32 + dy);
        (x >= 0 && y >= 0 && x < COLS as i32 && y < ROWS as i32).then(|| Pos::new(x as usize, y as usize))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Special {
    None,
    /// Zoomies: Rufus tears along the whole row.
    LineH,
    /// Zoomies, but up and down.
    LineV,
    /// Big Woof: flattens everything nearby.
    Bomb,
}

impl Special {
    pub fn is_line(self) -> bool {
        matches!(self, Special::LineH | Special::LineV)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Item {
    Normal { kind: u8, special: Special },
    /// Golden Rufus: fetches every piece of one kind.
    Super,
    /// A bone that has to be walked down to the bottom of the board.
    Bone,
    /// A delivery box. Needs matches next to it (or a blast) to open.
    Box { hp: u8 },
}

impl Item {
    pub const fn plain(kind: u8) -> Item {
        Item::Normal { kind, special: Special::None }
    }
    pub fn kind(self) -> Option<u8> {
        match self {
            Item::Normal { kind, .. } => Some(kind),
            _ => None,
        }
    }
    pub fn special(self) -> Special {
        match self {
            Item::Normal { special, .. } => special,
            _ => Special::None,
        }
    }
    /// A piece that does something when it goes off.
    pub fn is_power(self) -> bool {
        matches!(self, Item::Super) || self.special() != Special::None
    }
    pub fn is_plain(self) -> bool {
        matches!(self, Item::Normal { special: Special::None, .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub id: u32,
    pub item: Item,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Cell {
    /// Part of the board at all? Levels cut shapes out of the 7x9 grid.
    pub exists: bool,
    /// Layers of muddy paw prints still to clean off this tile.
    pub mud: u8,
    pub piece: Option<Piece>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    Match,
    Combo,
    Row,
    Col,
    Bomb,
    Fetch,
    All,
    Fire,
}

#[derive(Clone, Copy, Debug)]
pub struct Pop {
    pub id: u32,
    pub pos: Pos,
    pub item: Item,
    pub t: u32,
    pub cause: Cause,
    /// Matched pieces that are merging into a new special slide to this cell.
    pub into: Option<Pos>,
    /// Index into `ClearReport::blasts` of the blast that got it, if any.
    pub blast: Option<u16>,
    pub points: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Made {
    pub id: u32,
    pub pos: Pos,
    pub item: Item,
    pub t: u32,
}

/// An existing piece that changed what it is (a plain piece turned special).
#[derive(Clone, Copy, Debug)]
pub struct Morph {
    pub id: u32,
    pub pos: Pos,
    pub item: Item,
    pub t: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlastKind {
    /// Rufus runs the row, entering from the left (`dir` 1) or right (-1).
    Row { dir: i8 },
    Col { dir: i8 },
    Bomb { big: bool },
    Fetch { kind: u8 },
    Everything,
}

#[derive(Clone, Copy, Debug)]
pub struct Blast {
    pub kind: BlastKind,
    pub at: Pos,
    pub t: u32,
    /// Piece kind that set it off, for tinting. `None` for Golden Rufus etc.
    pub tint: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
pub struct BoxHit {
    pub id: u32,
    pub pos: Pos,
    pub t: u32,
    pub hp_left: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct MudHit {
    pub pos: Pos,
    pub t: u32,
    pub left: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct GroupInfo {
    pub kind: u8,
    pub size: u8,
    /// Centre of the group, in cell units.
    pub cx: f32,
    pub cy: f32,
    pub points: u32,
    pub made: Option<Item>,
}

#[derive(Clone, Debug, Default)]
pub struct ClearReport {
    /// 1 for the clear the player's move caused, 2+ for chain reactions.
    pub cascade: u32,
    pub pops: Vec<Pop>,
    pub made: Vec<Made>,
    pub morphs: Vec<Morph>,
    pub blasts: Vec<Blast>,
    pub box_hits: Vec<BoxHit>,
    pub mud_hits: Vec<MudHit>,
    pub groups: Vec<GroupInfo>,
    pub gained: u32,
    /// How long the whole step takes to play out.
    pub duration: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Fall {
    pub id: u32,
    pub item: Item,
    /// Row it starts from. Negative-ish rows above the column top are new
    /// pieces queueing to drop in.
    pub from_y: i32,
    pub to: Pos,
    pub spawned: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FallReport {
    pub falls: Vec<Fall>,
}

#[derive(Clone, Copy, Debug)]
pub struct Shuffled {
    pub id: u32,
    pub item: Item,
    pub from: Pos,
    pub to: Pos,
}

#[derive(Clone, Debug)]
pub enum Step {
    Clear(ClearReport),
    Fall(FallReport),
    /// Bones that reached the bottom and were delivered.
    Bones(Vec<(u32, Pos)>),
    /// No moves were left, so everything got mixed up.
    Shuffle(Vec<Shuffled>),
    /// Settled. Ready for the next move.
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub a: Pos,
    pub b: Pos,
}

/// Running totals the level goals are checked against.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub kinds: [u32; KINDS],
    pub mud: u32,
    pub boxes: u32,
    pub bones: u32,
    pub specials: u32,
    pub best_cascade: u32,
}

#[derive(Clone, Copy, Debug)]
enum Trigger {
    Row(Pos),
    Col(Pos),
    /// Set off whatever is sitting on this cell.
    Fire(Pos),
}

struct Group {
    kind: u8,
    cells: Vec<Pos>,
    max_run: u8,
    has_h: bool,
    has_v: bool,
    cross: Option<Pos>,
}

/// Scratch state for one clear step.
struct Ctx {
    heap: BinaryHeap<Reverse<(u32, u32, u8, u8, u16)>>,
    seq: u32,
    protected: [bool; N],
    box_adj_done: [bool; N],
    rep: ClearReport,
    mult: u32,
}

const CAUSES: [Cause; 8] = [
    Cause::Match,
    Cause::Combo,
    Cause::Row,
    Cause::Col,
    Cause::Bomb,
    Cause::Fetch,
    Cause::All,
    Cause::Fire,
];

impl Ctx {
    fn new(mult: u32) -> Ctx {
        Ctx {
            heap: BinaryHeap::new(),
            seq: 0,
            protected: [false; N],
            box_adj_done: [false; N],
            rep: ClearReport { cascade: mult, ..Default::default() },
            mult,
        }
    }
    fn hit(&mut self, t: u32, pos: Pos, cause: Cause, fx: u16) {
        let c = CAUSES.iter().position(|&c| c == cause).unwrap() as u8;
        self.heap.push(Reverse((t, self.seq, pos.i() as u8, c, fx)));
        self.seq += 1;
    }
    fn fx(&mut self, kind: BlastKind, at: Pos, t: u32, tint: Option<u8>) -> u16 {
        self.rep.blasts.push(Blast { kind, at, t, tint });
        (self.rep.blasts.len() - 1) as u16
    }
}

#[derive(Clone)]
pub struct Board {
    cells: [Cell; N],
    /// Which piece kinds are in play on this board.
    palette: Vec<u8>,
    pub rng: Rng,
    next_id: u32,
    /// Depth of the chain reaction currently playing out (0 when idle).
    pub cascade: u32,
    pub score: u32,
    pub tally: Tally,
    /// Zoomies meter, 0..=ZOOMIES_FULL.
    pub charge: u32,
    /// Bones still waiting to drop in from the top.
    pub bones_queued: u32,
    /// Most bones allowed on the board at once.
    pub bones_cap: u32,
    last_swap: Option<(Pos, Pos)>,
    swap_horizontal: bool,
    combo: Option<(Pos, Pos)>,
    triggers: Vec<(u32, Trigger)>,
    no_charge: bool,
}

impl Board {
    /// Builds a board from nine rows of seven characters:
    ///
    /// ```text
    /// ' ' or '#'  no tile          '.'  random piece
    /// 'm' / 'M'   1 / 2 layers of mud under a random piece
    /// 'b' / 'B'   box needing 1 / 2 hits        'o'  bone
    /// '0'..'5'    a piece of that kind (tests, tutorials)
    /// '-' '|' '*' kind-0 row / column / bomb special,  '@' Golden Rufus
    /// ```
    pub fn from_rows(rows: &[&str], palette: &[u8], seed: u64) -> Board {
        assert!(rows.len() <= ROWS, "too many rows");
        assert!((3..=MAX_IN_PLAY).contains(&palette.len()), "need 3 to 6 kinds in play");
        assert!(palette.iter().all(|&k| (k as usize) < KINDS));
        let mut b = Board {
            cells: [Cell::default(); N],
            palette: palette.to_vec(),
            rng: Rng::new(seed),
            next_id: 1,
            cascade: 0,
            score: 0,
            tally: Tally::default(),
            charge: 0,
            bones_queued: 0,
            bones_cap: 0,
            last_swap: None,
            swap_horizontal: true,
            combo: None,
            triggers: Vec::new(),
            no_charge: false,
        };
        let mut random: Vec<usize> = Vec::new();
        for (y, row) in rows.iter().enumerate() {
            assert!(row.chars().count() <= COLS, "row {y} too wide: {row:?}");
            for (x, ch) in row.chars().enumerate() {
                let i = Pos::new(x, y).i();
                let item = match ch {
                    ' ' | '#' => continue,
                    '.' => None,
                    'm' => {
                        b.cells[i].mud = 1;
                        None
                    }
                    'M' => {
                        b.cells[i].mud = 2;
                        None
                    }
                    'b' => Some(Item::Box { hp: 1 }),
                    'B' => Some(Item::Box { hp: 2 }),
                    'o' => Some(Item::Bone),
                    '0'..='5' => Some(Item::plain(ch as u8 - b'0')),
                    '-' => Some(Item::Normal { kind: 0, special: Special::LineH }),
                    '|' => Some(Item::Normal { kind: 0, special: Special::LineV }),
                    '*' => Some(Item::Normal { kind: 0, special: Special::Bomb }),
                    '@' => Some(Item::Super),
                    other => panic!("unknown layout character {other:?}"),
                };
                b.cells[i].exists = true;
                match item {
                    Some(item) => b.cells[i].piece = Some(b.new_piece(item)),
                    None => random.push(i),
                }
            }
        }
        b.fill_random(&random);
        b
    }

    pub fn palette(&self) -> &[u8] {
        &self.palette
    }

    fn random_kind(&mut self) -> u8 {
        self.palette[self.rng.below(self.palette.len() as u32) as usize]
    }

    fn new_piece(&mut self, item: Item) -> Piece {
        let id = self.next_id;
        self.next_id += 1;
        Piece { id, item }
    }

    /// Fills the given cells with random pieces: no ready-made matches, and
    /// at least one move available.
    fn fill_random(&mut self, slots: &[usize]) {
        if slots.is_empty() {
            return;
        }
        let ids: Vec<u32> = slots.iter().map(|_| self.new_piece(Item::plain(0)).id).collect();
        for attempt in 0..200 {
            for &i in slots {
                self.cells[i].piece = None;
            }
            for (&i, &id) in slots.iter().zip(&ids) {
                let p = Pos::from_i(i);
                let n = self.palette.len();
                let start = self.rng.below(n as u32) as usize;
                for k in 0..n {
                    let kind = self.palette[(start + k) % n];
                    self.cells[i].piece = Some(Piece { id, item: Item::plain(kind) });
                    if !self.run_through(p) {
                        break;
                    }
                }
            }
            if (self.find_groups().is_empty() && !self.find_moves().is_empty()) || attempt == 199 {
                break;
            }
        }
    }

    // ---- reading the board ----------------------------------------------

    pub fn cell(&self, p: Pos) -> &Cell {
        &self.cells[p.i()]
    }

    pub fn get(&self, x: usize, y: usize) -> &Cell {
        &self.cells[y * COLS + x]
    }

    pub fn exists(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < COLS as i32 && y < ROWS as i32 && self.cells[y as usize * COLS + x as usize].exists
    }

    pub fn pieces(&self) -> impl Iterator<Item = (Pos, Piece)> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.piece.map(|p| (Pos::from_i(i), p)))
    }

    pub fn mud_left(&self) -> u32 {
        self.cells.iter().map(|c| c.mud as u32).sum()
    }

    pub fn boxes_left(&self) -> u32 {
        self.pieces().filter(|(_, p)| matches!(p.item, Item::Box { .. })).count() as u32
    }

    pub fn bones_on_board(&self) -> u32 {
        self.pieces().filter(|(_, p)| p.item == Item::Bone).count() as u32
    }

    pub fn has_powers(&self) -> bool {
        self.pieces().any(|(_, p)| p.item.is_power())
    }

    pub fn zoomies_ready(&self) -> bool {
        self.charge >= ZOOMIES_FULL
    }

    /// Lowest tile of a column (where bones get delivered), if it has any.
    pub fn column_bottom(&self, x: usize) -> Option<usize> {
        (0..ROWS).rev().find(|&y| self.get(x, y).exists)
    }

    pub fn column_top(&self, x: usize) -> Option<usize> {
        (0..ROWS).find(|&y| self.get(x, y).exists)
    }

    fn kind_at(&self, x: i32, y: i32) -> Option<u8> {
        BoardView { cells: &self.cells }.kind_at(x, y)
    }

    /// Is there a run of three or more through this cell?
    fn run_through(&self, p: Pos) -> bool {
        BoardView { cells: &self.cells }.run_through(p)
    }

    fn has_holes(&self) -> bool {
        self.cells.iter().any(|c| c.exists && c.piece.is_none())
    }

    fn most_common_kind(&mut self) -> Option<u8> {
        let mut counts = [0u32; KINDS];
        for (_, p) in self.pieces() {
            if let Some(k) = p.item.kind() {
                counts[k as usize] += 1;
            }
        }
        let best = *counts.iter().max().unwrap();
        if best == 0 {
            return None;
        }
        let ties: Vec<u8> = (0..KINDS as u8).filter(|&k| counts[k as usize] == best).collect();
        Some(self.rng.pick(&ties))
    }

    // ---- matching ---------------------------------------------------------

    fn find_groups(&self) -> Vec<Group> {
        let mut parent: [usize; N] = std::array::from_fn(|i| i);
        fn root(parent: &mut [usize; N], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        let mut in_h = [0u8; N];
        let mut in_v = [0u8; N];

        for y in 0..ROWS {
            let mut x = 0;
            while x < COLS {
                let Some(k) = self.kind_at(x as i32, y as i32) else {
                    x += 1;
                    continue;
                };
                let mut e = x + 1;
                while e < COLS && self.kind_at(e as i32, y as i32) == Some(k) {
                    e += 1;
                }
                if e - x >= 3 {
                    let first = root(&mut parent, Pos::new(x, y).i());
                    for i in x..e {
                        let c = Pos::new(i, y).i();
                        in_h[c] = (e - x) as u8;
                        let r = root(&mut parent, c);
                        parent[r] = first;
                    }
                }
                x = e;
            }
        }
        for x in 0..COLS {
            let mut y = 0;
            while y < ROWS {
                let Some(k) = self.kind_at(x as i32, y as i32) else {
                    y += 1;
                    continue;
                };
                let mut e = y + 1;
                while e < ROWS && self.kind_at(x as i32, e as i32) == Some(k) {
                    e += 1;
                }
                if e - y >= 3 {
                    let first = root(&mut parent, Pos::new(x, y).i());
                    for i in y..e {
                        let c = Pos::new(x, i).i();
                        in_v[c] = (e - y) as u8;
                        let r = root(&mut parent, c);
                        parent[r] = first;
                    }
                }
                y = e;
            }
        }

        let mut groups: Vec<(usize, Group)> = Vec::new();
        for i in 0..N {
            if in_h[i] == 0 && in_v[i] == 0 {
                continue;
            }
            let r = root(&mut parent, i);
            let gi = match groups.iter().position(|(gr, _)| *gr == r) {
                Some(gi) => gi,
                None => {
                    let kind = self.cells[i].piece.unwrap().item.kind().unwrap();
                    groups.push((
                        r,
                        Group { kind, cells: Vec::new(), max_run: 0, has_h: false, has_v: false, cross: None },
                    ));
                    groups.len() - 1
                }
            };
            let g = &mut groups[gi].1;
            g.cells.push(Pos::from_i(i));
            g.max_run = g.max_run.max(in_h[i]).max(in_v[i]);
            g.has_h |= in_h[i] > 0;
            g.has_v |= in_v[i] > 0;
            if in_h[i] > 0 && in_v[i] > 0 && g.cross.is_none() {
                g.cross = Some(Pos::from_i(i));
            }
        }
        groups.into_iter().map(|(_, g)| g).collect()
    }

    // ---- moves ------------------------------------------------------------

    fn is_combo(a: Item, b: Item) -> bool {
        match (a, b) {
            (Item::Super, Item::Super) => true,
            (Item::Super, Item::Normal { .. }) | (Item::Normal { .. }, Item::Super) => true,
            (Item::Normal { special: sa, .. }, Item::Normal { special: sb, .. }) => {
                sa != Special::None && sb != Special::None
            }
            _ => false,
        }
    }

    fn swap_cells(&mut self, a: Pos, b: Pos) {
        let pa = self.cells[a.i()].piece;
        self.cells[a.i()].piece = self.cells[b.i()].piece;
        self.cells[b.i()].piece = pa;
    }

    /// Would swapping these two neighbours do anything?
    pub fn can_swap(&self, a: Pos, b: Pos) -> bool {
        if !a.adjacent(b) {
            return false;
        }
        let (Some(pa), Some(pb)) = (self.cells[a.i()].piece, self.cells[b.i()].piece) else {
            return false;
        };
        if matches!(pa.item, Item::Box { .. }) || matches!(pb.item, Item::Box { .. }) {
            return false;
        }
        if Board::is_combo(pa.item, pb.item) {
            return true;
        }
        // Only the pieces move; tiles (exists/mud) stay put.
        let mut probe = self.cells;
        let (ca, cb) = (self.cells[a.i()], self.cells[b.i()]);
        probe[a.i()] = Cell { piece: cb.piece, ..ca };
        probe[b.i()] = Cell { piece: ca.piece, ..cb };
        let view = BoardView { cells: &probe };
        view.run_through(a) || view.run_through(b)
    }

    /// Tries the player's move. On success the pieces have been exchanged and
    /// `advance()` will play out the result; on failure nothing has changed.
    pub fn swap(&mut self, a: Pos, b: Pos) -> bool {
        if !self.can_swap(a, b) {
            return false;
        }
        let (ia, ib) = (self.cells[a.i()].piece.unwrap().item, self.cells[b.i()].piece.unwrap().item);
        self.swap_cells(a, b);
        self.swap_horizontal = a.y == b.y;
        self.cascade = 0;
        if Board::is_combo(ia, ib) {
            self.combo = Some((a, b));
        } else {
            self.last_swap = Some((a, b));
        }
        true
    }

    /// Every swap that would do something.
    pub fn find_moves(&self) -> Vec<Move> {
        let mut out = Vec::new();
        for y in 0..ROWS {
            for x in 0..COLS {
                let a = Pos::new(x, y);
                for b in [a.offset(1, 0), a.offset(0, 1)].into_iter().flatten() {
                    if self.can_swap(a, b) {
                        out.push(Move { a, b });
                    }
                }
            }
        }
        out
    }

    // ---- the turn loop ------------------------------------------------------

    /// Performs the next piece of board business and says what it was.
    pub fn advance(&mut self) -> Step {
        if self.combo.is_some() || !self.triggers.is_empty() {
            return Step::Clear(self.clear_step(Vec::new()));
        }
        if self.has_holes() {
            return Step::Fall(self.fall());
        }
        let groups = self.find_groups();
        if !groups.is_empty() {
            return Step::Clear(self.clear_step(groups));
        }
        let bones = self.collect_bones();
        if !bones.is_empty() {
            return Step::Bones(bones);
        }
        self.last_swap = None;
        if self.find_moves().is_empty() {
            let moved = self.shuffle();
            if !moved.is_empty() {
                return Step::Shuffle(moved);
            }
        }
        self.cascade = 0;
        self.no_charge = false;
        Step::Idle
    }

    /// Runs `advance()` until the board settles. For tests and simulation.
    pub fn settle(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        for _ in 0..10_000 {
            match self.advance() {
                Step::Idle => return steps,
                s => steps.push(s),
            }
        }
        panic!("board did not settle");
    }

    // ---- clearing -----------------------------------------------------------

    fn clear_step(&mut self, groups: Vec<Group>) -> ClearReport {
        self.cascade += 1;
        self.tally.best_cascade = self.tally.best_cascade.max(self.cascade);
        let score_before = self.score;
        let mut ctx = Ctx::new(self.cascade);

        if let Some((a, b)) = self.combo.take() {
            self.start_combo(&mut ctx, a, b);
        }
        for (t, trig) in std::mem::take(&mut self.triggers) {
            match trig {
                Trigger::Row(p) => self.blast_row(&mut ctx, p, t, None),
                Trigger::Col(p) => self.blast_col(&mut ctx, p, t, None),
                Trigger::Fire(p) => ctx.hit(t, p, Cause::Fire, u16::MAX),
            }
        }
        for g in &groups {
            self.clear_group(&mut ctx, g);
        }
        while let Some(Reverse((t, _, i, cause, fx))) = ctx.heap.pop() {
            self.apply_hit(&mut ctx, t, Pos::from_i(i as usize), CAUSES[cause as usize], fx);
        }
        self.last_swap = None;

        let mut rep = ctx.rep;
        let last = rep
            .pops
            .iter()
            .map(|p| p.t)
            .chain(rep.made.iter().map(|m| m.t))
            .chain(rep.box_hits.iter().map(|b| b.t))
            .chain(rep.mud_hits.iter().map(|m| m.t))
            .chain(rep.morphs.iter().map(|m| m.t))
            .max()
            .unwrap_or(0);
        rep.duration = last + TAIL;
        rep.gained = self.score - score_before;
        if !self.no_charge {
            let bonus = if self.cascade >= 2 { 3 } else { 0 };
            self.charge = (self.charge + rep.pops.len() as u32 + bonus).min(ZOOMIES_FULL);
        }
        rep
    }

    fn clear_group(&mut self, ctx: &mut Ctx, g: &Group) {
        let swapped = self
            .last_swap
            .and_then(|(a, b)| [b, a].into_iter().find(|p| g.cells.contains(p)));

        let make = if g.max_run >= 5 {
            Some(Item::Super)
        } else if g.has_h && g.has_v {
            Some(Item::Normal { kind: g.kind, special: Special::Bomb })
        } else if g.max_run == 4 {
            let horizontal = if swapped.is_some() { self.swap_horizontal } else { self.rng.chance(0.5) };
            let special = if horizontal { Special::LineH } else { Special::LineV };
            Some(Item::Normal { kind: g.kind, special })
        } else {
            None
        };
        let spawn = make.map(|_| swapped.or(g.cross).unwrap_or(g.cells[g.cells.len() / 2]));

        let score_before = self.score;
        let mut set_off: Vec<(Item, Pos)> = Vec::new();
        for &pos in &g.cells {
            if let Some(piece) = self.pop_at(ctx, pos, 0, Cause::Match, None, spawn) {
                if piece.item.is_power() {
                    set_off.push((piece.item, pos));
                }
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let Some(n) = pos.offset(dx, dy) else { continue };
                if ctx.box_adj_done[n.i()] {
                    continue;
                }
                if matches!(self.cells[n.i()].piece, Some(Piece { item: Item::Box { .. }, .. })) {
                    ctx.box_adj_done[n.i()] = true;
                    self.damage_box(ctx, n, 60);
                }
            }
        }
        if let (Some(item), Some(pos)) = (make, spawn) {
            let piece = self.new_piece(item);
            self.cells[pos.i()].piece = Some(piece);
            ctx.protected[pos.i()] = true;
            ctx.rep.made.push(Made { id: piece.id, pos, item, t: 0 });
            self.tally.specials += 1;
            self.score += ctx.mult
                * match item {
                    Item::Super => PTS_SUPER,
                    Item::Normal { special: Special::Bomb, .. } => PTS_BOMB,
                    _ => PTS_LINE,
                };
        }
        for (item, pos) in set_off {
            self.trigger(ctx, item, pos, 0);
        }
        let n = g.cells.len() as f32;
        ctx.rep.groups.push(GroupInfo {
            kind: g.kind,
            size: g.cells.len() as u8,
            cx: g.cells.iter().map(|p| p.x as f32).sum::<f32>() / n,
            cy: g.cells.iter().map(|p| p.y as f32).sum::<f32>() / n,
            points: self.score - score_before,
            made: make,
        });
    }

    /// Removes a matchable piece, recording it. Does not set off specials.
    fn pop_at(
        &mut self,
        ctx: &mut Ctx,
        pos: Pos,
        t: u32,
        cause: Cause,
        fx: Option<u16>,
        into: Option<Pos>,
    ) -> Option<Piece> {
        let piece = self.cells[pos.i()].piece?;
        if !matches!(piece.item, Item::Normal { .. } | Item::Super) {
            return None;
        }
        self.cells[pos.i()].piece = None;
        let base = match (piece.item, cause) {
            (Item::Super, _) => 50,
            (_, Cause::Match) => PTS_MATCH,
            _ => PTS_BLAST,
        };
        let points = base * ctx.mult;
        self.score += points;
        if let Some(k) = piece.item.kind() {
            self.tally.kinds[k as usize] += 1;
        }
        ctx.rep.pops.push(Pop { id: piece.id, pos, item: piece.item, t, cause, into, blast: fx, points });
        self.mud_hit(ctx, pos, t);
        Some(piece)
    }

    fn mud_hit(&mut self, ctx: &mut Ctx, pos: Pos, t: u32) {
        let cell = &mut self.cells[pos.i()];
        if cell.mud > 0 {
            cell.mud -= 1;
            ctx.rep.mud_hits.push(MudHit { pos, t, left: cell.mud });
            self.tally.mud += 1;
            self.score += PTS_MUD * ctx.mult;
        }
    }

    fn damage_box(&mut self, ctx: &mut Ctx, pos: Pos, t: u32) {
        let Some(Piece { id, item: Item::Box { hp } }) = self.cells[pos.i()].piece else { return };
        let hp_left = hp.saturating_sub(1);
        if hp_left == 0 {
            self.cells[pos.i()].piece = None;
            self.tally.boxes += 1;
            self.score += PTS_BOX * ctx.mult;
        } else {
            self.cells[pos.i()].piece = Some(Piece { id, item: Item::Box { hp: hp_left } });
        }
        ctx.rep.box_hits.push(BoxHit { id, pos, t, hp_left });
    }

    fn apply_hit(&mut self, ctx: &mut Ctx, t: u32, pos: Pos, cause: Cause, fx: u16) {
        let i = pos.i();
        if !self.cells[i].exists || ctx.protected[i] {
            return;
        }
        let fx = (fx != u16::MAX).then_some(fx);
        match self.cells[i].piece {
            None => self.mud_hit(ctx, pos, t),
            Some(p) => match p.item {
                Item::Bone => {}
                Item::Box { .. } => self.damage_box(ctx, pos, t),
                Item::Normal { .. } | Item::Super => {
                    self.pop_at(ctx, pos, t, cause, fx, None);
                    self.trigger(ctx, p.item, pos, t);
                }
            },
        }
    }

    fn trigger(&mut self, ctx: &mut Ctx, item: Item, at: Pos, t: u32) {
        match item {
            Item::Normal { kind, special: Special::LineH } => self.blast_row(ctx, at, t, Some(kind)),
            Item::Normal { kind, special: Special::LineV } => self.blast_col(ctx, at, t, Some(kind)),
            Item::Normal { kind, special: Special::Bomb } => self.blast_bomb(ctx, at, t, Some(kind), false),
            Item::Super => {
                if let Some(kind) = self.most_common_kind() {
                    self.blast_fetch(ctx, at, t, kind);
                }
            }
            _ => {}
        }
    }

    fn blast_row(&mut self, ctx: &mut Ctx, at: Pos, t: u32, tint: Option<u8>) {
        let dir: i8 = if self.rng.chance(0.5) { 1 } else { -1 };
        let fx = ctx.fx(BlastKind::Row { dir }, at, t, tint);
        for x in 0..COLS {
            let rank = if dir > 0 { x } else { COLS - 1 - x } as u32;
            let p = Pos::new(x, at.y as usize);
            if self.cells[p.i()].exists {
                ctx.hit(t + RUN_LEAD + RUN_STEP * rank, p, Cause::Row, fx);
            }
        }
    }

    fn blast_col(&mut self, ctx: &mut Ctx, at: Pos, t: u32, tint: Option<u8>) {
        let dir: i8 = if self.rng.chance(0.5) { 1 } else { -1 };
        let fx = ctx.fx(BlastKind::Col { dir }, at, t, tint);
        for y in 0..ROWS {
            let rank = if dir > 0 { y } else { ROWS - 1 - y } as u32;
            let p = Pos::new(at.x as usize, y);
            if self.cells[p.i()].exists {
                ctx.hit(t + RUN_LEAD + RUN_STEP * rank, p, Cause::Col, fx);
            }
        }
    }

    fn blast_bomb(&mut self, ctx: &mut Ctx, at: Pos, t: u32, tint: Option<u8>, big: bool) {
        let fx = ctx.fx(BlastKind::Bomb { big }, at, t, tint);
        let reach: i32 = if big { 3 } else { 2 };
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let inside = if big {
                    // 5x5 block with a tip on each side
                    (dx.abs() <= 2 && dy.abs() <= 2) || dx.abs() + dy.abs() <= 3
                } else {
                    // diamond: 3x3 block with a tip on each side
                    dx.abs() + dy.abs() <= 2
                };
                if !inside {
                    continue;
                }
                let Some(p) = at.offset(dx, dy) else { continue };
                if self.cells[p.i()].exists {
                    let ring = dx.abs().max(dy.abs()) as u32;
                    ctx.hit(t + BOMB_LEAD + BOMB_RING * ring, p, Cause::Bomb, fx);
                }
            }
        }
    }

    fn of_kind(&self, kind: u8, from: Pos) -> Vec<Pos> {
        let mut v: Vec<Pos> = self
            .pieces()
            .filter(|(_, p)| p.item.kind() == Some(kind))
            .map(|(pos, _)| pos)
            .collect();
        let d = |p: &Pos| {
            let (dx, dy) = (p.x as i32 - from.x as i32, p.y as i32 - from.y as i32);
            dx * dx + dy * dy
        };
        v.sort_by_key(|p| (d(p), p.i()));
        v
    }

    fn blast_fetch(&mut self, ctx: &mut Ctx, at: Pos, t: u32, kind: u8) {
        let fx = ctx.fx(BlastKind::Fetch { kind }, at, t, Some(kind));
        for (n, p) in self.of_kind(kind, at).into_iter().enumerate() {
            ctx.hit(t + FETCH_LEAD + FETCH_STEP * n as u32, p, Cause::Fetch, fx);
        }
    }

    /// Golden Rufus swapped onto a special: every piece of that kind becomes
    /// that special, then they all go off.
    fn blast_morph(&mut self, ctx: &mut Ctx, at: Pos, t: u32, kind: u8, special: Special) {
        let fx = ctx.fx(BlastKind::Fetch { kind }, at, t, Some(kind));
        let targets = self.of_kind(kind, at);
        let n = targets.len() as u32;
        for (k, &p) in targets.iter().enumerate() {
            let piece = self.cells[p.i()].piece.unwrap();
            if piece.item.is_plain() {
                let sp = if special.is_line() {
                    if self.rng.chance(0.5) { Special::LineH } else { Special::LineV }
                } else {
                    special
                };
                let item = Item::Normal { kind, special: sp };
                self.cells[p.i()].piece = Some(Piece { id: piece.id, item });
                ctx.rep.morphs.push(Morph { id: piece.id, pos: p, item, t: t + MORPH_LEAD + MORPH_STEP * k as u32 });
            }
        }
        let start = t + MORPH_LEAD + MORPH_STEP * n + 320;
        for (k, &p) in targets.iter().enumerate() {
            ctx.hit(start + 110 * k as u32, p, Cause::Fetch, fx);
        }
    }

    fn blast_all(&mut self, ctx: &mut Ctx, at: Pos, t: u32) {
        let fx = ctx.fx(BlastKind::Everything, at, t, None);
        for i in 0..N {
            if !self.cells[i].exists {
                continue;
            }
            let p = Pos::from_i(i);
            let ring = (p.x as i32 - at.x as i32).abs().max((p.y as i32 - at.y as i32).abs()) as u32;
            ctx.hit(t + ALL_LEAD + ALL_RING * ring, p, Cause::All, fx);
        }
    }

    /// Two power pieces were swapped onto each other. `b` is where the
    /// dragged piece ended up, and is where the effect is centred.
    fn start_combo(&mut self, ctx: &mut Ctx, a: Pos, b: Pos) {
        let ia = self.cells[a.i()].piece.unwrap().item;
        let ib = self.cells[b.i()].piece.unwrap().item;
        match (ia, ib) {
            (Item::Super, Item::Super) => {
                self.pop_at(ctx, a, 0, Cause::Combo, None, Some(b));
                self.pop_at(ctx, b, 0, Cause::Combo, None, None);
                self.blast_all(ctx, b, 0);
            }
            (Item::Super, Item::Normal { kind, special }) | (Item::Normal { kind, special }, Item::Super) => {
                let sup = if ia == Item::Super { a } else { b };
                self.pop_at(ctx, sup, 0, Cause::Combo, None, None);
                if special == Special::None {
                    self.blast_fetch(ctx, sup, 0, kind);
                } else {
                    self.blast_morph(ctx, sup, 0, kind, special);
                }
            }
            (Item::Normal { special: sa, kind: ka }, Item::Normal { special: sb, kind: kb }) => {
                self.pop_at(ctx, a, 0, Cause::Combo, None, Some(b));
                self.pop_at(ctx, b, 0, Cause::Combo, None, None);
                match (sa.is_line(), sb.is_line()) {
                    (true, true) => {
                        self.blast_row(ctx, b, 0, Some(ka));
                        self.blast_col(ctx, b, 0, Some(kb));
                    }
                    (false, false) => self.blast_bomb(ctx, b, 0, Some(kb), true),
                    _ => {
                        // Zoomies + Big Woof: three rows and three columns.
                        let mut t = 0;
                        for d in [0, -1, 1] {
                            if let Some(p) = b.offset(0, d) {
                                self.blast_row(ctx, p, t, Some(ka));
                            }
                            if let Some(p) = b.offset(d, 0) {
                                self.blast_col(ctx, p, t + 60, Some(kb));
                            }
                            t += 120;
                        }
                    }
                }
            }
            _ => unreachable!("not a combo: {ia:?} + {ib:?}"),
        }
    }

    // ---- gravity ------------------------------------------------------------

    fn spawn_item(&mut self, top_of_column: bool, bone_spawned: &mut bool) -> Item {
        if top_of_column
            && !*bone_spawned
            && self.bones_queued > 0
            && self.bones_on_board() < self.bones_cap
            && self.rng.chance(0.4)
        {
            *bone_spawned = true;
            self.bones_queued -= 1;
            return Item::Bone;
        }
        Item::plain(self.random_kind())
    }

    fn fall(&mut self) -> FallReport {
        let mut falls = Vec::new();
        let mut bone_spawned = false;
        for x in 0..COLS {
            let rows: Vec<usize> = (0..ROWS).filter(|&y| self.get(x, y).exists).collect();
            if rows.is_empty() {
                continue;
            }
            // Slide everything down. Pieces drop straight through any gap in
            // the column's tiles.
            let mut write = rows.len();
            for r in (0..rows.len()).rev() {
                let y = rows[r];
                if let Some(p) = self.cells[Pos::new(x, y).i()].piece.take() {
                    write -= 1;
                    let ty = rows[write];
                    self.cells[Pos::new(x, ty).i()].piece = Some(p);
                    if ty != y {
                        falls.push(Fall { id: p.id, item: p.item, from_y: y as i32, to: Pos::new(x, ty), spawned: false });
                    }
                }
            }
            // New pieces queue up above the top tile.
            let top = rows[0] as i32;
            let mut queued = 0;
            for r in (0..write).rev() {
                queued += 1;
                let item = self.spawn_item(r == 0, &mut bone_spawned);
                let p = self.new_piece(item);
                let to = Pos::new(x, rows[r]);
                self.cells[to.i()].piece = Some(p);
                falls.push(Fall { id: p.id, item, from_y: top - queued, to, spawned: true });
            }
        }
        FallReport { falls }
    }

    fn collect_bones(&mut self) -> Vec<(u32, Pos)> {
        let mut out = Vec::new();
        for x in 0..COLS {
            let Some(y) = self.column_bottom(x) else { continue };
            let pos = Pos::new(x, y);
            if let Some(Piece { id, item: Item::Bone }) = self.cells[pos.i()].piece {
                self.cells[pos.i()].piece = None;
                self.tally.bones += 1;
                self.score += PTS_BONE;
                out.push((id, pos));
            }
        }
        out
    }

    // ---- shuffling ----------------------------------------------------------

    fn shuffle(&mut self) -> Vec<Shuffled> {
        let slots: Vec<usize> = (0..N)
            .filter(|&i| matches!(self.cells[i].piece, Some(Piece { item: Item::Normal { .. } | Item::Super, .. })))
            .collect();
        if slots.len() < 3 {
            return Vec::new();
        }
        let before: Vec<Piece> = slots.iter().map(|&i| self.cells[i].piece.unwrap()).collect();
        let mut pieces = before.clone();
        let mut ok = false;
        for _ in 0..300 {
            self.rng.shuffle(&mut pieces);
            for (&i, &p) in slots.iter().zip(&pieces) {
                self.cells[i].piece = Some(p);
            }
            if self.find_groups().is_empty() && !self.find_moves().is_empty() {
                ok = true;
                break;
            }
        }
        if !ok {
            // The pieces on hand can't make a playable board (say, one of
            // each kind left). Deal fresh kinds onto the same pieces.
            for _ in 0..500 {
                for (&i, p) in slots.iter().zip(pieces.iter_mut()) {
                    if p.item.is_plain() {
                        p.item = Item::plain(self.random_kind());
                    }
                    self.cells[i].piece = Some(*p);
                }
                if self.find_groups().is_empty() && !self.find_moves().is_empty() {
                    break;
                }
            }
        }
        let mut out = Vec::new();
        for (&to, p) in slots.iter().zip(&pieces) {
            let from = slots[before.iter().position(|b| b.id == p.id).unwrap()];
            out.push(Shuffled { id: p.id, item: p.item, from: Pos::from_i(from), to: Pos::from_i(to) });
        }
        out
    }

    // ---- extras -------------------------------------------------------------

    /// Spends the full meter: Rufus does three laps (two rows and a column).
    /// Costs no move. Call `advance()` afterwards to play it out.
    pub fn zoomies(&mut self) -> bool {
        if !self.zoomies_ready() {
            return false;
        }
        self.charge = 0;
        self.cascade = 0;
        self.no_charge = true;
        let mut rows: Vec<usize> = (0..ROWS).filter(|&y| (0..COLS).any(|x| self.get(x, y).exists)).collect();
        let mut cols: Vec<usize> = (0..COLS).filter(|&x| self.column_top(x).is_some()).collect();
        self.rng.shuffle(&mut rows);
        self.rng.shuffle(&mut cols);
        let mut t = 0;
        if let Some(&y) = rows.first() {
            self.triggers.push((t, Trigger::Row(Pos::new(0, y))));
            t += 400;
        }
        if let Some(&x) = cols.first() {
            self.triggers.push((t, Trigger::Col(Pos::new(x, 0))));
            t += 460;
        }
        if let Some(&y) = rows.get(1) {
            self.triggers.push((t, Trigger::Row(Pos::new(0, y))));
        }
        true
    }

    /// End-of-level bonus: turns one plain piece into a random power piece.
    pub fn plant_bonus(&mut self) -> Option<Morph> {
        let plain: Vec<Pos> = self.pieces().filter(|(_, p)| p.item.is_plain()).map(|(pos, _)| pos).collect();
        if plain.is_empty() {
            return None;
        }
        let pos = self.rng.pick(&plain);
        let piece = self.cells[pos.i()].piece.unwrap();
        let special = match self.rng.below(5) {
            0 => Special::Bomb,
            1 | 2 => Special::LineH,
            _ => Special::LineV,
        };
        let item = Item::Normal { kind: piece.item.kind().unwrap(), special };
        self.cells[pos.i()].piece = Some(Piece { id: piece.id, item });
        self.score += PTS_BONUS;
        Some(Morph { id: piece.id, pos, item, t: 0 })
    }

    /// Queues every power piece on the board to go off, one after another.
    /// Returns false if there were none. Call `advance()` to play it out.
    pub fn fire_powers(&mut self) -> bool {
        let mut at: Vec<Pos> = self.pieces().filter(|(_, p)| p.item.is_power()).map(|(pos, _)| pos).collect();
        if at.is_empty() {
            return false;
        }
        self.rng.shuffle(&mut at);
        self.cascade = 0;
        self.no_charge = true;
        for (k, p) in at.into_iter().enumerate() {
            self.triggers.push((k as u32 * 140, Trigger::Fire(p)));
        }
        true
    }

    /// Test helper: put a specific item on a tile.
    pub fn place(&mut self, pos: Pos, item: Item) -> u32 {
        let p = self.new_piece(item);
        self.cells[pos.i()].exists = true;
        self.cells[pos.i()].piece = Some(p);
        p.id
    }
}

/// Read-only match check over a borrowed grid, for trying swaps without
/// cloning the whole board.
struct BoardView<'a> {
    cells: &'a [Cell; N],
}

impl BoardView<'_> {
    fn kind_at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x >= COLS as i32 || y >= ROWS as i32 {
            return None;
        }
        let c = &self.cells[y as usize * COLS + x as usize];
        if !c.exists {
            return None;
        }
        c.piece.and_then(|p| p.item.kind())
    }

    fn run_through(&self, p: Pos) -> bool {
        let (x, y) = (p.x as i32, p.y as i32);
        let Some(k) = self.kind_at(x, y) else { return false };
        for (dx, dy) in [(1, 0), (0, 1)] {
            let mut n = 1;
            let mut s = 1;
            while self.kind_at(x + dx * s, y + dy * s) == Some(k) {
                n += 1;
                s += 1;
            }
            s = 1;
            while self.kind_at(x - dx * s, y - dy * s) == Some(k) {
                n += 1;
                s += 1;
            }
            if n >= 3 {
                return true;
            }
        }
        false
    }
}
