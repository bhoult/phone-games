//! The eighteen stages: three in each biome, the third being the stand at
//! the resident's home. A stage is a map, a cast of invaders and a few
//! numbers; the waves themselves are generated from those, the same way
//! every time.

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
        tip: "Tap a mound to build on it. Tap the ground to send your creature there.",
        map: [
            "....A....",
            "....#....",
            "...o#o...",
            "....#....",
            ".####....",
            ".#.o...T.",
            ".#.......",
            ".######..",
            "..o..o#..",
            "......#o.",
            "....###..",
            "....H....",
            ".........",
        ],
        foes: &[(Ant, 1), (Beetle, 3), (Locust, 5)],
        waves: 6,
        budget: 22.0,
        growth: 1.22,
        boss: None,
        seeds: 130,
        crowd: &[1.56, 1.71, 2.31, 2.03, 2.48, 4.86],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "Dandelion Row",
        biome: 0,
        tip: "Wasps fly over the pack. Thistles and bees can reach them.",
        map: [
            ".A.....B.",
            ".#.....#.",
            ".#o...o#.",
            ".###.###.",
            "...#o#...",
            "...###...",
            "....#....",
            "...o#o...",
            "..###....",
            "..#o.....",
            "..####...",
            "....oH...",
            ".........",
        ],
        foes: &[(Ant, 1), (Locust, 2), (Wasp, 3), (Beetle, 5)],
        waves: 7,
        budget: 28.0,
        growth: 1.2,
        boss: None,
        seeds: 140,
        crowd: &[1.72, 2.59, 0.60, 1.56, 2.22, 1.95, 2.43],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Warren",
        biome: 0,
        tip: "Old Mandible shrugs off light hits. Slow him down and hit him hard.",
        map: [
            "......A..",
            "..o...#..",
            ".######..",
            ".#..o....",
            ".#.....o.",
            ".#######.",
            "..o.o..#.",
            ".......#.",
            ".#######.",
            ".#.o..o..",
            ".###o....",
            "...#H....",
            ".........",
        ],
        foes: &[(Ant, 1), (Beetle, 2), (Wasp, 3), (Locust, 4)],
        waves: 8,
        budget: 32.0,
        growth: 1.2,
        boss: Some(StagBeetle),
        seeds: 150,
        crowd: &[1.73, 2.07, 0.77, 1.34, 1.18, 1.58, 6.11, 7.51],
        boss_tough: 1.00,
        mood: Mood::Dusk,
    },
    // ---- the old wood -----------------------------------------------------
    LevelDef {
        name: "Fern Gully",
        biome: 1,
        tip: "Creepers grow back. Shoo them quickly, or poison them: poison stops the growing.",
        map: [
            ".T.....T.",
            "A###.....",
            "..o#o....",
            "...####..",
            "....o.#..",
            ".....o#..",
            ".######..",
            ".#.o.....",
            ".#....T..",
            ".####....",
            "..o.#o...",
            "....##H..",
            ".........",
        ],
        foes: &[(Ant, 1), (Creeper, 2), (Beetle, 4), (Moth, 5)],
        waves: 7,
        budget: 34.0,
        growth: 1.2,
        boss: None,
        seeds: 150,
        crowd: &[2.21, 2.71, 2.36, 2.99, 2.23, 4.53, 4.03],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "Hollow Log",
        biome: 1,
        tip: "The trail forks and joins again. Spore pods burst into sprouts.",
        map: [
            "....A....",
            "...o#o...",
            "...###...",
            "...#o#...",
            ".###.###.",
            ".#o...o#.",
            ".#.....#.",
            ".###o###.",
            "...#.#...",
            "..o###o..",
            "....#....",
            "...oH....",
            ".........",
        ],
        foes: &[(Creeper, 1), (SporePod, 2), (Moth, 3), (Ant, 4), (Beetle, 6)],
        waves: 8,
        budget: 38.0,
        growth: 1.19,
        boss: None,
        seeds: 160,
        crowd: &[0.96, 1.21, 1.25, 1.90, 2.38, 2.56, 3.25, 4.35],
        boss_tough: 1.00,
        mood: Mood::Dawn,
    },
    LevelDef {
        name: "Hazel's Oak",
        biome: 1,
        tip: "The Strangler keeps growing back and keeps dropping sprouts. Do not let up.",
        map: [
            "..A......",
            "..#o.....",
            "..####...",
            "....o#...",
            "B##..#o..",
            "..#o.###.",
            "..#....#.",
            "..####o#.",
            "....o#.#.",
            ".....###.",
            ".....o#..",
            ".....H#..",
            ".........",
        ],
        foes: &[(Ant, 1), (Creeper, 1), (SporePod, 3), (Moth, 4), (Beetle, 5)],
        waves: 9,
        budget: 42.0,
        growth: 1.18,
        boss: Some(Strangler),
        seeds: 170,
        crowd: &[1.59, 1.50, 0.92, 1.37, 1.66, 1.90, 1.57, 0.91, 2.02],
        boss_tough: 0.63,
        mood: Mood::Dusk,
    },
    // ---- reedmere ---------------------------------------------------------
    LevelDef {
        name: "Lily Reach",
        biome: 2,
        tip: "Mosquitoes come in clouds. Frogs and eels are happy to see them.",
        map: [
            ".......A.",
            "......o#.",
            "..######.",
            "..#o.....",
            "..#.~~...",
            ".o#.~~~..",
            "..#####..",
            "....o.#o.",
            ".~~..o#..",
            ".~~.###..",
            "...o#....",
            "....#H...",
            ".........",
        ],
        foes: &[(Mosquito, 1), (Slug, 2), (Creeper, 4)],
        waves: 8,
        budget: 44.0,
        growth: 1.19,
        boss: None,
        seeds: 160,
        crowd: &[1.15, 3.38, 2.22, 0.54, 1.73, 1.86, 3.13, 3.18],
        boss_tough: 1.00,
        mood: Mood::Dawn,
    },
    LevelDef {
        name: "Orange Stakes",
        biome: 2,
        tip: "People have armor. Mud softens it, and eels ignore it.",
        map: [
            ".A.......",
            ".#o......",
            ".#####...",
            "....o#o..",
            "..~~.###B",
            "..~~o#...",
            ".....#o..",
            ".#####...",
            ".#o...~~.",
            ".####.~~.",
            "...o#....",
            "...H#....",
            ".........",
        ],
        foes: &[(Surveyor, 1), (Mosquito, 2), (Slug, 4), (Creeper, 5)],
        waves: 8,
        budget: 48.0,
        growth: 1.19,
        boss: None,
        seeds: 170,
        crowd: &[3.40, 1.78, 2.20, 3.70, 2.08, 1.29, 3.95, 3.16],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Lodge",
        biome: 2,
        tip: "The Drainer is all armor. Mud first, then everything else.",
        map: [
            "A.......B",
            "#o.....o#",
            "###...###",
            "..#.o.#..",
            "..#####..",
            "...o#o...",
            ".~~.#.~~.",
            ".~~.#o.~.",
            "..###....",
            "..#o..~~.",
            "..###....",
            "...o#H...",
            ".........",
        ],
        foes: &[(Mosquito, 1), (Surveyor, 2), (Slug, 3), (Creeper, 4)],
        waves: 9,
        budget: 52.0,
        growth: 1.18,
        boss: Some(PumpTruck),
        seeds: 180,
        crowd: &[0.82, 3.04, 4.14, 2.71, 2.71, 0.93, 2.61, 2.00, 2.74],
        boss_tough: 1.01,
        mood: Mood::Night,
    },
    // ---- red rock ---------------------------------------------------------
    LevelDef {
        name: "Dry Wash",
        biome: 3,
        tip: "Scorpions are armored. Boulders and bites get through; spines mostly do not.",
        map: [
            "..r......",
            "A####....",
            "...o#o...",
            "....####.",
            ".....o.#.",
            ".#######.",
            ".#.o.....",
            ".#....T..",
            ".######..",
            "..o..o#..",
            ".....o#..",
            "...H###..",
            ".........",
        ],
        foes: &[(FireAnt, 1), (Scorpion, 2), (Camper, 4)],
        waves: 8,
        budget: 119.0,
        growth: 1.19,
        boss: None,
        seeds: 170,
        crowd: &[5.41, 5.85, 4.62, 2.35, 3.94, 4.32, 8.26, 10.11],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "Tumbleweed Alley",
        biome: 3,
        tip: "Tumbleweeds and quad bikes cannot be held up. Slow them and hit them from the side.",
        map: [
            "..A...B..",
            "..#...#..",
            ".o#...#o.",
            "..#####..",
            "...o#o...",
            "....#....",
            "...o#o...",
            "....#....",
            "..###o...",
            "..#o.....",
            "..####...",
            "....o#H..",
            ".........",
        ],
        foes: &[(Tumbleweed, 1), (FireAnt, 2), (Camper, 3), (Atv, 5), (Scorpion, 6)],
        waves: 9,
        budget: 128.0,
        growth: 1.18,
        boss: None,
        seeds: 180,
        crowd: &[2.04, 2.87, 4.98, 1.45, 3.20, 4.47, 3.73, 2.77, 6.40],
        boss_tough: 1.00,
        mood: Mood::Dusk,
    },
    LevelDef {
        name: "Mesa's Burrow",
        biome: 3,
        tip: "Nothing can stand in front of Sandblaster. It has to be worn down on the way past.",
        map: [
            "....A....",
            "...o#....",
            "..###o...",
            "..#....rr",
            "..#o..##B",
            "..###.#..",
            "...o#o#..",
            "....###..",
            "...o#....",
            ".####o...",
            ".#.o.....",
            ".H.......",
            ".........",
        ],
        foes: &[(FireAnt, 1), (Scorpion, 2), (Tumbleweed, 3), (Camper, 4), (Atv, 5)],
        waves: 9,
        budget: 136.0,
        growth: 1.18,
        boss: Some(Buggy),
        seeds: 190,
        crowd: &[5.66, 3.53, 0.53, 2.38, 1.94, 2.96, 5.01, 1.54, 2.84],
        boss_tough: 0.39,
        mood: Mood::Day,
    },
    // ---- high crag --------------------------------------------------------
    LevelDef {
        name: "Switchback",
        biome: 4,
        tip: "The trail doubles back on itself. A mound between two bends covers both.",
        map: [
            "A........",
            "#######..",
            ".o...o#..",
            "..#####..",
            "..#.o....",
            "..#####..",
            "...o.o#..",
            "..#####..",
            "..#.o....",
            "..####...",
            "...o.#o..",
            "....H#...",
            ".........",
        ],
        foes: &[(Trapper, 1), (Beetle, 2), (Atv, 4), (Surveyor, 5)],
        waves: 9,
        budget: 99.0,
        growth: 1.18,
        boss: None,
        seeds: 190,
        crowd: &[2.62, 4.09, 3.00, 2.06, 3.90, 3.28, 3.02, 1.49, 7.81],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "Thin Air",
        biome: 4,
        tip: "Drones fly. The eyrie was built for exactly this.",
        map: [
            ".........",
            "A##...##B",
            "..#o.o#..",
            "..#.o.#o.",
            "..#####..",
            "...o#....",
            "....#o...",
            "..###....",
            "..#o..T..",
            "..#####..",
            ".....o#..",
            "....H##..",
            ".........",
        ],
        foes: &[(Drone, 1), (Trapper, 2), (Atv, 4), (Beetle, 5)],
        waves: 9,
        budget: 105.0,
        growth: 1.18,
        boss: None,
        seeds: 190,
        crowd: &[1.31, 2.59, 1.42, 3.03, 3.34, 1.59, 2.22, 2.81, 2.71],
        boss_tough: 1.00,
        mood: Mood::Dawn,
    },
    LevelDef {
        name: "The Boulder Den",
        biome: 4,
        tip: "The Chopper flies. Only structures and creatures that hit the air can touch it.",
        map: [
            "...A.....",
            "..o#.....",
            "...####..",
            "....o.#..",
            "B###..#o.",
            "..o#.o#..",
            "...####..",
            "..o#.....",
            "...#o....",
            "...####..",
            "....o.#..",
            "...H###..",
            ".........",
        ],
        foes: &[(Trapper, 1), (Drone, 2), (Atv, 3), (Surveyor, 4)],
        waves: 10,
        budget: 108.0,
        growth: 1.17,
        boss: Some(Chopper),
        seeds: 200,
        crowd: &[3.54, 1.66, 1.59, 2.62, 1.80, 2.02, 1.37, 2.06, 3.33, 2.41],
        boss_tough: 0.60,
        mood: Mood::Dusk,
    },
    // ---- road's end -------------------------------------------------------
    LevelDef {
        name: "Stump Field",
        biome: 5,
        tip: "Your friends each lent a structure. Rattlers send people running.",
        map: [
            ".A.....B.",
            ".#o...o#.",
            ".##...##.",
            "..#.o.#..",
            "..#####..",
            "...o#....",
            "....#o...",
            ".####....",
            ".#o...T..",
            ".#####...",
            "....o#o..",
            "....H#...",
            ".........",
        ],
        foes: &[(Worker, 1), (FireAnt, 2), (Mower, 4), (Drone, 5)],
        waves: 9,
        budget: 117.0,
        growth: 1.18,
        boss: None,
        seeds: 200,
        crowd: &[2.01, 1.53, 1.82, 4.08, 1.35, 3.46, 1.62, 1.37, 3.39],
        boss_tough: 1.00,
        mood: Mood::Day,
    },
    LevelDef {
        name: "The Spray Line",
        biome: 5,
        tip: "Exterminators spray structures out of action. Stop them before they get close.",
        map: [
            "..o...o..",
            ".#######B",
            ".#..o....",
            ".#.......",
            ".#######.",
            "..o...o#.",
            ".......#.",
            ".#######.",
            ".#..o....",
            ".#.o.....",
            ".#####...",
            "....o#H..",
            ".........",
        ],
        foes: &[(Worker, 1), (Exterminator, 2), (Creeper, 3), (Drone, 4), (Mower, 6)],
        waves: 10,
        budget: 123.0,
        growth: 1.17,
        boss: None,
        seeds: 210,
        crowd: &[1.59, 2.03, 2.34, 1.09, 2.84, 1.37, 2.92, 0.77, 2.62, 1.63],
        boss_tough: 1.00,
        mood: Mood::Dusk,
    },
    LevelDef {
        name: "Home",
        biome: 5,
        tip: "This is where it started. Everyone you helped has come to help you.",
        map: [
            "..A...B..",
            "..#o.o#..",
            "..#...#..",
            "..###.#..",
            "...o#o#o.",
            "....###..",
            "...o#o...",
            "....#....",
            ".####o...",
            ".#o......",
            ".#####...",
            "...o.#H..",
            ".........",
        ],
        foes: &[(Worker, 1), (Mower, 2), (Exterminator, 3), (Drone, 4), (Atv, 5), (FireAnt, 6)],
        waves: 10,
        budget: 132.0,
        growth: 1.17,
        boss: Some(BigYellow),
        seeds: 220,
        crowd: &[3.55, 5.53, 2.66, 0.18, 5.13, 5.07, 3.49, 3.47, 8.51, 2.90],
        boss_tough: 0.62,
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
