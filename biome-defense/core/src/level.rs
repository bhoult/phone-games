//! The eighteen stages: three in each biome, the third being the stand at
//! the resident's home. A stage is a map, a cast of invaders and a few
//! numbers; the waves themselves are generated from those, the same way
//! every time.

// The wave sizes below are written by the tuner. Now and then one of them
// lands on 3.14, and the linter takes it for a badly typed pi.
#![allow(clippy::approx_constant)]

use crate::defs::Foe;
use crate::map::{Map, ROWS};
use crate::rng::Rng;

pub const LEVEL_COUNT: usize = 18;
pub const PER_BIOME: usize = 3;

/// The light a stage is played in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    Day,
    Dawn,
    Dusk,
    Night,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl Difficulty {
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn from_index(i: usize) -> Difficulty {
        [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard][i.min(2)]
    }
    pub fn name(self) -> &'static str {
        ["Easy", "Normal", "Hard"][self.index()]
    }
    /// How tough invaders are, against a stage's own setting.
    pub fn toughness(self) -> f32 {
        [0.75, 1.0, 1.3][self.index()]
    }
    /// Hearts the home starts with.
    pub fn hearts(self) -> i32 {
        [14, 10, 8][self.index()]
    }
    pub fn easier(self) -> Option<Difficulty> {
        match self {
            Difficulty::Easy => None,
            Difficulty::Normal => Some(Difficulty::Easy),
            Difficulty::Hard => Some(Difficulty::Normal),
        }
    }
}

pub struct LevelDef {
    pub name: &'static str,
    pub biome: usize,
    pub tip: &'static str,
    pub map: [&'static str; ROWS],
    /// How many structures the stage's income is sized around: everything
    /// it pays out comes to a share of what building and fully upgrading
    /// this many would cost. Structures can stand anywhere open; this only
    /// says how rich the stage is.
    pub plots: u8,
    /// Who comes, and the wave (counting from 1) each first appears in.
    pub foes: &'static [(Foe, u8)],
    pub waves: u8,
    /// How much trouble the first wave is; each later wave is `growth`
    /// times the one before. See [`Foe::threat`].
    pub budget: f32,
    pub growth: f32,
    /// Arrives with the last wave.
    pub boss: Option<Foe>,
    /// Seeds to start with.
    pub seeds: u32,
    /// How much pressure each wave puts on, against what `budget` and
    /// `growth` alone would give, and how tough the boss is. A wave at 4
    /// has twice the invaders, each twice as tough, in the same time.
    /// Both are set by the balance tool (`tools/apply_tune.py`), not by
    /// hand.
    pub crowd: &'static [f32],
    pub boss_tough: f32,
    pub mood: Mood,
}

use Foe::*;

pub static LEVELS: [LevelDef; LEVEL_COUNT] = [
    // ---- the meadow -------------------------------------------------------
    LevelDef {
        name: "First Burrow",
        biome: 0,
        tip: "Build beside the trail. Send your creature onto it to hold invaders where the structures can reach them.",
        map: [
            "....A....",
            "....#....",
            "....#....",
            "....#....",
            ".####....",
            ".#.....T.",
            ".#.......",
            ".######..",
            "......#..",
            "......#..",
            "....###..",
            "....H....",
            ".........",
        ],
        plots: 6,
        foes: &[(Ant, 1), (Beetle, 3), (Locust, 5)],
        waves: 6,
        budget: 22.0,
        growth: 1.22,
        boss: None,
        seeds: 130,
        crowd: &[1.61, 1.81, 3.95, 3.49, 4.60, 6.59],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "Long Grass",
        biome: 0,
        tip: "Wasps fly over the pack. Shrikes and bees can reach them.",
        map: [
            ".A.....B.",
            ".#.....#.",
            ".#.....#.",
            ".###.###.",
            "...#.#...",
            "...###...",
            "....#....",
            "....#....",
            "..###....",
            "..#......",
            "..####...",
            ".....H...",
            ".........",
        ],
        plots: 7,
        foes: &[(Ant, 1), (Locust, 2), (Wasp, 3), (Beetle, 5)],
        waves: 7,
        budget: 28.0,
        growth: 1.2,
        boss: None,
        seeds: 140,
        crowd: &[1.79, 1.93, 1.03, 1.25, 2.11, 1.95, 3.23],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Warren",
        biome: 0,
        tip: "Old Mandible shrugs off light hits. Slow him, then hit him hard.",
        map: [
            "......A..",
            "......#..",
            ".######..",
            ".#.......",
            ".#.......",
            ".#######.",
            ".......#.",
            ".......#.",
            ".#######.",
            ".#.......",
            ".###.....",
            "...#H....",
            ".........",
        ],
        plots: 8,
        foes: &[(Ant, 1), (Beetle, 2), (Wasp, 3), (Locust, 4)],
        waves: 8,
        budget: 32.0,
        growth: 1.2,
        boss: Some(StagBeetle),
        seeds: 150,
        crowd: &[1.72, 1.71, 0.35, 1.59, 1.77, 2.75, 4.40, 7.12],
        boss_tough: 1.18,
        mood: Mood::Dusk,
    },
    // ---- the old wood -----------------------------------------------------
    LevelDef {
        name: "Fern Gully",
        biome: 1,
        tip: "Kudzu grows back. Kill it quickly, or poison it: poison stops the growing.",
        map: [
            ".T.....T.",
            "A###.....",
            "...#.....",
            "...####..",
            "......#..",
            "......#..",
            ".######..",
            ".#.......",
            ".#....T..",
            ".####....",
            "....#....",
            "....##H..",
            ".........",
        ],
        plots: 7,
        foes: &[(Ant, 1), (Creeper, 2), (Beetle, 4), (Moth, 5)],
        waves: 7,
        budget: 34.0,
        growth: 1.2,
        boss: None,
        seeds: 150,
        crowd: &[1.84, 2.06, 2.37, 3.97, 3.10, 3.87, 5.25],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Deadfall",
        biome: 1,
        tip: "The trail forks and joins again. Seed pods burst into runners.",
        map: [
            "....A....",
            "....#....",
            "...###...",
            "...#.#...",
            ".###.###.",
            ".#.....#.",
            ".#.....#.",
            ".###.###.",
            "...#.#...",
            "...###...",
            "....#....",
            "....H....",
            ".........",
        ],
        plots: 9,
        foes: &[(Creeper, 1), (SporePod, 2), (Moth, 3), (Ant, 4), (Beetle, 6)],
        waves: 8,
        budget: 38.0,
        growth: 1.19,
        boss: None,
        seeds: 160,
        crowd: &[1.23, 1.39, 1.07, 1.38, 2.20, 2.07, 3.31, 4.88],
        boss_tough: 1.00,
        mood: Mood::Dawn,
    },
    LevelDef {
        name: "Hazel's Oak",
        biome: 1,
        tip: "The Strangler keeps growing and keeps shedding runners. Do not let up.",
        map: [
            "..A......",
            "..#......",
            "..####...",
            ".....#...",
            "B##..#...",
            "..#..###.",
            "..#....#.",
            "..####.#.",
            ".....#.#.",
            ".....###.",
            "......#..",
            ".....H#..",
            ".........",
        ],
        plots: 7,
        foes: &[(Ant, 1), (Creeper, 1), (SporePod, 3), (Moth, 4), (Beetle, 5)],
        waves: 9,
        budget: 42.0,
        growth: 1.18,
        boss: Some(Strangler),
        seeds: 170,
        crowd: &[1.70, 1.47, 1.40, 1.76, 2.48, 1.77, 1.50, 1.27, 2.67],
        boss_tough: 0.70,
        mood: Mood::Dusk,
    },
    // ---- reedmere ---------------------------------------------------------
    LevelDef {
        name: "Low Water",
        biome: 2,
        tip: "Mosquitoes come in clouds. Frogs and eels feed well.",
        map: [
            ".......A.",
            ".......#.",
            "..######.",
            "..#......",
            "..#.~~...",
            "..#.~~~..",
            "..#####..",
            "......#..",
            ".~~...#..",
            ".~~.###..",
            "....#....",
            "....#H...",
            ".........",
        ],
        plots: 7,
        foes: &[(Mosquito, 1), (Slug, 2), (Creeper, 4)],
        waves: 8,
        budget: 44.0,
        growth: 1.19,
        boss: None,
        seeds: 160,
        crowd: &[1.02, 3.19, 0.83, 1.22, 3.66, 3.01, 2.75, 4.77],
        boss_tough: 1.00,
        mood: Mood::Dawn,
    },
    LevelDef {
        name: "Orange Stakes",
        biome: 2,
        tip: "People wear armor. Mud softens it; eels ignore it.",
        map: [
            ".A.......",
            ".#.......",
            ".#####...",
            ".....#...",
            "..~~.###B",
            "..~~.#...",
            ".....#...",
            ".#####...",
            ".#....~~.",
            ".####.~~.",
            "....#....",
            "...H#....",
            ".........",
        ],
        plots: 7,
        foes: &[(Surveyor, 1), (Mosquito, 2), (Slug, 4), (Creeper, 5)],
        waves: 8,
        budget: 48.0,
        growth: 1.19,
        boss: None,
        seeds: 170,
        crowd: &[3.40, 1.37, 2.14, 3.73, 2.12, 1.63, 5.40, 2.70],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Lodge",
        biome: 2,
        tip: "The Drainer is all armor. Mud first, then everything else.",
        map: [
            "A.......B",
            "#.......#",
            "###...###",
            "..#...#..",
            "..#####..",
            "....#....",
            ".~~.#.~~.",
            ".~~.#..~.",
            "..###....",
            "..#...~~.",
            "..###....",
            "....#H...",
            ".........",
        ],
        plots: 8,
        foes: &[(Mosquito, 1), (Surveyor, 2), (Slug, 3), (Creeper, 4)],
        waves: 9,
        budget: 52.0,
        growth: 1.18,
        boss: Some(PumpTruck),
        seeds: 180,
        crowd: &[0.92, 3.31, 4.20, 1.17, 4.39, 0.95, 3.04, 2.21, 3.66],
        boss_tough: 0.89,
        mood: Mood::Night,
    },
    // ---- red rock ---------------------------------------------------------
    LevelDef {
        name: "Dry Wash",
        biome: 3,
        tip: "Scorpions are armored. Boulders and venom get through; spines mostly do not.",
        map: [
            "..r......",
            "A####....",
            "....#....",
            "....####.",
            ".......#.",
            ".#######.",
            ".#.......",
            ".#....T..",
            ".######..",
            "......#..",
            "......#..",
            "...H###..",
            ".........",
        ],
        plots: 7,
        foes: &[(FireAnt, 1), (Scorpion, 2), (Camper, 4)],
        waves: 8,
        budget: 119.0,
        growth: 1.19,
        boss: None,
        seeds: 170,
        crowd: &[4.80, 5.97, 2.75, 4.20, 4.52, 4.06, 5.89, 8.48],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Narrows",
        biome: 3,
        tip: "Tumbleweeds and quad bikes cannot be held. Slow them and hit them as they pass.",
        map: [
            "..A...B..",
            "..#...#..",
            "..#...#..",
            "..#####..",
            "....#....",
            "....#....",
            "....#....",
            "....#....",
            "..###....",
            "..#......",
            "..####...",
            ".....#H..",
            ".........",
        ],
        plots: 9,
        foes: &[(Tumbleweed, 1), (FireAnt, 2), (Camper, 3), (Atv, 5), (Scorpion, 6)],
        waves: 9,
        budget: 128.0,
        growth: 1.18,
        boss: None,
        seeds: 180,
        crowd: &[1.61, 2.42, 4.99, 1.44, 3.38, 2.75, 2.83, 2.38, 5.25],
        boss_tough: 1.00,
        mood: Mood::Dusk,
    },
    LevelDef {
        name: "Mesa's Burrow",
        biome: 3,
        tip: "Nothing can stand in front of Sandblaster. Wear it down on the way past.",
        map: [
            "....A....",
            "....#....",
            "..###....",
            "..#....rr",
            "..#...##B",
            "..###.#..",
            "....#.#..",
            "....###..",
            "....#....",
            ".####....",
            ".#.......",
            ".H.......",
            ".........",
        ],
        plots: 8,
        foes: &[(FireAnt, 1), (Scorpion, 2), (Tumbleweed, 3), (Camper, 4), (Atv, 5)],
        waves: 9,
        budget: 136.0,
        growth: 1.18,
        boss: Some(Buggy),
        seeds: 190,
        crowd: &[4.57, 3.45, 1.52, 2.05, 2.11, 2.19, 3.28, 1.40, 1.91],
        boss_tough: 0.30,
        mood: Mood::Day,
    },
    // ---- high crag --------------------------------------------------------
    LevelDef {
        name: "Switchback",
        biome: 4,
        tip: "The trail doubles back on itself. Ground between two bends covers both.",
        map: [
            "A........",
            "#######..",
            "......#..",
            "..#####..",
            "..#......",
            "..#####..",
            "......#..",
            "..#####..",
            "..#......",
            "..####...",
            ".....#...",
            "....H#...",
            ".........",
        ],
        plots: 8,
        foes: &[(Trapper, 1), (Beetle, 2), (Atv, 4), (Surveyor, 5)],
        waves: 9,
        budget: 99.0,
        growth: 1.18,
        boss: None,
        seeds: 190,
        crowd: &[2.33, 2.05, 2.71, 0.32, 3.46, 1.62, 3.66, 1.76, 6.53],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "Thin Air",
        biome: 4,
        tip: "Drones fly. The eyrie was made for this.",
        map: [
            ".........",
            "A##...##B",
            "..#...#..",
            "..#...#..",
            "..#####..",
            "....#....",
            "....#....",
            "..###....",
            "..#...T..",
            "..#####..",
            "......#..",
            "....H##..",
            ".........",
        ],
        plots: 8,
        foes: &[(Drone, 1), (Trapper, 2), (Atv, 4), (Beetle, 5)],
        waves: 9,
        budget: 105.0,
        growth: 1.18,
        boss: None,
        seeds: 190,
        crowd: &[1.48, 2.59, 1.84, 2.56, 5.72, 2.40, 2.48, 3.22, 1.72],
        boss_tough: 1.00,
        mood: Mood::Dawn,
    },
    LevelDef {
        name: "The Boulder Den",
        biome: 4,
        tip: "The Chopper flies. Only what reaches the air can touch it.",
        map: [
            "...A.....",
            "...#.....",
            "...####..",
            "......#..",
            "B###..#..",
            "...#..#..",
            "...####..",
            "...#.....",
            "...#.....",
            "...####..",
            "......#..",
            "...H###..",
            ".........",
        ],
        plots: 8,
        foes: &[(Trapper, 1), (Drone, 2), (Atv, 3), (Surveyor, 4)],
        waves: 10,
        budget: 108.0,
        growth: 1.17,
        boss: Some(Chopper),
        seeds: 200,
        crowd: &[3.05, 1.54, 0.96, 1.64, 1.37, 3.89, 1.81, 1.96, 4.11, 1.99],
        boss_tough: 0.73,
        mood: Mood::Dusk,
    },
    // ---- road's end -------------------------------------------------------
    LevelDef {
        name: "Stump Field",
        biome: 5,
        tip: "Each friend has sent something from home. Rattlers turn people around.",
        map: [
            ".A.....B.",
            ".#.....#.",
            ".##...##.",
            "..#...#..",
            "..#####..",
            "....#....",
            "....#....",
            ".####....",
            ".#....T..",
            ".#####...",
            ".....#...",
            "....H#...",
            ".........",
        ],
        plots: 8,
        foes: &[(Worker, 1), (FireAnt, 2), (Mower, 4), (Drone, 5)],
        waves: 9,
        budget: 117.0,
        growth: 1.18,
        boss: None,
        seeds: 200,
        crowd: &[1.53, 2.13, 2.19, 3.27, 0.67, 3.20, 3.14, 0.28, 3.39],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Spray Line",
        biome: 5,
        tip: "Exterminators spray structures out of action. Stop them before they are close.",
        map: [
            ".........",
            ".#######B",
            ".#.......",
            ".#.......",
            ".#######.",
            ".......#.",
            ".......#.",
            ".#######.",
            ".#.......",
            ".#.......",
            ".#####...",
            ".....#H..",
            ".........",
        ],
        plots: 8,
        foes: &[(Worker, 1), (Exterminator, 2), (Creeper, 3), (Drone, 4), (Mower, 6)],
        waves: 10,
        budget: 123.0,
        growth: 1.17,
        boss: None,
        seeds: 210,
        crowd: &[1.51, 1.70, 1.61, 1.29, 2.06, 1.94, 3.43, 1.35, 3.23, 2.44],
        boss_tough: 1.00,
        mood: Mood::Dusk,
    },
    LevelDef {
        name: "Home",
        biome: 5,
        tip: "This is where it started. Everyone you stood with is here.",
        map: [
            "..A...B..",
            "..#...#..",
            "..#...#..",
            "..###.#..",
            "....#.#..",
            "....###..",
            "....#....",
            "....#....",
            ".####....",
            ".#.......",
            ".#####...",
            ".....#H..",
            ".........",
        ],
        plots: 10,
        foes: &[(Worker, 1), (Mower, 2), (Exterminator, 3), (Drone, 4), (Atv, 5), (FireAnt, 6)],
        waves: 10,
        budget: 132.0,
        growth: 1.17,
        boss: Some(BigYellow),
        seeds: 220,
        crowd: &[3.58, 5.11, 5.24, 0.19, 4.84, 4.04, 3.47, 4.82, 6.07, 3.99],
        boss_tough: 0.67,
        mood: Mood::Night,
    },
];

impl LevelDef {
    pub fn map(&self) -> Map {
        Map::parse(&self.map)
    }
}

/// Is this stage the stand at a resident's home (the last of its biome)?
pub fn is_finale(level: usize) -> bool {
    level % PER_BIOME == PER_BIOME - 1
}

// ---- waves ----------------------------------------------------------------

/// Some number of one kind of invader, coming in one after another.
#[derive(Clone, Debug)]
pub struct Group {
    pub foe: Foe,
    pub count: u16,
    /// Seconds between one and the next.
    pub gap: f32,
    /// Seconds after the wave starts that the first appears.
    pub delay: f32,
    pub spawn: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Wave {
    pub groups: Vec<Group>,
    /// This wave's share of everything the stage's invaders drop. Fixed by
    /// the wave's place in the stage, not by how many invaders are in it.
    pub pot: f32,
    /// Health multiplier for this wave's invaders.
    pub tough: f32,
}

impl Wave {
    /// Seconds from the wave starting to its last invader appearing.
    pub fn duration(&self) -> f32 {
        self.groups.iter().map(|g| g.delay + g.gap * g.count.saturating_sub(1) as f32).fold(0.0, f32::max)
    }

    /// How many of each kind, most numerous first.
    pub fn tally(&self) -> Vec<(Foe, u16)> {
        let mut out: Vec<(Foe, u16)> = Vec::new();
        for g in &self.groups {
            match out.iter_mut().find(|(f, _)| *f == g.foe) {
                Some((_, n)) => *n += g.count,
                None => out.push((g.foe, g.count)),
            }
        }
        out.sort_by(|a, b| b.0.def().boss.cmp(&a.0.def().boss).then(b.1.cmp(&a.1)));
        out
    }

    pub fn threat(&self) -> f32 {
        self.groups.iter().map(|g| g.foe.threat() * g.count as f32).sum()
    }
}

fn gap_for(foe: Foe) -> f32 {
    let d = foe.def();
    ((0.42 + d.size * 0.5) / d.speed.max(0.3)).clamp(0.3, 2.2)
}

/// The longest a single group takes to come in. Big crowds bunch up.
const GROUP_SECS: f32 = 30.0;

/// The waves of a stage. Generated from the stage's numbers with a fixed
/// seed, so a stage plays the same every time and on every device.
pub fn waves(level: usize) -> Vec<Wave> {
    waves_with(level, LEVELS[level].crowd)
}

/// The same, with the crowd sizes given (the balance tool searches on
/// them). Waves past the end of `crowd` are left at their plain size.
pub fn waves_with(level: usize, crowd: &[f32]) -> Vec<Wave> {
    let def = &LEVELS[level];
    let spawns = def.map.iter().flat_map(|r| r.bytes()).filter(|b| matches!(b, b'A' | b'B')).count().max(1) as u8;
    let mut rng = Rng::new(0xB10_3E00 + level as u64 * 7919);
    let mut out = Vec::with_capacity(def.waves as usize);
    // later waves pay a little more than earlier ones
    let pots: Vec<f32> = (0..def.waves as i32).map(|w| 1.1f32.powi(w)).collect();
    let pot_sum: f32 = pots.iter().sum();
    for w in 0..def.waves as usize {
        let n = w as u8 + 1;
        // pressure is split evenly between more invaders and tougher ones
        let press = crowd.get(w).copied().unwrap_or(1.0).max(0.01).sqrt();
        let budget = def.budget * def.growth.powi(w as i32) * press;
        let known: Vec<Foe> = def.foes.iter().filter(|(_, first)| *first <= n).map(|(f, _)| *f).collect();
        let fresh: Vec<Foe> = def.foes.iter().filter(|(_, first)| *first == n && n > 1).map(|(f, _)| *f).collect();
        let last = w + 1 == def.waves as usize;

        // who is in this wave, and what share of the trouble each brings
        let mut picks: Vec<(Foe, f32)> = Vec::new();
        let mut hurry = 1.0;
        if w == 0 {
            picks.push((known[0], 1.0));
        } else if !fresh.is_empty() {
            // a newcomer gets most of a wave to itself
            for f in &fresh {
                picks.push((*f, 0.6 / fresh.len() as f32));
            }
            let old: Vec<Foe> = known.iter().copied().filter(|f| !fresh.contains(f)).collect();
            picks.push((rng.pick(&old), 0.4));
        } else if w % 4 == 3 && !last {
            // every so often, a rush of whatever is fastest
            let fastest = *known.iter().max_by(|a, b| a.def().speed.partial_cmp(&b.def().speed).unwrap()).unwrap();
            picks.push((fastest, 1.0));
            hurry = 0.6;
        } else {
            let mut pool = known.clone();
            rng.shuffle(&mut pool);
            let k = if pool.len() >= 3 && rng.chance(0.5) { 3 } else { pool.len().min(2) };
            let weights: Vec<f32> = (0..k).map(|_| rng.range(0.6, 1.4)).collect();
            let total: f32 = weights.iter().sum();
            for i in 0..k {
                picks.push((pool[i], weights[i] / total));
            }
        }

        let mut wave = Wave { groups: Vec::new(), pot: pots[w] / pot_sum, tough: press };
        let mut at = 0.0f32;
        let mut side = (w % 2) as u8;
        for (foe, share) in picks {
            let count = ((budget * share / foe.threat()).round() as u16).clamp(1, 120);
            // the dice are rolled whatever the crowd size, so that resizing
            // one wave leaves the make-up of the others alone
            let jitter = rng.range(0.9, 1.15);
            let overlap = rng.chance(0.5);
            let lead = rng.range(1.5, 3.5);
            // more invaders come closer together, not over a longer time
            let mut gap = gap_for(foe) * hurry * jitter / press;
            if spawns > 1 && count >= 6 {
                // big groups come down both trails at once
                let half = count / 2;
                gap = (gap * 1.7).min(GROUP_SECS / (count - half) as f32).max(0.15);
                wave.groups.push(Group { foe, count: count - half, gap, delay: at, spawn: side % spawns });
                wave.groups.push(Group { foe, count: half, gap, delay: at + gap * 0.5, spawn: (side + 1) % spawns });
                at += if overlap { lead } else { (gap * (count - half) as f32 + 1.0).min(12.0) };
            } else {
                gap = gap.min(GROUP_SECS / count as f32).max(0.15);
                wave.groups.push(Group { foe, count, gap, delay: at, spawn: side % spawns });
                at += if overlap { lead } else { (gap * count as f32 + 1.0).min(12.0) };
            }
            side += 1;
        }
        if last {
            if let Some(boss) = def.boss {
                wave.groups.push(Group { foe: boss, count: 1, gap: 0.0, delay: 4.0, spawn: 0 });
            }
        }
        out.push(wave);
    }
    out
}
