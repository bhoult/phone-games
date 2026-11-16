//! Difficulty tuning: works out how many moves a level should give on each
//! setting by playing it a few hundred times with the stand-in players.
//!
//! Used by `examples/balance.rs` (report and retune) and by
//! `examples/mklevels.rs` (which designs the later levels).

use crate::bot::{self, BotCfg, Outcome};
use crate::chapters::CHAPTER_LEN;
use crate::level::{Difficulty, LevelDef};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// THE DIFFICULTY KNOB. For each setting and each chapter of the first day:
/// the chance that a casual player (one who takes the best-looking move
/// about two times in three) wins a level on any one attempt, at the
/// chapter's first level and at its last. Levels in between are spread
/// evenly. Each chapter opens gentler than the one before closed, because it
/// is teaching something new.
///
/// Every chapter after the first day uses the last column: it opens and
/// closes the way Night Patrol does, and it does not get harder from there.
///
/// Raise a number to make that stretch easier, lower it to make it harder,
/// then run `tools/apply_tune.py`. These describe a stand-in player, not a
/// person: if a setting feels wrong to you, move its whole row.
#[rustfmt::skip]
pub const CURVES: [[(f32, f32); 6]; 3] = [
    // Backyard      Dog Park      The Lake      Mud Season    Delivery Day  Night Patrol and after
    [(0.995, 0.97), (0.97, 0.93), (0.96, 0.92), (0.95, 0.90), (0.94, 0.88), (0.93, 0.85)], // Easy
    [(0.98,  0.88), (0.90, 0.78), (0.87, 0.75), (0.84, 0.71), (0.80, 0.67), (0.76, 0.62)], // Normal
    [(0.92,  0.66), (0.68, 0.46), (0.64, 0.46), (0.58, 0.42), (0.52, 0.36), (0.46, 0.30)], // Hard
];

/// The column of [`CURVES`] a level is tuned against.
fn column(level: usize) -> usize {
    (level / CHAPTER_LEN).min(CURVES[0].len() - 1)
}

/// How often the casual stand-in should win this level on this setting.
pub fn target(d: Difficulty, level: usize) -> f32 {
    let (open, close) = CURVES[d.index()][column(level)];
    open + (close - open) * (level % CHAPTER_LEN) as f32 / (CHAPTER_LEN - 1) as f32
}

/// Easy has to be easy for everybody, not only for the casual stand-in. This
/// is how often a sloppy player (mostly swiping at whatever matches, but
/// using the zoomies when they fill) should get through on Easy.
pub fn sloppy_floor(d: Difficulty, level: usize) -> f32 {
    match d {
        Difficulty::Easy => 0.90 - 0.03 * column(level) as f32,
        _ => 0.0,
    }
}

/// The first chapter also has to work for someone who is just swiping at
/// things and has not found the zoomies meter yet. This is how often that
/// player (random legal moves, no zoomies) should get through.
pub fn swiping_floor(d: Difficulty, level: usize) -> f32 {
    match (d, level) {
        (Difficulty::Easy, 0 | 1) => 0.75,
        (Difficulty::Easy, 2..=5) => 0.40,
        (Difficulty::Normal, 0 | 1) => 0.50,
        (Difficulty::Normal, 2..=5) => 0.22,
        _ => 0.0,
    }
}

/// The tuner and the report use different boards, so the report is a test of
/// the tuning rather than a restatement of it.
pub fn tune_seed(s: u64) -> u64 {
    s * 31 + 5
}
pub fn report_seed(s: u64) -> u64 {
    s * 7919 + 104_729
}

pub fn pct(sorted: &[u32], q: f32) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() - 1) as f32 * q) as usize]
}

pub fn rate(v: &[Outcome]) -> f32 {
    v.iter().filter(|o| o.won).count() as f32 / v.len().max(1) as f32
}

fn games_at(def: &'static LevelDef, cfg: &BotCfg, budget: u32, games: u64) -> Vec<Outcome> {
    (0..games).map(|s| bot::play_def(def, tune_seed(s), cfg, budget)).collect()
}

/// As many moves as anybody could want.
pub const LOTS: u32 = 150;

/// How many moves a win takes this player when moves are no object.
fn moves_needed(def: &'static LevelDef, cfg: &BotCfg, games: u64) -> Vec<u32> {
    let mut needed: Vec<u32> = games_at(def, cfg, LOTS, games)
        .iter()
        .map(|o| if o.won { o.moves_used } else { LOTS })
        .collect();
    needed.sort();
    needed
}

/// The smallest move budget with which this player wins at least `want` of
/// the time.
fn budget_for(def: &'static LevelDef, cfg: &BotCfg, want: f32, games: u64, needed: &[u32]) -> u32 {
    if want <= 0.0 {
        return 3;
    }
    // The percentile is nearly the answer. It is not exactly it, because of
    // what can still happen after the last move: zoomies, and the look.
    let mut b = pct(needed, want).clamp(3, LOTS);
    while b > 3 && rate(&games_at(def, cfg, b - 1, games)) >= want {
        b -= 1;
    }
    while b < LOTS && rate(&games_at(def, cfg, b, games)) < want {
        b += 1;
    }
    b
}

/// What the tuner decides for one level.
pub struct Tuned {
    /// Move budgets for Easy, Normal and Hard.
    pub moves: [u32; 3],
    /// Whether that budget was set by a floor rather than by the curve.
    pub floored: [bool; 3],
    /// Scores for two and three paws.
    pub paws: [u32; 2],
    /// Moves the casual player needs to win: 25th, 50th, 75th percentile.
    pub casual: [u32; 3],
}

/// Tunes one level. `level` is where it sits in the game, which decides how
/// hard it is meant to be; `def` does not have to be in `LEVELS` yet.
pub fn tune_level(def: &'static LevelDef, level: usize, games: u64) -> Tuned {
    let casual = moves_needed(def, &BotCfg::CASUAL, games);
    let random = moves_needed(def, &BotCfg::RANDOM, games);
    let sloppy = moves_needed(def, &BotCfg::SLOPPY, games);
    let mut moves = [0u32; 3];
    let mut floored = [false; 3];
    for d in Difficulty::ALL {
        let by_target = budget_for(def, &BotCfg::CASUAL, target(d, level), games, &casual);
        let by_floor = budget_for(def, &BotCfg::RANDOM, swiping_floor(d, level), games, &random)
            .max(budget_for(def, &BotCfg::SLOPPY, sloppy_floor(d, level), games, &sloppy));
        moves[d.index()] = by_target.max(by_floor);
        floored[d.index()] = by_floor > by_target;
    }
    // an easier setting never gets fewer moves
    moves[1] = moves[1].max(moves[2]);
    moves[0] = moves[0].max(moves[1]);
    // Paw thresholds from the scores both kinds of player finish on at Normal.
    let mut scores: Vec<u32> = [BotCfg::CAREFUL, BotCfg::CASUAL]
        .iter()
        .flat_map(|cfg| games_at(def, cfg, moves[1], games))
        .filter(|o| o.won)
        .map(|o| o.score)
        .collect();
    scores.sort();
    let round = |x: u32| (x + 50) / 100 * 100;
    let two = round(pct(&scores, 0.40));
    let three = round(pct(&scores, 0.80)).max(two + 200);
    Tuned { moves, floored, paws: [two, three], casual: [pct(&casual, 0.25), pct(&casual, 0.5), pct(&casual, 0.75)] }
}

/// Runs `f(0)`, `f(1)`, ... `f(n - 1)` across every core and returns the
/// results in order.
pub fn par_map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<T>>> = Mutex::new((0..n).map(|_| None).collect());
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get()).min(n.max(1));
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= n {
                    break;
                }
                let value = f(i);
                out.lock().unwrap()[i] = Some(value);
            });
        }
    });
    out.into_inner().unwrap().into_iter().map(|v| v.expect("every job ran")).collect()
}
