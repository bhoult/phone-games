//! Plays every stage with the stand-in player and reports how it went.
//!
//!   cargo run -p biome-core --release --example balance          the report
//!   cargo run -p biome-core --release --example balance tune     size every
//!       wave and boss, and print `TUNE` lines for tools/apply_tune.py to
//!       write into core/src/level.rs
//!   cargo run -p biome-core --release --example balance waves    who comes when
//!   cargo run -p biome-core --release --example balance pack     how much of
//!       the work the pack does, and whether it can win with nothing built
//!   cargo run -p biome-core --release --example balance carpet   whether
//!       building and never upgrading does better than the stand-in's way
//!
//! How hard the game is comes from one table, `CURVE`, below.

use biome_core::bot::{expected_profile, play, Style};
use biome_core::creature::HEROES;
use biome_core::level::{Difficulty, LEVELS, LEVEL_COUNT, PER_BIOME};
use biome_core::sim::{Phase, Setup, Sim};

/// For each biome: how much pressure a wave puts on at the start of a
/// stage and at the end of it, on Normal, as a share of the most the
/// stand-in player can stop without losing a heart. Raise a number to
/// make that stretch harder. Easy and Hard are fixed steps in invader
/// health either side of Normal (see `Difficulty::toughness`).
///
/// The stand-in player is not a person. If the game feels wrong, move
/// these numbers rather than trusting them.
const CURVE: [(f32, f32); 6] = [(0.45, 0.66), (0.50, 0.70), (0.53, 0.73), (0.56, 0.76), (0.58, 0.78), (0.60, 0.80)];

/// Seeds the stand-in is tried with. More than one, so that a stage is
/// not sized to a single lucky run.
const SEEDS: [u64; 2] = [7, 1009];

fn setup(level: usize, hero: biome_core::Cr, difficulty: Difficulty, seed: u64) -> Setup {
    let p = expected_profile(level, hero);
    Setup { level, difficulty, pack: p.pack(), guests: p.guests(level), seed }
}

/// Does the stand-in stop the first `upto` waves, at these crowd sizes,
/// without losing a heart, whichever creature it starts with? The boss is
/// made of paper here: it is sized afterwards.
fn holds(level: usize, crowd: &[f32], upto: usize) -> bool {
    HEROES.iter().all(|&hero| {
        SEEDS.iter().all(|&seed| {
            let mut sim = Sim::new(&setup(level, hero, Difficulty::Normal, seed));
            // the later waves stay on the list, so the stand-in plans for
            // them as it would in a whole stage; they just never start
            sim.set_waves(biome_core::level::waves_with(level, crowd), upto);
            sim.set_boss(0.001);
            let sim = play(sim, Style::Careful);
            // only this wave's own invaders are judged here: the waves
            // before it were sized in their own turn
            sim.phase == Phase::Won && sim.leaks[upto - 1] == 0
        })
    })
}

/// Does the stand-in win the whole stage with a boss this tough?
fn beats_boss(level: usize, crowd: &[f32], boss: f32) -> bool {
    HEROES.iter().all(|&hero| {
        SEEDS.iter().all(|&seed| {
            let mut sim = Sim::new(&setup(level, hero, Difficulty::Normal, seed));
            sim.set_waves(biome_core::level::waves_with(level, crowd), usize::MAX);
            sim.set_boss(boss);
            play(sim, Style::Careful).phase == Phase::Won
        })
    })
}

/// The largest value of a setting that still passes.
fn limit(pass: impl Fn(f32) -> bool) -> f32 {
    let (mut lo, mut hi) = (0.02f32, 0.25f32);
    while pass(hi) && hi < 30.0 {
        lo = hi;
        hi *= 2.0;
    }
    for _ in 0..9 {
        let mid = (lo + hi) / 2.0;
        if pass(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

/// How tough a boss is, as a share of the toughest the stand-in can beat
/// once the waves around it are set.
const BOSS_SHARE: f32 = 0.6;

/// Plays every stage both ways: the stand-in's (build what the stage is
/// sized for, then upgrade) and carpeting (spend everything on more
/// level-one structures). Hearts left, averaged over the three creatures.
fn carpet() {
    println!("Hearts kept (of the home's own), stand-in's way against never upgrading, by difficulty.\n");
    println!("{:<22}{:>22}{:>22}", "stage", "Normal  tend / carpet", "Hard  tend / carpet");
    let mut wins = [[0u32; 2]; 2];
    let mut built = [0.0f32; 2];
    for level in 0..LEVEL_COUNT {
        let mut row = format!("{:>2} {:<19}", level + 1, LEVELS[level].name);
        for (di, d) in [Difficulty::Normal, Difficulty::Hard].into_iter().enumerate() {
            let mut cell = String::new();
            for (si, style) in [Style::Careful, Style::Carpet].into_iter().enumerate() {
                let mut kept = 0.0;
                for &hero in &HEROES {
                    let sim = play(Sim::new(&setup(level, hero, d, 7)), style);
                    kept += sim.hearts.max(0) as f32 / sim.max_hearts as f32 / 3.0;
                    wins[di][si] += (sim.phase == Phase::Won) as u32;
                    if di == 0 {
                        built[si] += sim.towers.iter().flatten().count() as f32 / (3 * LEVEL_COUNT) as f32;
                    }
                }
                cell += &format!("{}{:>4.0}%", if si == 0 { "" } else { " /" }, kept * 100.0);
            }
            row += &format!("{cell:>22}");
        }
        println!("{row}");
    }
    println!("\nstages won of {}: Normal {} tending, {} carpeting; Hard {} tending, {} carpeting", LEVEL_COUNT * 3, wins[0][0], wins[0][1], wins[1][0], wins[1][1]);
    println!("structures standing at the end, on average: {:.1} tending, {:.1} carpeting", built[0], built[1]);
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("carpet") {
        carpet();
        return;
    }
    let tune = std::env::args().nth(1).as_deref() == Some("tune");
    if tune {
        // Each wave in turn: find the biggest crowd the stand-in can stop
        // with the waves before it already settled, and keep a share.
        for level in 0..LEVEL_COUNT {
            let n = LEVELS[level].waves as usize;
            let (a, b) = CURVE[level / PER_BIOME];
            let mut crowd = vec![1.0f32; n];
            for w in 0..n {
                let share = a + (b - a) * w as f32 / (n - 1) as f32;
                let lim = limit(|m| {
                    let mut trial = crowd.clone();
                    trial[w] = m;
                    holds(level, &trial, w + 1)
                });
                crowd[w] = (lim * share * 100.0).round() / 100.0;
            }
            let boss = if LEVELS[level].boss.is_some() { (limit(|x| beats_boss(level, &crowd, x)) * BOSS_SHARE * 100.0).round() / 100.0 } else { 1.0 };
            let list: Vec<String> = crowd.iter().map(|c| format!("{c:.2}")).collect();
            println!("{:2} {:18} boss {:5.2}  crowd {}", level + 1, LEVELS[level].name, boss, list.join(" "));
            eprintln!("TUNE {} {:.2} {}", level, boss, list.join(", "));
        }
        return;
    }

    if std::env::args().nth(1).as_deref() == Some("why") {
        // why <stage> [easy|normal|hard] [careful|idle]: who got through, and when
        let level: usize = std::env::args().nth(2).and_then(|a| a.parse().ok()).unwrap_or(1) - 1;
        let d = match std::env::args().nth(3).as_deref() {
            Some("easy") => Difficulty::Easy,
            Some("hard") => Difficulty::Hard,
            _ => Difficulty::Normal,
        };
        let style = if std::env::args().nth(4).as_deref() == Some("idle") { Style::Idle } else { Style::Careful };
        for &hero in &HEROES {
            let mut sim = Sim::new(&setup(level, hero, d, 7));
            let mut bot = biome_core::bot::Bot::new(style);
            let mut leaks: Vec<String> = Vec::new();
            while sim.playing() && sim.time < 1200.0 {
                bot.act(&mut sim);
                sim.tick();
                for e in sim.events.drain(..) {
                    if let biome_core::Ev::Leak { foe, .. } = e {
                        leaks.push(format!("{} (wave {}, {:.0}s)", foe.def().name, sim.wave, sim.time));
                    }
                }
            }
            let built: Vec<String> = sim.towers.iter().flatten().map(|t| format!("{}{}", t.kind.def().name.split(' ').next().unwrap(), t.level)).collect();
            println!("{:?}: {:?} {}/{} hearts, pack did {:.0}%, built [{}]", hero, sim.phase, sim.hearts, sim.max_hearts, 100.0 * sim.dealt[1] / (sim.dealt[0] + sim.dealt[1]).max(1.0), built.join(" "));
            println!("    pack: {}", sim.units.iter().map(|u| format!("{}{}", u.who.def().name, if u.guest { "(guest)" } else { "" })).collect::<Vec<_>>().join(", "));
            if !leaks.is_empty() {
                println!("    got through: {}", leaks.join(", "));
            }
        }
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("waves") {
        for level in 0..LEVEL_COUNT {
            let def = &LEVELS[level];
            println!("{:2} {}", level + 1, def.name);
            for (w, wave) in biome_core::level::waves(level).iter().enumerate() {
                let who: Vec<String> = wave.tally().iter().map(|(f, n)| format!("{} {}", n, f.def().name)).collect();
                println!("     {:2}  {:4.0}s  x{:.1} health  {}", w + 1, wave.duration(), wave.tough, who.join(", "));
            }
        }
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("pack") {
        // how much of the work the pack does, and whether it can win alone
        println!("{:22} {:>24}   {}", "stage", "pack's share of damage", "doing nothing: Easy / Normal / Hard, per creature");
        for level in 0..LEVEL_COUNT {
            let mut line = format!("{:2} {:19}", level + 1, LEVELS[level].name);
            for &hero in &HEROES {
                let sim = play(Sim::new(&setup(level, hero, Difficulty::Normal, 7)), Style::Careful);
                line += &format!(" {:6.0}%", 100.0 * sim.dealt[1] / (sim.dealt[0] + sim.dealt[1]).max(1.0));
            }
            line += "     ";
            for d in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
                for &hero in &HEROES {
                    let sim = play(Sim::new(&setup(level, hero, d, 7)), Style::Idle);
                    line += if sim.phase == Phase::Won { "W" } else { "." };
                }
                line += " ";
            }
            println!("{line}");
        }
        return;
    }

    println!("Stand-in player, with the pack a player would have on reaching each stage.");
    println!("Each cell: result, hearts left of hearts at the start, stars. One column per starting creature.\n");
    println!("{:22} {:>5}  {:^26} {:^26} {:^26}  idle", "stage", "mins", "Easy", "Normal", "Hard");
    let mut bad = 0;
    for level in 0..LEVEL_COUNT {
        let def = &LEVELS[level];
        let mut line = format!("{:2} {:19}", level + 1, def.name);
        let mut mins = 0.0f32;
        let mut cells = String::new();
        for d in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
            let mut cell = String::new();
            for &hero in &HEROES {
                let sim = play(Sim::new(&setup(level, hero, d, 7)), Style::Careful);
                let won = sim.phase == Phase::Won;
                if !won && d != Difficulty::Hard {
                    bad += 1;
                }
                mins = mins.max(sim.time / 60.0);
                cell += &format!("{}{:2}/{:2}{} ", if won { 'W' } else { 'L' }, sim.hearts, sim.max_hearts, if won { (b'0' + sim.stars()) as char } else { '-' });
            }
            cells += &format!(" {:26}", cell.trim_end());
        }
        // watching and doing nothing has to lose, even on Easy
        let idle = play(Sim::new(&setup(level, HEROES[0], Difficulty::Easy, 7)), Style::Idle);
        if idle.phase == Phase::Won {
            bad += 1;
        }
        line += &format!(" {:5.1} {}  {}", mins, cells, if idle.phase == Phase::Won { "WON (bad)" } else { "lost" });
        println!("{line}");
    }
    println!("\n{}", if bad == 0 { "ok: the stand-in wins everything on Easy and Normal, and doing nothing loses everywhere".to_string() } else { format!("{bad} PROBLEMS") });
}
