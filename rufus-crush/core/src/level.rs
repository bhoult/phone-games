//! Level definitions and the per-level session (moves, goals, paw rating).

use crate::board::*;

/// Piece kinds, in atlas order.
pub const BALL: u8 = 0;
pub const SQUIRREL: u8 = 1;
pub const CAT: u8 = 2;
pub const SOCK: u8 = 3;
pub const POODLE: u8 = 4;
pub const TOY: u8 = 5;
pub const STICK: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    /// Reach this score.
    Score(u32),
    /// Clear this many pieces of a kind.
    Kind(u8, u32),
    /// Clean every muddy paw print.
    Mud,
    /// Walk this many bones down to the bottom.
    Bones(u32),
    /// Open every delivery box.
    Boxes,
}

/// How generous the move budgets are. The player picks; the goals, boards
/// and rules are the same on all three.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Difficulty {
        Difficulty::ALL[i.min(2)]
    }

    pub fn easier(self) -> Option<Difficulty> {
        self.index().checked_sub(1).map(Difficulty::from_index)
    }
}

#[derive(Clone, Copy)]
pub struct LevelDef {
    pub title: &'static str,
    /// One line for the level card: teaches the new thing, or is just a joke.
    pub tip: &'static str,
    /// Move budget on Easy, Normal and Hard. Written by `tools/apply_tune.py`.
    pub moves: [u32; 3],
    /// Piece kinds in play.
    pub kinds: &'static [u8],
    pub goals: &'static [Goal],
    pub rows: [&'static str; ROWS],
    /// Score needed for two and three paws (finishing at all earns one).
    pub paws: [u32; 2],
}

pub use crate::chapters::{Chapter, Place, CHAPTERS, CHAPTER_LEN};

use Goal::*;

const K4: &[u8] = &[BALL, SQUIRREL, CAT, SOCK];
const K5: &[u8] = &[BALL, SQUIRREL, CAT, SOCK, POODLE];
const K5_TOY: &[u8] = &[BALL, SQUIRREL, CAT, POODLE, TOY];
const K5_HOME: &[u8] = &[BALL, CAT, SOCK, POODLE, TOY];
const K5_FETCH: &[u8] = &[BALL, SQUIRREL, CAT, POODLE, STICK];
const K5_LAKE: &[u8] = &[BALL, SQUIRREL, POODLE, TOY, STICK];
const K6: &[u8] = &[BALL, SQUIRREL, CAT, SOCK, POODLE, TOY];

/// The first day: the levels drawn by hand.
pub const HAND_MADE: usize = 36;

/// Every level. The first [`HAND_MADE`] are the ones below; the rest were
/// designed by `examples/mklevels.rs` and live in `later.rs`.
pub static LEVELS: [LevelDef; HAND_MADE + crate::later::COUNT] = all_levels();

const fn all_levels() -> [LevelDef; HAND_MADE + crate::later::COUNT] {
    let mut out = [HAND[0]; HAND_MADE + crate::later::COUNT];
    let mut i = 0;
    while i < HAND_MADE {
        out[i] = HAND[i];
        i += 1;
    }
    while i < out.len() {
        out[i] = crate::later::LATER[i - HAND_MADE];
        i += 1;
    }
    out
}

/// How many chapters there are. The last one may be short.
pub fn chapter_count() -> usize {
    LEVELS.len().div_ceil(CHAPTER_LEN)
}

/// The levels of a chapter.
pub fn chapter_levels(chapter: usize) -> std::ops::Range<usize> {
    chapter * CHAPTER_LEN..((chapter + 1) * CHAPTER_LEN).min(LEVELS.len())
}

/// Where a level takes place.
pub fn place_of(level: usize) -> Place {
    CHAPTERS[(level / CHAPTER_LEN).min(CHAPTERS.len() - 1)].place
}

const HAND: [LevelDef; HAND_MADE] = [
    // ---- The Backyard: learning the ropes ---------------------------------
    LevelDef {
        title: "Good Morning, Rufus",
        tip: "Swipe to line up 3 of a kind.",
        moves: [34, 26, 15],
        kinds: K4,
        goals: &[Score(8500)],
        rows: [
            "       ",
            "       ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "       ",
        ],
        paws: [12800, 16200],
    },
    LevelDef {
        title: "Ball. Ball? BALL!",
        tip: "Rufus needs tennis balls. He will not say why.",
        moves: [42, 36, 17],
        kinds: K4,
        goals: &[Kind(BALL, 120)],
        rows: [
            "       ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "       ",
        ],
        paws: [23400, 30300],
    },
    LevelDef {
        title: "Zoomies",
        tip: "Match 4 in a line and Rufus gets the ZOOMIES.",
        moves: [35, 27, 18],
        kinds: K4,
        goals: &[Score(16000)],
        rows: [
            "       ",
            " ..... ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            " ..... ",
            "       ",
        ],
        paws: [20400, 23900],
    },
    LevelDef {
        title: "Squirrel Patrol",
        tip: "The squirrels are plotting something. Rufus is sure of it.",
        moves: [33, 29, 17],
        kinds: K5,
        goals: &[Kind(SQUIRREL, 52), Kind(CAT, 40)],
        rows: [
            "       ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
        ],
        paws: [9600, 12100],
    },
    LevelDef {
        title: "The Big Woof",
        tip: "Match in an L or a T for a BIG WOOF.",
        moves: [36, 30, 19],
        kinds: K5,
        goals: &[Score(4600)],
        rows: [
            "   .   ",
            "  ...  ",
            " ..... ",
            ".......",
            ".......",
            ".......",
            " ..... ",
            "  ...  ",
            "   .   ",
        ],
        paws: [7200, 8300],
    },
    LevelDef {
        title: "Golden Boy",
        tip: "Match 5 in a line for Golden Rufus. He fetches EVERYTHING.",
        moves: [39, 36, 16],
        kinds: K5,
        goals: &[Kind(BALL, 62), Kind(SOCK, 62)],
        rows: [
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
        ],
        paws: [13300, 15800],
    },
    // ---- The Dog Park: bones ----------------------------------------------
    LevelDef {
        title: "New Friend Smell",
        tip: "A poodle! Rufus would like to smell her. Immediately.",
        moves: [27, 22, 18],
        kinds: K5,
        goals: &[Kind(POODLE, 62)],
        rows: [
            "       ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
        ],
        paws: [9500, 12100],
    },
    LevelDef {
        title: "Buried Treasure",
        tip: "Walk the bones down to the bottom of the board.",
        moves: [35, 26, 19],
        kinds: K5,
        goals: &[Bones(4)],
        rows: [
            "       ",
            ".o...o.",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "       ",
        ],
        paws: [8200, 11000],
    },
    LevelDef {
        title: "Fetch, Don't Return",
        tip: "Rufus brings the stick back. Rufus keeps the stick. Both are true.",
        moves: [26, 21, 17],
        kinds: K5_FETCH,
        goals: &[Kind(BALL, 52), Kind(STICK, 52)],
        rows: [
            ".......",
            ".......",
            " ..... ",
            " ..... ",
            " ..... ",
            " ..... ",
            " ..... ",
            ".......",
            ".......",
        ],
        paws: [8300, 10800],
    },
    LevelDef {
        title: "Sniff Protocol",
        tip: "Every blade of grass has news. Rufus reads all of it.",
        moves: [31, 23, 18],
        kinds: K5_TOY,
        goals: &[Score(4500)],
        rows: [
            "       ",
            "..   ..",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "..   ..",
            "       ",
        ],
        paws: [6000, 7200],
    },
    LevelDef {
        title: "No Take, Only Throw",
        tip: "More bones arrive from the top as you deliver them.",
        moves: [41, 25, 19],
        kinds: K5_TOY,
        goals: &[Bones(5)],
        rows: [
            "..o.o..",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "       ",
        ],
        paws: [10200, 13700],
    },
    LevelDef {
        title: "King of the Park",
        tip: "Squirrels chased. Poodles greeted. Cats... observed from a distance.",
        moves: [28, 20, 15],
        kinds: K5,
        goals: &[Kind(SQUIRREL, 66), Kind(POODLE, 66), Kind(CAT, 66)],
        rows: [
            " ..... ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            " ..... ",
        ],
        paws: [11200, 14200],
    },
    // ---- The Lake: sticks, and a great deal of water ------------------------
    LevelDef {
        title: "Cannonball",
        tip: "Rufus does not test the water. Rufus IS the test.",
        moves: [28, 24, 19],
        kinds: K5_LAKE,
        goals: &[Kind(STICK, 50)],
        rows: [
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "..   ..",
            ".     .",
            "       ",
        ],
        paws: [7000, 8900],
    },
    LevelDef {
        title: "Fetch! (Wet Edition)",
        tip: "Throw it in the lake. No, further. FURTHER.",
        moves: [26, 20, 17],
        kinds: K5_LAKE,
        goals: &[Kind(STICK, 60), Kind(BALL, 60)],
        rows: [
            "       ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
        ],
        paws: [9500, 12200],
    },
    LevelDef {
        title: "Doggy Paddle",
        tip: "All four legs going. Top speed: a gentle drift.",
        moves: [33, 27, 21],
        kinds: K5_LAKE,
        goals: &[Score(5000)],
        rows: [
            ".......",
            ".......",
            ".......",
            "..   ..",
            "..   ..",
            ".......",
            ".......",
            ".......",
            "       ",
        ],
        paws: [6700, 8000],
    },
    LevelDef {
        title: "Duck Diplomacy",
        tip: "The ducks have asked Rufus to leave. Rufus is thinking it over.",
        moves: [34, 22, 16],
        kinds: K5_TOY,
        goals: &[Bones(4)],
        rows: [
            " .o.o. ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            " ..... ",
            "       ",
        ],
        paws: [8200, 10700],
    },
    LevelDef {
        title: "That's Not a Stick",
        tip: "It's a log. Rufus knows it's a log. He's bringing it anyway.",
        moves: [29, 22, 18],
        kinds: K5_LAKE,
        goals: &[Kind(STICK, 56), Score(6000)],
        rows: [
            ".......",
            ".......",
            " ..... ",
            " ..... ",
            " ..... ",
            " ..... ",
            ".......",
            ".......",
            "       ",
        ],
        paws: [8000, 9700],
    },
    LevelDef {
        title: "The Shake-Off",
        tip: "He waits until he's standing right next to you. Every time.",
        moves: [28, 21, 18],
        kinds: K5_LAKE,
        goals: &[Kind(STICK, 50), Kind(BALL, 50), Kind(TOY, 50)],
        rows: [
            ".......",
            ".......",
            ". ... .",
            ". ... .",
            ".......",
            ".......",
            ".......",
            ".......",
            "       ",
        ],
        paws: [7700, 9600],
    },
    // ---- Mud Season ---------------------------------------------------------
    LevelDef {
        title: "Puddle Inspector",
        tip: "Match on top of muddy paw prints to clean them up.",
        moves: [23, 17, 13],
        kinds: K5,
        goals: &[Mud, Score(5500)],
        rows: [
            "       ",
            ".......",
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            ".......",
            "       ",
        ],
        paws: [7300, 8900],
    },
    LevelDef {
        title: "Who Tracked This In?",
        tip: "Dark mud needs cleaning twice. Rufus has no idea how it got here.",
        moves: [25, 18, 14],
        kinds: K5,
        goals: &[Mud],
        rows: [
            "M..m..M",
            ".M...M.",
            "..M.M..",
            "m..M..m",
            "..M.M..",
            ".M...M.",
            "M..m..M",
            "mmmmmmm",
            "       ",
        ],
        paws: [8900, 11800],
    },
    LevelDef {
        title: "It Wasn't Me",
        tip: "It was like this when he got here. He is prepared to swear to that.",
        moves: [30, 21, 16],
        kinds: K5_TOY,
        goals: &[Mud],
        rows: [
            "       ",
            "MM...MM",
            "Mm...mM",
            "...m...",
            "..mmm..",
            "...m...",
            "Mm...mM",
            "MM...MM",
            "       ",
        ],
        paws: [7900, 10300],
    },
    LevelDef {
        title: "Bath? No Thank You",
        tip: "Rufus loves water. Rufus hates baths. Do not ask him to explain.",
        moves: [25, 20, 16],
        kinds: K5_TOY,
        goals: &[Mud, Kind(TOY, 56)],
        rows: [
            "  ...  ",
            " ..... ",
            ".MmmmM.",
            ".m...m.",
            ".m...m.",
            ".m...m.",
            ".MmmmM.",
            " ..... ",
            "  ...  ",
        ],
        paws: [8700, 11100],
    },
    LevelDef {
        title: "Wall to Wall",
        tip: "The carpet was beige once. Rufus has made improvements.",
        moves: [26, 17, 13],
        kinds: K5_TOY,
        goals: &[Mud],
        rows: [
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            "mmmmmmm",
            "MMMMMMM",
            "MMMMMMM",
        ],
        paws: [12100, 15700],
    },
    LevelDef {
        title: "Swamp Dog",
        tip: "Somewhere under all this is a black lab. Probably.",
        moves: [32, 19, 14],
        kinds: K5_TOY,
        goals: &[Mud, Bones(4)],
        rows: [
            "..o.o..",
            ".......",
            ".MMMMM.",
            ".MMMMM.",
            ".MMMMM.",
            ".mmmmm.",
            ".......",
            ".......",
            "       ",
        ],
        paws: [10100, 13300],
    },
    // ---- Delivery Day -------------------------------------------------------
    LevelDef {
        title: "The Doorbell Incident",
        tip: "Match next to a box to tear it open. Taped ones take two.",
        moves: [20, 15, 12],
        kinds: K5_HOME,
        goals: &[Boxes, Score(4500)],
        rows: [
            ".......",
            ".......",
            ".......",
            ".bbbbb.",
            ".BBBBB.",
            ".bbbbb.",
            ".......",
            ".......",
            ".......",
        ],
        paws: [6600, 8800],
    },
    LevelDef {
        title: "Sock Heist",
        tip: "Nobody has a matching pair any more. Nobody knows why.",
        moves: [27, 21, 17],
        kinds: K5_HOME,
        goals: &[Kind(SOCK, 80)],
        rows: [
            ".......",
            ".......",
            ".......",
            "b.....b",
            ".......",
            ".......",
            "bb...bb",
            ".......",
            ".......",
        ],
        paws: [12400, 15400],
    },
    LevelDef {
        title: "Is This For Me?",
        tip: "Rufus opens every parcel personally. It's a security thing.",
        moves: [24, 18, 14],
        kinds: K5_HOME,
        goals: &[Boxes, Kind(TOY, 54)],
        rows: [
            ".......",
            ".......",
            ".B.B.B.",
            ".......",
            "B.BBB.B",
            ".......",
            ".B.B.B.",
            ".......",
            ".......",
        ],
        paws: [9000, 11800],
    },
    LevelDef {
        title: "Mailman: The Reckoning",
        tip: "He comes every day. Rufus scares him off every day. It's working.",
        moves: [30, 21, 16],
        kinds: K5_HOME,
        goals: &[Boxes, Score(10500)],
        rows: [
            ".......",
            ".......",
            "bbbbbbb",
            ".......",
            ".......",
            ".......",
            "BbBbBbB",
            ".......",
            ".......",
        ],
        paws: [12700, 14800],
    },
    LevelDef {
        title: "Under the Couch",
        tip: "Everything Rufus has ever lost is under there.",
        moves: [35, 23, 17],
        kinds: K5_HOME,
        goals: &[Bones(4)],
        rows: [
            ".o.o.o.",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "bbbbbbb",
            "BBBBBBB",
        ],
        paws: [10600, 14400],
    },
    LevelDef {
        title: "Not On The Couch",
        tip: "He was never on the couch. The warm dog-shaped dent is unrelated.",
        moves: [25, 18, 15],
        kinds: K5_HOME,
        goals: &[Boxes, Kind(SOCK, 62), Kind(TOY, 62)],
        rows: [
            " ..... ",
            ".......",
            "..B.B..",
            ".......",
            "B.....B",
            ".......",
            "..B.B..",
            ".......",
            " ..... ",
        ],
        paws: [10300, 12900],
    },
    // ---- Night Patrol: all six kinds ---------------------------------------
    LevelDef {
        title: "What Was That Noise",
        tip: "It was nothing. Rufus barked at it anyway, to be safe.",
        moves: [35, 27, 21],
        kinds: K6,
        goals: &[Score(6500)],
        rows: [
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
        ],
        paws: [8100, 9400],
    },
    LevelDef {
        title: "Midnight Snack",
        tip: "The fridge opened. Rufus was asleep three rooms away. He's here now.",
        moves: [41, 29, 21],
        kinds: K6,
        goals: &[Bones(3)],
        rows: [
            "       ",
            "o..o..o",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            "       ",
        ],
        paws: [5600, 7000],
    },
    LevelDef {
        title: "The Cat Knows",
        tip: "She saw everything. She's deciding what to do with it.",
        moves: [31, 25, 21],
        kinds: K6,
        goals: &[Kind(CAT, 36), Kind(SQUIRREL, 36)],
        rows: [
            "  ...  ",
            " ..... ",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            " ..... ",
            "  ...  ",
        ],
        paws: [5400, 6900],
    },
    LevelDef {
        title: "Muddy Midnight",
        tip: "Fresh sheets on the bed. Rufus has plans for them.",
        moves: [41, 28, 21],
        kinds: K6,
        goals: &[Mud],
        rows: [
            ".......",
            "MMMMMMM",
            "MMMMMMM",
            "MMMMMMM",
            "MMMMMMM",
            "MMMMMMM",
            "MMMMMMM",
            ".......",
            "       ",
        ],
        paws: [8800, 10700],
    },
    LevelDef {
        title: "One More Lap",
        tip: "It's bedtime, which is the best time for zoomies.",
        moves: [35, 22, 16],
        kinds: K6,
        goals: &[Boxes, Mud],
        rows: [
            ".......",
            ".B.B.B.",
            ".......",
            "MMMMMMM",
            ".......",
            ".B.B.B.",
            ".......",
            "MMMMMMM",
            ".......",
        ],
        paws: [6500, 8300],
    },
    LevelDef {
        title: "Best Boy",
        tip: "A full day of being a very good dog. One last job.",
        moves: [41, 26, 20],
        kinds: K6,
        goals: &[Kind(BALL, 34), Kind(SQUIRREL, 34), Kind(SOCK, 34), Bones(3)],
        rows: [
            "..o.o..",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
            ".......",
        ],
        paws: [7100, 9100],
    },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GoalIcon {
    Score,
    Kind(u8),
    Mud,
    Bone,
    Box,
}

#[derive(Clone, Copy, Debug)]
pub struct GoalItem {
    pub icon: GoalIcon,
    pub have: u32,
    pub need: u32,
}

impl GoalItem {
    pub fn done(&self) -> bool {
        self.have >= self.need
    }
    pub fn left(&self) -> u32 {
        self.need.saturating_sub(self.have)
    }
}

/// One attempt at one level.
#[derive(Clone)]
pub struct Session {
    pub level: usize,
    def: &'static LevelDef,
    pub board: Board,
    pub moves_left: u32,
    mud_total: u32,
    box_total: u32,
    bonus_planted: u32,
    begged: bool,
}

/// Most leftover moves that turn into power pieces at the end of a level.
/// Any beyond that are paid out as points, so the finale stays short.
pub const BONUS_PLANTS: u32 = 10;
pub const PTS_SPARE_MOVE: u32 = 150;

/// The pitiful look. Run out of moves when the level is nearly done and
/// Rufus looks at you until you give him a few more. Once per attempt.
pub const BEG_MOVES: u32 = 3;
/// It only works on a near miss (see [`Session::nearly_done`]): this much of
/// each score or collecting goal has to be done already.
pub const BEG_FROM: f32 = 0.85;

impl Session {
    /// A fresh attempt at Normal difficulty.
    pub fn new(level: usize, seed: u64) -> Session {
        Session::with(level, seed, Difficulty::Normal)
    }

    pub fn with(level: usize, seed: u64, difficulty: Difficulty) -> Session {
        let def = &LEVELS[level];
        Session::of(def, level, seed, def.moves[difficulty.index()])
    }

    /// An attempt at any level definition, with any move budget. `level` is
    /// only a label here: this is how the level designer tries out levels
    /// that are not in [`LEVELS`] yet.
    pub fn of(def: &'static LevelDef, level: usize, seed: u64, moves: u32) -> Session {
        let mut board = Board::from_rows(&def.rows, def.kinds, seed);
        let bones_wanted: u32 = def
            .goals
            .iter()
            .map(|g| if let Goal::Bones(n) = g { *n } else { 0 })
            .sum();
        board.bones_queued = bones_wanted.saturating_sub(board.bones_on_board());
        board.bones_cap = board.bones_on_board().max(2);
        Session {
            level,
            def,
            mud_total: board.mud_left(),
            box_total: board.boxes_left(),
            board,
            moves_left: moves,
            bonus_planted: 0,
            begged: false,
        }
    }

    pub fn def(&self) -> &'static LevelDef {
        self.def
    }

    /// The player's move. Spends a move only if the swap does something.
    pub fn swap(&mut self, a: Pos, b: Pos) -> bool {
        if self.moves_left == 0 || !self.board.swap(a, b) {
            return false;
        }
        self.moves_left -= 1;
        true
    }

    pub fn goals(&self) -> Vec<GoalItem> {
        let t = &self.board.tally;
        self.def()
            .goals
            .iter()
            .map(|g| match *g {
                Goal::Score(n) => GoalItem { icon: GoalIcon::Score, have: self.board.score, need: n },
                Goal::Kind(k, n) => GoalItem { icon: GoalIcon::Kind(k), have: t.kinds[k as usize], need: n },
                Goal::Mud => GoalItem { icon: GoalIcon::Mud, have: t.mud, need: self.mud_total },
                Goal::Bones(n) => GoalItem { icon: GoalIcon::Bone, have: t.bones, need: n },
                Goal::Boxes => GoalItem { icon: GoalIcon::Box, have: t.boxes, need: self.box_total },
            })
            .collect()
    }

    pub fn won(&self) -> bool {
        self.goals().iter().all(|g| g.done())
    }

    pub fn lost(&self) -> bool {
        self.moves_left == 0 && !self.won()
    }

    /// Whether the pitiful look would work right now. A full zoomies meter
    /// comes first: that is free, and Rufus would rather run than beg.
    pub fn can_beg(&self) -> bool {
        !self.begged && self.lost() && !self.board.zoomies_ready() && self.nearly_done()
    }

    /// Close enough to finishing that a few more moves could do it. Every
    /// goal has to be within reach: nearly all the points or pieces, or only
    /// a handful of mud patches, boxes or bones still to go.
    pub fn nearly_done(&self) -> bool {
        self.goals().iter().all(|g| match g.icon {
            GoalIcon::Score | GoalIcon::Kind(_) => g.have as f32 >= BEG_FROM * g.need as f32,
            GoalIcon::Mud => g.left() <= 3,
            GoalIcon::Box => g.left() <= 2,
            GoalIcon::Bone => g.left() <= 1,
        })
    }

    /// The pitiful look: a few more moves, once. Returns whether it worked.
    pub fn beg(&mut self) -> bool {
        if !self.can_beg() {
            return false;
        }
        self.begged = true;
        self.moves_left += BEG_MOVES;
        true
    }

    pub fn begged(&self) -> bool {
        self.begged
    }

    /// 1 to 3 paws for a finished level, from the final score.
    pub fn paws(&self) -> u8 {
        let [two, three] = self.def().paws;
        match self.board.score {
            s if s >= three => 3,
            s if s >= two => 2,
            _ => 1,
        }
    }

    /// End-of-level bonus: spends one leftover move on a power piece.
    pub fn plant_bonus(&mut self) -> Option<Morph> {
        if self.moves_left == 0 || self.bonus_planted >= BONUS_PLANTS {
            return None;
        }
        let m = self.board.plant_bonus()?;
        self.moves_left -= 1;
        self.bonus_planted += 1;
        Some(m)
    }

    /// Pays out any moves still left once planting is done. Returns the points.
    pub fn cash_in_moves(&mut self) -> u32 {
        let pts = self.moves_left * PTS_SPARE_MOVE;
        self.board.score += pts;
        self.moves_left = 0;
        pts
    }

    /// Plays out the whole end-of-level bonus at once (simulation only).
    pub fn finish_bonus(&mut self) {
        while self.plant_bonus().is_some() {}
        self.cash_in_moves();
        while self.board.fire_powers() {
            self.board.settle();
        }
    }
}
