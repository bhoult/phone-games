//! Difficulty tuning. Plays every level a few hundred times with the stand-in
//! players and either reports how it went or works out move budgets.
//!
//!   cargo run -p rufus-core --release --example balance [games]        report
//!   cargo run -p rufus-core --release --example balance tune [games]   suggest
//!
//! `tools/apply_tune.py` runs the second form and writes the result into
//! `level.rs` and `later.rs`. The targets it aims at are `CURVES` in
//! `src/tune.rs`.

use rufus_core::bot::{self, BotCfg, Outcome};
use rufus_core::level::{chapter_count, chapter_levels, place_of, Difficulty, CHAPTERS, CHAPTER_LEN, HAND_MADE, LEVELS};
use rufus_core::tune::{self, par_map, rate, report_seed, target};
use rufus_core::Place;

fn tune(games: u64) {
    println!("{:>4} {:<26} | casual needs {:>3} {:>3} {:>3} | {:>14} => {:>14}  paws", "#", "level", "p25", "p50", "p75", "now E/N/H", "moves E/N/H");
    let tuned = par_map(LEVELS.len(), |i| tune::tune_level(&LEVELS[i], i, games));
    for (i, (def, t)) in LEVELS.iter().zip(&tuned).enumerate() {
        let mark = |k: usize| if t.floored[k] { "*" } else { "" };
        println!(
            "{:>4} {:<26} | {:>16} {:>3} {:>3} | {:>14} => {:>14}  [{}, {}]",
            i + 1,
            def.title,
            t.casual[0],
            t.casual[1],
            t.casual[2],
            format!("{}/{}/{}", def.moves[0], def.moves[1], def.moves[2]),
            format!("{}{}/{}{}/{}", t.moves[0], mark(0), t.moves[1], mark(1), t.moves[2]),
            t.paws[0],
            t.paws[1],
        );
        eprintln!("TUNE {} {} {} {} {} {}", i, t.moves[0], t.moves[1], t.moves[2], t.paws[0], t.paws[1]);
    }
    println!("\n* budget set by a floor (sloppy or just-swiping player) rather than the curve");
}

/// How one level went on boards the tuner never saw.
struct Row {
    casual: [f32; 3],
    careful: [f32; 3],
    sloppy: [f32; 2],
    swiping: [f32; 2],
    paws: [f32; 3],
    shuffles: f32,
    look_used: f32,
    look_saves: f32,
}

fn measure(i: usize, games: u64) -> Row {
    let def = &LEVELS[i];
    let run = |cfg: &BotCfg, d: Difficulty| -> Vec<Outcome> {
        (0..games).map(|s| bot::play_with_moves(i, report_seed(s), cfg, Some(def.moves[d.index()]))).collect()
    };
    let casual = Difficulty::ALL.map(|d| run(&BotCfg::CASUAL, d));
    let careful = Difficulty::ALL.map(|d| run(&BotCfg::CAREFUL, d));
    let swiping = [run(&BotCfg::RANDOM, Difficulty::Easy), run(&BotCfg::RANDOM, Difficulty::Normal)];
    let sloppy = [run(&BotCfg::SLOPPY, Difficulty::Easy), run(&BotCfg::SLOPPY, Difficulty::Normal)];
    let normal: Vec<Outcome> = careful[1].iter().chain(casual[1].iter()).copied().collect();
    let wins: Vec<&Outcome> = normal.iter().filter(|o| o.won).collect();
    let paws = |n: u8| wins.iter().filter(|o| o.paws == n).count() as f32 / wins.len().max(1) as f32;
    let looks: Vec<&Outcome> = normal.iter().filter(|o| o.begged).collect();
    Row {
        casual: [rate(&casual[0]), rate(&casual[1]), rate(&casual[2])],
        careful: [rate(&careful[0]), rate(&careful[1]), rate(&careful[2])],
        sloppy: [rate(&sloppy[0]), rate(&sloppy[1])],
        swiping: [rate(&swiping[0]), rate(&swiping[1])],
        paws: [paws(1), paws(2), paws(3)],
        shuffles: casual[1].iter().map(|o| o.shuffles).sum::<u32>() as f32 / games as f32,
        look_used: looks.len() as f32 / normal.len() as f32,
        look_saves: looks.iter().filter(|o| o.won).count() as f32 / looks.len().max(1) as f32,
    }
}

fn mean(xs: impl Iterator<Item = f32>) -> f32 {
    let v: Vec<f32> = xs.collect();
    v.iter().sum::<f32>() / v.len().max(1) as f32
}

fn triple(f: impl Fn(usize) -> f32) -> String {
    format!("{:.0}/{:.0}/{:.0}", 100.0 * f(0), 100.0 * f(1), 100.0 * f(2))
}

fn report(games: u64) {
    println!("Chance of winning a level on one attempt, on boards the tuner never saw.");
    println!("casual: best-looking move 2 times in 3.  careful: always.  sloppy: 1 time in 4, but uses zoomies.");
    println!("swiping: random moves, no zoomies.\n");
    println!(
        "{:>4} {:<26} | {:>8} | {:>14} | {:>14} | {:>10} | {:>9} | {:>12} | {:>5} | the look (Normal)",
        "#", "level", "moves", "casual E/N/H", "careful E/N/H", "sloppy E/N", "swiping E/N", "paws 1/2/3 %", "shuf"
    );
    let rows = par_map(LEVELS.len(), |i| measure(i, games));
    let p = |x: f32| (100.0 * x).round();
    for c in 0..chapter_count() {
        for i in chapter_levels(c) {
            let (def, r) = (&LEVELS[i], &rows[i]);
            println!(
                "{:>4} {:<26} | {:>8} | {:>14} | {:>14} | {:>10} | {:>9} | {:>3.0} {:>3.0} {:>3.0}  | {:>5.2} | used {:>2.0}%, saves {:>2.0}%",
                i + 1,
                def.title,
                format!("{}/{}/{}", def.moves[0], def.moves[1], def.moves[2]),
                format!("{}/{}/{}", p(r.casual[0]), p(r.casual[1]), p(r.casual[2])),
                format!("{}/{}/{}", p(r.careful[0]), p(r.careful[1]), p(r.careful[2])),
                format!("{}/{}", p(r.sloppy[0]), p(r.sloppy[1])),
                format!("{}/{}", p(r.swiping[0]), p(r.swiping[1])),
                p(r.paws[0]),
                p(r.paws[1]),
                p(r.paws[2]),
                r.shuffles,
                100.0 * r.look_used,
                100.0 * r.look_saves,
            );
        }
        let of = |f: &dyn Fn(&Row) -> f32| mean(chapter_levels(c).map(|i| f(&rows[i])));
        println!(
            "     {:<26} | {:>8} | {:>14} | {:>14} |\n",
            format!("-- {}", CHAPTERS[c].name),
            "",
            triple(|k| of(&|r| r.casual[k])),
            triple(|k| of(&|r| r.careful[k]))
        );
    }

    // The whole game at a glance.
    let summary = |name: &str, levels: &[usize]| {
        if levels.is_empty() {
            return;
        }
        println!("{name} ({} levels)", levels.len());
        for d in Difficulty::ALL {
            let k = d.index();
            let off: Vec<f32> = levels.iter().map(|&i| rows[i].casual[k] - target(d, i)).collect();
            let budgets: Vec<u32> = levels.iter().map(|&i| LEVELS[i].moves[k]).collect();
            println!(
                "  {:<6} casual {:>3.0}% (aimed at {:>3.0}%), careful {:>3.0}% | off target by {:>4.1} points on average, {:>3} levels by more than 10, worst {:+.0} | moves {}-{}, mean {:.1}",
                format!("{d:?}"),
                100.0 * mean(levels.iter().map(|&i| rows[i].casual[k])),
                100.0 * mean(levels.iter().map(|&i| target(d, i))),
                100.0 * mean(levels.iter().map(|&i| rows[i].careful[k])),
                100.0 * mean(off.iter().map(|x| x.abs())),
                off.iter().filter(|x| x.abs() > 0.10).count(),
                100.0 * off.iter().copied().fold(0.0f32, |a, b| if b.abs() > a.abs() { b } else { a }),
                budgets.iter().min().unwrap(),
                budgets.iter().max().unwrap(),
                budgets.iter().sum::<u32>() as f32 / budgets.len() as f32,
            );
        }
        let worst = levels.iter().copied().max_by(|&a, &b| rows[a].shuffles.total_cmp(&rows[b].shuffles)).unwrap();
        println!(
            "  sloppy on Easy {:.0}%, reshuffles per game {:.2} on average, {} levels over 0.15, worst {:.2} (level {}); the look used in {:.0}% of Normal attempts",
            100.0 * mean(levels.iter().map(|&i| rows[i].sloppy[0])),
            mean(levels.iter().map(|&i| rows[i].shuffles)),
            levels.iter().filter(|&&i| rows[i].shuffles > 0.15).count(),
            rows[worst].shuffles,
            worst + 1,
            100.0 * mean(levels.iter().map(|&i| rows[i].look_used)),
        );
    };
    println!("== Summary ==");
    let hand: Vec<usize> = (0..HAND_MADE.min(LEVELS.len())).collect();
    let later: Vec<usize> = (HAND_MADE.min(LEVELS.len())..LEVELS.len()).collect();
    summary("The first day, made by hand", &hand);
    summary("Every level after it", &later);
    for place in Place::ALL {
        let here: Vec<usize> = later.iter().copied().filter(|&i| place_of(i) == place).collect();
        if !here.is_empty() {
            println!(
                "  {:<6} casual E/N/H {:>9}, careful {:>9}, across {} levels",
                format!("{place:?}"),
                triple(|k| mean(here.iter().map(|&i| rows[i].casual[k]))),
                triple(|k| mean(here.iter().map(|&i| rows[i].careful[k]))),
                here.len()
            );
        }
    }
    // By position in the chapter: every chapter should open gentler than it closes.
    for pos in 0..CHAPTER_LEN {
        let here: Vec<usize> = later.iter().copied().filter(|&i| i % CHAPTER_LEN == pos).collect();
        if !here.is_empty() {
            println!("  level {} of a chapter: casual E/N/H {:>9}", pos + 1, triple(|k| mean(here.iter().map(|&i| rows[i].casual[k]))));
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
