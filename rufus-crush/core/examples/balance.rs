//! Difficulty tuning. Plays every level a few hundred times with the stand-in
//! players and either reports how it went or works out move budgets.
//!
//!   cargo run -p rufus-core --release --example balance [games]        report
//!   cargo run -p rufus-core --release --example balance tune [games]   suggest
//!
//! `tools/apply_tune.py` runs the second form and writes the result into
//! `level.rs`.

use rufus_core::bot::{self, BotCfg, Outcome};
use rufus_core::level::{Difficulty, CHAPTERS, CHAPTER_LEN, LEVELS};

/// THE DIFFICULTY KNOB. For each setting and each chapter: the chance that a
/// casual player (one who takes the best-looking move about two times in
/// three) wins a level on any one attempt, at the chapter's first level and
/// at its last. Levels in between are spread evenly. Each chapter opens
/// gentler than the one before closed, because it is teaching something new.
///
/// Raise a number to make that stretch easier, lower it to make it harder,
/// then run `tools/apply_tune.py`. These describe a stand-in player, not a
/// person: if a setting feels wrong to you, move its whole row.
#[rustfmt::skip]
const CURVES: [[(f32, f32); 6]; 3] = [
    // Backyard      Dog Park      The Lake      Mud Season    Delivery Day  Night Patrol
    [(0.995, 0.97), (0.97, 0.93), (0.96, 0.92), (0.95, 0.90), (0.94, 0.88), (0.93, 0.85)], // Easy
    [(0.98,  0.88), (0.90, 0.78), (0.87, 0.75), (0.84, 0.71), (0.80, 0.67), (0.76, 0.62)], // Normal
    [(0.92,  0.66), (0.68, 0.46), (0.64, 0.46), (0.58, 0.42), (0.52, 0.36), (0.46, 0.30)], // Hard
];

fn target(d: Difficulty, level: usize) -> f32 {
    let (open, close) = CURVES[d.index()][level / CHAPTER_LEN];
    open + (close - open) * (level % CHAPTER_LEN) as f32 / (CHAPTER_LEN - 1) as f32
}

/// Easy has to be easy for everybody, not only for the casual stand-in. This
/// is how often a sloppy player (mostly swiping at whatever matches, but
/// using the zoomies when they fill) should get through on Easy.
fn sloppy_floor(d: Difficulty, level: usize) -> f32 {
    match d {
        Difficulty::Easy => 0.90 - 0.03 * (level / CHAPTER_LEN) as f32,
        _ => 0.0,
    }
}

/// The first chapter also has to work for someone who is just swiping at
/// things and has not found the zoomies meter yet. This is how often that
/// player (random legal moves, no zoomies) should get through.
fn swiping_floor(d: Difficulty, level: usize) -> f32 {
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
fn tune_seed(s: u64) -> u64 {
    s * 31 + 5
}
fn report_seed(s: u64) -> u64 {
    s * 7919 + 104_729
}

fn pct(sorted: &[u32], q: f32) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[((sorted.len() - 1) as f32 * q) as usize]
}

fn rate(v: &[Outcome]) -> f32 {
    v.iter().filter(|o| o.won).count() as f32 / v.len().max(1) as f32
}

fn games_at(level: usize, cfg: &BotCfg, budget: u32, games: u64) -> Vec<Outcome> {
    (0..games).map(|s| bot::play_with_moves(level, tune_seed(s), cfg, Some(budget))).collect()
}

const LOTS: u32 = 150;

/// How many moves a win takes this player when moves are no object.
fn moves_needed(level: usize, cfg: &BotCfg, games: u64) -> Vec<u32> {
    let mut needed: Vec<u32> = games_at(level, cfg, LOTS, games)
        .iter()
        .map(|o| if o.won { o.moves_used } else { LOTS })
        .collect();
    needed.sort();
    needed
}

/// The smallest move budget with which this player wins at least `want` of
/// the time.
fn budget_for(level: usize, cfg: &BotCfg, want: f32, games: u64, needed: &[u32]) -> u32 {
    if want <= 0.0 {
        return 3;
    }
    // The percentile is nearly the answer. It is not exactly it, because of
    // what can still happen after the last move: zoomies, and the look.
    let mut b = pct(needed, want).clamp(3, LOTS);
    while b > 3 && rate(&games_at(level, cfg, b - 1, games)) >= want {
        b -= 1;
    }
    while b < LOTS && rate(&games_at(level, cfg, b, games)) < want {
        b += 1;
    }
    b
}

fn tune(games: u64) {
    println!("{:>2} {:<24} | casual needs {:>3} {:>3} {:>3} | {:>14} => {:>14}  paws", "#", "level", "p25", "p50", "p75", "now E/N/H", "moves E/N/H");
    for (i, def) in LEVELS.iter().enumerate() {
        let casual = moves_needed(i, &BotCfg::CASUAL, games);
        let random = moves_needed(i, &BotCfg::RANDOM, games);
        let sloppy = moves_needed(i, &BotCfg::SLOPPY, games);
        let mut moves = [0u32; 3];
        let mut floored = [false; 3];
        for d in Difficulty::ALL {
            let by_target = budget_for(i, &BotCfg::CASUAL, target(d, i), games, &casual);
            let by_floor = budget_for(i, &BotCfg::RANDOM, swiping_floor(d, i), games, &random)
                .max(budget_for(i, &BotCfg::SLOPPY, sloppy_floor(d, i), games, &sloppy));
            moves[d.index()] = by_target.max(by_floor);
            floored[d.index()] = by_floor > by_target;
        }
        // an easier setting never gets fewer moves
        moves[1] = moves[1].max(moves[2]);
        moves[0] = moves[0].max(moves[1]);
        // Paw thresholds from the scores both kinds of player finish on at Normal.
        let mut scores: Vec<u32> = [BotCfg::CAREFUL, BotCfg::CASUAL]
            .iter()
            .flat_map(|cfg| games_at(i, cfg, moves[1], games))
            .filter(|o| o.won)
            .map(|o| o.score)
            .collect();
        scores.sort();
        let round = |x: u32| (x + 50) / 100 * 100;
        let two = round(pct(&scores, 0.40));
        let three = round(pct(&scores, 0.80)).max(two + 200);
        let mark = |k: usize| if floored[k] { "*" } else { "" };
        println!(
            "{:>2} {:<24} | {:>16} {:>3} {:>3} | {:>14} => {:>14}  [{}, {}]",
            i + 1,
            def.title,
            pct(&casual, 0.25),
            pct(&casual, 0.5),
            pct(&casual, 0.75),
            format!("{}/{}/{}", def.moves[0], def.moves[1], def.moves[2]),
            format!("{}{}/{}{}/{}", moves[0], mark(0), moves[1], mark(1), moves[2]),
            two,
            three,
        );
        eprintln!("TUNE {} {} {} {} {} {}", i, moves[0], moves[1], moves[2], two, three);
    }
    println!("\n* budget set by a floor (sloppy or just-swiping player) rather than the curve");
}

fn report(games: u64) {
    println!("Chance of winning a level on one attempt, on boards the tuner never saw.");
    println!("casual: best-looking move 2 times in 3.  careful: always.  sloppy: 1 time in 4, but uses zoomies.");
    println!("swiping: random moves, no zoomies.\n");
    println!(
        "{:>2} {:<24} | {:>8} | {:>14} | {:>14} | {:>10} | {:>9} | {:>12} | {:>5} | the look (Normal)",
        "#", "level", "moves", "casual E/N/H", "careful E/N/H", "sloppy E/N", "swiping E/N", "paws 1/2/3 %", "shuf"
    );
    let mut sums = [[0.0f32; 3]; 2];
    for (i, def) in LEVELS.iter().enumerate() {
        let run = |cfg: &BotCfg, d: Difficulty| -> Vec<Outcome> {
            (0..games).map(|s| bot::play_with_moves(i, report_seed(s), cfg, Some(def.moves[d.index()]))).collect()
        };
        let casual = Difficulty::ALL.map(|d| run(&BotCfg::CASUAL, d));
        let careful = Difficulty::ALL.map(|d| run(&BotCfg::CAREFUL, d));
        let swiping = [run(&BotCfg::RANDOM, Difficulty::Easy), run(&BotCfg::RANDOM, Difficulty::Normal)];
        let sloppy = [run(&BotCfg::SLOPPY, Difficulty::Easy), run(&BotCfg::SLOPPY, Difficulty::Normal)];
        let normal: Vec<Outcome> = careful[1].iter().chain(casual[1].iter()).copied().collect();
        let wins: Vec<&Outcome> = normal.iter().filter(|o| o.won).collect();
        let paws = |n: u8| 100.0 * wins.iter().filter(|o| o.paws == n).count() as f32 / wins.len().max(1) as f32;
        let looks: Vec<&Outcome> = normal.iter().filter(|o| o.begged).collect();
        let shuf = casual[1].iter().map(|o| o.shuffles).sum::<u32>() as f32 / games as f32;
        let p = |v: &[Outcome]| (100.0 * rate(v)).round();
        println!(
            "{:>2} {:<24} | {:>8} | {:>14} | {:>14} | {:>10} | {:>9} | {:>3.0} {:>3.0} {:>3.0}  | {:>5.2} | used {:>2.0}%, saves {:>2.0}%",
            i + 1,
            def.title,
            format!("{}/{}/{}", def.moves[0], def.moves[1], def.moves[2]),
            format!("{}/{}/{}", p(&casual[0]), p(&casual[1]), p(&casual[2])),
            format!("{}/{}/{}", p(&careful[0]), p(&careful[1]), p(&careful[2])),
            format!("{}/{}", p(&sloppy[0]), p(&sloppy[1])),
            format!("{}/{}", p(&swiping[0]), p(&swiping[1])),
            paws(1),
            paws(2),
            paws(3),
            shuf,
            100.0 * looks.len() as f32 / normal.len() as f32,
            100.0 * looks.iter().filter(|o| o.won).count() as f32 / looks.len().max(1) as f32,
        );
        for k in 0..3 {
            sums[0][k] += rate(&casual[k]);
            sums[1][k] += rate(&careful[k]);
        }
        if i % CHAPTER_LEN == CHAPTER_LEN - 1 {
            let avg = |row: [f32; 3]| format!("{:.0}/{:.0}/{:.0}", 100.0 * row[0] / 6.0, 100.0 * row[1] / 6.0, 100.0 * row[2] / 6.0);
            println!("   {:<24} | {:>8} | {:>14} | {:>14} |\n", format!("-- {}", CHAPTERS[i / CHAPTER_LEN]), "", avg(sums[0]), avg(sums[1]));
            sums = [[0.0; 3]; 2];
        }
    }
}

fn main() {
    let arg = |n: usize| std::env::args().nth(n);
    if arg(1).as_deref() == Some("tune") {
        tune(arg(2).and_then(|s| s.parse().ok()).unwrap_or(400));
    } else {
        report(arg(1).and_then(|s| s.parse().ok()).unwrap_or(400));
    }
}
