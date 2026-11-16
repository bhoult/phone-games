//! What exists in the world: the invaders, the structures that hold them
//! off, and the six biomes. All of it is plain data; `sim` gives it rules.

use crate::creature::Cr;

// ---- invaders -------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Insect,
    Plant,
    People,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Insect => "insects",
            Class::Plant => "plants",
            Class::People => "people",
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Foe {
    Ant,
    Beetle,
    Locust,
    Wasp,
    Moth,
    Mosquito,
    Slug,
    FireAnt,
    Scorpion,
    Creeper,
    SporePod,
    Sprout,
    Tumbleweed,
    Surveyor,
    Camper,
    Trapper,
    Worker,
    Exterminator,
    Atv,
    Mower,
    Drone,
    StagBeetle,
    Strangler,
    PumpTruck,
    Buggy,
    Chopper,
    BigYellow,
}

pub const FOE_COUNT: usize = 27;

/// What makes an invader more than a bag of health.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Quirk {
    /// Grows back this fraction of its health every second.
    Regen(f32),
    /// Bursts into this many of something smaller when shooed.
    Split(Foe, u8),
    /// Rolls or drives: cannot be held up by the pack, a dam or a decoy.
    Roll,
    /// Sprays structures within `radius` every `every` seconds, putting
    /// them out of action for `secs`.
    Jam { radius: f32, every: f32, secs: f32 },
    /// Lets out `n` of `what` every `every` seconds.
    Brood { what: Foe, every: f32, n: u8 },
}

pub struct FoeDef {
    pub name: &'static str,
    pub class: Class,
    pub hp: f32,
    /// Tiles per second.
    pub speed: f32,
    /// Taken off every hit (never below a fifth of the hit).
    pub armor: f32,
    /// Damage per second to whoever is holding it up.
    pub bite: f32,
    /// Hearts the home loses if it gets through.
    pub hearts: u8,
    pub flying: bool,
    pub boss: bool,
    /// Width on the ground, in tiles.
    pub size: f32,
    pub quirks: &'static [Quirk],
    pub blurb: &'static str,
}

const fn foe(name: &'static str, class: Class, hp: f32, speed: f32, armor: f32, bite: f32, size: f32, quirks: &'static [Quirk], blurb: &'static str) -> FoeDef {
    FoeDef { name, class, hp, speed, armor, bite, hearts: 1, flying: false, boss: false, size, quirks, blurb }
}
const fn flier(mut d: FoeDef) -> FoeDef {
    d.flying = true;
    d
}
const fn hearts(mut d: FoeDef, n: u8) -> FoeDef {
    d.hearts = n;
    d
}
const fn boss(mut d: FoeDef) -> FoeDef {
    d.boss = true;
    d.hearts = 99;
    d
}

use Class::*;

pub static FOES: [FoeDef; FOE_COUNT] = [
    foe("Ant", Insect, 22.0, 1.0, 0.0, 4.0, 0.5, &[], "Plain, plentiful, and in a hurry."),
    foe("Beetle", Insect, 70.0, 0.62, 3.0, 8.0, 0.68, &[], "Armored. Light hits bounce off."),
    foe("Locust", Insect, 30.0, 1.7, 0.0, 5.0, 0.55, &[], "Fast. Gone before slow structures notice."),
    flier(foe("Wasp", Insect, 26.0, 1.3, 0.0, 0.0, 0.5, &[], "Flies over the pack. Needs something that hits the air.")),
    flier(foe("Moth", Insect, 46.0, 1.0, 0.0, 0.0, 0.6, &[], "A big soft flier.")),
    flier(foe("Mosquito", Insect, 13.0, 1.9, 0.0, 0.0, 0.4, &[], "Tiny, fast, and never alone.")),
    hearts(foe("Slug", Insect, 170.0, 0.45, 0.0, 6.0, 0.75, &[], "Slow and very hard to discourage."), 2),
    foe("Fire Ant", Insect, 40.0, 1.5, 0.0, 6.0, 0.48, &[], "Angry, quick, and there are always more."),
    hearts(foe("Scorpion", Insect, 185.0, 0.75, 5.0, 16.0, 0.75, &[], "Thick shell, mean sting."), 2),
    foe("Creeper", Plant, 62.0, 0.8, 0.0, 6.0, 0.62, &[Quirk::Regen(0.04)], "Grows back unless it is shooed quickly."),
    foe("Spore Pod", Plant, 95.0, 0.55, 0.0, 5.0, 0.7, &[Quirk::Split(Foe::Sprout, 3)], "Bursts into three sprouts."),
    foe("Sprout", Plant, 16.0, 1.5, 0.0, 3.0, 0.4, &[], "Small, quick, freshly sprouted."),
    foe("Tumbleweed", Plant, 60.0, 2.1, 0.0, 0.0, 0.62, &[Quirk::Roll], "Rolls straight past anyone in the way."),
    foe("Surveyor", People, 115.0, 0.8, 1.0, 10.0, 0.62, &[], "Carries orange stakes. Not easily scared."),
    foe("Camper", People, 125.0, 0.9, 0.0, 8.0, 0.62, &[], "Loud, lost, and dropping wrappers."),
    hearts(foe("Trapper", People, 165.0, 0.7, 2.0, 14.0, 0.66, &[], "Heavy boots, heavy pack."), 2),
    foe("Worker", People, 145.0, 0.85, 2.0, 12.0, 0.64, &[], "Hard hat. Harder to turn around."),
    hearts(foe("Exterminator", People, 185.0, 0.7, 1.0, 10.0, 0.66, &[Quirk::Jam { radius: 1.7, every: 7.0, secs: 2.5 }], "Sprays nearby structures out of action."), 2),
    hearts(foe("Quad Bike", People, 95.0, 1.7, 2.0, 0.0, 0.8, &[Quirk::Roll], "Fast, armored, and cannot be held up."), 2),
    hearts(foe("Mower", People, 270.0, 1.1, 6.0, 20.0, 0.8, &[], "Armored, quick, and chews through whoever holds it."), 2),
    flier(foe("Drone", People, 72.0, 1.5, 1.0, 0.0, 0.55, &[], "Buzzes over everything on the ground.")),
    boss(foe("Old Mandible", Insect, 900.0, 0.42, 6.0, 30.0, 1.25, &[], "The stag beetle the others are following.")),
    boss(foe("The Strangler", Plant, 1500.0, 0.38, 2.0, 25.0, 1.4, &[Quirk::Regen(0.012), Quirk::Brood { what: Foe::Sprout, every: 5.0, n: 2 }], "A knot of vine that keeps growing.")),
    boss(foe("The Drainer", People, 2400.0, 0.4, 8.0, 30.0, 1.5, &[], "A pump truck with the marsh in its tank.")),
    boss(foe("Sandblaster", People, 2100.0, 0.75, 6.0, 0.0, 1.5, &[Quirk::Roll], "A dune buggy. Nothing can hold it up.")),
    boss(flier(foe("The Chopper", People, 2700.0, 0.5, 4.0, 0.0, 1.6, &[Quirk::Brood { what: Foe::Drone, every: 6.0, n: 2 }], "Flies, and brings drones."))),
    boss(foe("Big Yellow", People, 6000.0, 0.3, 10.0, 60.0, 1.8, &[Quirk::Brood { what: Foe::Worker, every: 7.0, n: 2 }, Quirk::Jam { radius: 2.0, every: 9.0, secs: 2.5 }], "The machine that flattened a den.")),
];

impl Foe {
    pub fn def(self) -> &'static FoeDef {
        &FOES[self as usize]
    }

    pub fn from_index(i: usize) -> Foe {
        // SAFETY-free version of a transmute: the table is in enum order.
        const ALL: [Foe; FOE_COUNT] = [
            Foe::Ant, Foe::Beetle, Foe::Locust, Foe::Wasp, Foe::Moth, Foe::Mosquito, Foe::Slug, Foe::FireAnt, Foe::Scorpion,
            Foe::Creeper, Foe::SporePod, Foe::Sprout, Foe::Tumbleweed, Foe::Surveyor, Foe::Camper, Foe::Trapper, Foe::Worker,
            Foe::Exterminator, Foe::Atv, Foe::Mower, Foe::Drone, Foe::StagBeetle, Foe::Strangler, Foe::PumpTruck, Foe::Buggy,
            Foe::Chopper, Foe::BigYellow,
        ];
        ALL[i.min(FOE_COUNT - 1)]
    }

    pub fn rolls(self) -> bool {
        self.def().quirks.iter().any(|q| matches!(q, Quirk::Roll))
    }

    /// How much of a wave's budget one of these uses up: roughly how much
    /// work it is to stop. Speed counts in full, because a structure only
    /// gets as many shots as the invader takes to cross its reach.
    pub fn threat(self) -> f32 {
        let d = self.def();
        let mut t = self.own_threat();
        for q in d.quirks {
            match *q {
                Quirk::Regen(_) => t *= 1.25,
                Quirk::Split(what, n) => t += what.threat() * n as f32,
                Quirk::Roll => t *= 1.5,
                Quirk::Jam { .. } => t *= 1.4,
                Quirk::Brood { .. } => t *= 1.3,
            }
        }
        t
    }

    /// The same, counting only this invader's own health, speed and armor.
    /// What a wave pays out is shared between its invaders by this.
    pub fn own_threat(self) -> f32 {
        let d = self.def();
        d.hp * d.speed.max(0.3) * (1.0 + d.armor * 0.15) * if d.flying { 1.5 } else { 1.0 } / 10.0
    }
}

// ---- structures -----------------------------------------------------------

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tw {
    Thistle,
    Puffball,
    HoneyPot,
    Beehive,
    AcornSling,
    NeedlePine,
    SapTrap,
    Woodpecker,
    Cattail,
    FrogPond,
    MudWallow,
    EelPool,
    Cactus,
    RockTumbler,
    DustDevil,
    RattlerDen,
    Icicle,
    Snowball,
    FrostVent,
    Eyrie,
}

pub const TOWER_COUNT: usize = 20;

/// How a structure does its job.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    /// Throws one thing at one invader.
    Shot,
    /// Lobs something that bursts over everything within `splash`.
    Lob { splash: f32 },
    /// Slows everything in range by this fraction, all the time. `dmg` is
    /// then damage per second (usually none).
    Field { slow: f32 },
    /// Sends one attacker each at up to `targets` invaders.
    Swarm { targets: u8 },
    /// Hits instantly, no flight time.
    Snipe,
    /// Hits one invader, then jumps to others nearby.
    Chain { jumps: u8 },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rider {
    /// Slows what it hits by a fraction, for some seconds.
    Chill(f32, f32),
    /// Damage per second, for some seconds.
    Poison(f32, f32),
    /// Armor stripped, for some seconds.
    Shred(f32, f32),
    /// Every `every` seconds, people in range turn and run for `secs`.
    Fear { every: f32, secs: f32 },
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Bonus {
    None,
    Vs(Class, f32),
    VsAir(f32),
}

/// What a structure gains at its third and last level.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Perk {
    /// The shot carries on into a second invader.
    Pierce,
    /// The burst leaves poison behind.
    Linger,
    /// The slowing reaches fliers too.
    SlowAir,
    /// More attackers.
    More(u8),
    /// Hits stun for this long.
    Stun(f32),
    /// Fires at two invaders at once.
    Double,
    /// Every 5s, holds everything in range still for 0.8s.
    Root,
    /// Removes insects outright below this fraction of health.
    Execute(f32),
    /// Twice the armor stripped.
    Deep,
    /// More jumps.
    Jumps(u8),
    /// Hits leave a wound: 5 a second for 3s.
    Bleed,
    /// Also strips armor.
    Shred(f32),
    /// Fear lasts a second longer and reaches everything but bosses.
    Dread,
    /// A much stronger chill.
    DeepChill,
    /// A wider burst.
    Wide,
}

/// What a shot looks like in flight; the game layer picks art from this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Look {
    Dart,
    Spore,
    Bee,
    Acorn,
    Needle,
    Peck,
    Reed,
    Tongue,
    Zap,
    Rock,
    Fang,
    Icicle,
    Snowball,
    Dive,
    Feather,
    Pebble,
    None,
}

pub struct TowerDef {
    pub name: &'static str,
    /// A few words for the build card.
    pub tag: &'static str,
    pub blurb: &'static str,
    pub cost: u32,
    pub mode: Mode,
    pub range: f32,
    /// Attacks per second.
    pub rate: f32,
    pub dmg: f32,
    pub air: bool,
    pub ground: bool,
    /// Ignores armor entirely.
    pub pierce: bool,
    pub bonus: Bonus,
    pub riders: &'static [Rider],
    pub perk: Perk,
    pub perk_text: &'static str,
    pub look: Look,
}

/// Damage, range and rate at levels 1 to 3.
pub const LV_DMG: [f32; 3] = [1.0, 1.55, 2.3];
pub const LV_RANGE: [f32; 3] = [1.0, 1.07, 1.14];
pub const LV_RATE: [f32; 3] = [1.0, 1.1, 1.2];
/// A slowing field gets this much stronger per level.
pub const LV_SLOW: f32 = 0.07;
/// What levels 2 and 3 cost, as a share of the building price.
pub const LV_COST: [f32; 3] = [1.0, 0.75, 1.25];
/// Share of everything spent that selling gives back.
pub const SELL_BACK: f32 = 0.7;

#[allow(clippy::too_many_arguments)]
const fn tw(
    name: &'static str,
    tag: &'static str,
    blurb: &'static str,
    cost: u32,
    mode: Mode,
    range: f32,
    rate: f32,
    dmg: f32,
    air: bool,
    look: Look,
    perk: Perk,
    perk_text: &'static str,
) -> TowerDef {
    TowerDef { name, tag, blurb, cost, mode, range, rate, dmg, air, ground: true, pierce: false, bonus: Bonus::None, riders: &[], perk, perk_text, look }
}
const fn with(mut d: TowerDef, riders: &'static [Rider]) -> TowerDef {
    d.riders = riders;
    d
}
const fn vs(mut d: TowerDef, bonus: Bonus) -> TowerDef {
    d.bonus = bonus;
    d
}
const fn piercing(mut d: TowerDef) -> TowerDef {
    d.pierce = true;
    d
}

pub static TOWERS: [TowerDef; TOWER_COUNT] = [
    // ---- the meadow ----
    tw("Thistle Shooter", "quick darts, hits air", "Flicks thistle darts at one invader at a time.", 50, Mode::Shot, 2.3, 1.4, 9.0, true, Look::Dart, Perk::Pierce, "Darts carry on into a second invader."),
    tw("Puffball", "bursts over a crowd", "Lobs a puffball that bursts over everything near where it lands.", 70, Mode::Lob { splash: 0.9 }, 2.2, 0.55, 16.0, false, Look::Spore, Perk::Linger, "Spores linger: 6 damage a second for 3s."),
    tw("Honey Pot", "slows the ground", "Sticky ground. Everything walking past slows right down.", 45, Mode::Field { slow: 0.35 }, 1.5, 0.0, 0.0, false, Look::None, Perk::SlowAir, "Sticky enough to slow fliers too."),
    vs(tw("Beehive", "many stings, hits air", "Sends a bee at each of three invaders. Fliers hate it.", 80, Mode::Swarm { targets: 3 }, 1.9, 1.0, 6.0, true, Look::Bee, Perk::More(2), "Two more bees."), Bonus::VsAir(1.5)),
    // ---- the old wood ----
    tw("Acorn Sling", "bursts over a crowd", "Flings an acorn that cracks over everything nearby.", 70, Mode::Lob { splash: 0.85 }, 2.4, 0.6, 18.0, false, Look::Acorn, Perk::Stun(0.4), "Acorns leave invaders dizzy for a moment."),
    tw("Needle Pine", "very fast, hits air", "A hail of light needles. Weak against armor.", 55, Mode::Shot, 2.1, 2.6, 5.0, true, Look::Needle, Perk::Double, "Rakes two invaders at once."),
    tw("Sap Trap", "slows the ground", "Dripping sap gums up everything that walks through.", 45, Mode::Field { slow: 0.4 }, 1.5, 0.0, 0.0, false, Look::None, Perk::Root, "Every 5s, roots hold everything in reach for 0.8s."),
    vs(piercing(tw("Woodpecker Roost", "far, hard, ignores armor", "One hard peck from a long way off. Armor does not help.", 90, Mode::Snipe, 3.3, 0.45, 42.0, true, Look::Peck, Perk::Stun(0.6), "A solid peck stuns for 0.6s.")), Bonus::Vs(Insect, 1.5)),
    // ---- the marsh ----
    tw("Cattail Darts", "quick darts, hits air", "Reed darts at one invader at a time.", 50, Mode::Shot, 2.4, 1.3, 10.0, true, Look::Reed, Perk::Pierce, "Darts carry on into a second invader."),
    vs(tw("Frog Pond", "eats insects, hits air", "A long tongue. Double damage to insects.", 80, Mode::Snipe, 1.9, 0.8, 20.0, true, Look::Tongue, Perk::Execute(0.3), "Swallows insects below 30% health whole."), Bonus::Vs(Insect, 2.0)),
    with(tw("Mud Wallow", "slows, softens armor", "Deep mud. Slows the ground and strips 3 armor.", 55, Mode::Field { slow: 0.35 }, 1.5, 0.0, 0.0, false, Look::None, Perk::Deep, "Thicker mud: strips twice the armor."), &[Rider::Shred(3.0, 1.0)]),
    piercing(tw("Eel Pool", "jumps between invaders", "A jolt that leaps to two more invaders. Ignores armor.", 95, Mode::Chain { jumps: 2 }, 2.2, 0.7, 16.0, true, Look::Zap, Perk::Jumps(2), "Leaps to two more.")),
    // ---- red rock ----
    tw("Cactus Spitter", "very fast, hits air", "Sprays spines. Weak against armor.", 55, Mode::Shot, 2.2, 2.4, 6.0, true, Look::Needle, Perk::Bleed, "Spines stick: 5 damage a second for 3s."),
    tw("Rock Tumbler", "heavy burst", "Rolls a boulder that crashes through a crowd.", 85, Mode::Lob { splash: 0.95 }, 2.5, 0.45, 30.0, false, Look::Rock, Perk::Stun(0.5), "Boulders stun for half a second."),
    tw("Dust Devil", "slows ground and air", "A whirl of grit that slows everything, fliers included, and stings a little.", 60, Mode::Field { slow: 0.3 }, 1.6, 0.0, 3.0, true, Look::None, Perk::Shred(3.0), "Sandblasts 3 armor off whatever is inside."),
    vs(with(tw("Rattler Den", "scares people off", "A venomous bite, and a rattle that sends people running.", 90, Mode::Shot, 2.0, 0.9, 14.0, false, Look::Fang, Perk::Dread, "The rattle lasts longer and scares everything but bosses."), &[Rider::Poison(6.0, 3.0), Rider::Fear { every: 6.0, secs: 1.6 }]), Bonus::Vs(People, 1.5)),
    // ---- high crag ----
    with(tw("Icicle Thrower", "chills, hits air", "Icicles that slow what they hit.", 60, Mode::Shot, 2.4, 1.1, 12.0, true, Look::Icicle, Perk::DeepChill, "A much deeper chill."), &[Rider::Chill(0.25, 1.5)]),
    with(tw("Snowball Stack", "chilly burst", "Snowballs that burst over a crowd and chill it.", 75, Mode::Lob { splash: 1.0 }, 2.3, 0.6, 19.0, false, Look::Snowball, Perk::Wide, "Bigger snowballs, wider burst."), &[Rider::Chill(0.2, 1.5)]),
    tw("Frost Vent", "slows the ground", "Cold air pours out. Everything nearby slows to a trudge.", 60, Mode::Field { slow: 0.42 }, 1.5, 0.0, 0.0, false, Look::None, Perk::Root, "Every 5s, freezes everything in reach for 0.8s."),
    vs(piercing(tw("Eagle Eyrie", "far, hard, hits air", "An eagle stoops from a long way off. Double damage to fliers.", 100, Mode::Snipe, 3.5, 0.5, 46.0, true, Look::Dive, Perk::Double, "A second eagle: strikes two at once.")), Bonus::VsAir(2.0)),
];

impl Tw {
    pub fn def(self) -> &'static TowerDef {
        &TOWERS[self as usize]
    }

    /// Price of level `to` (1 builds it, 2 and 3 upgrade it), before discounts.
    pub fn price(self, to: u8) -> u32 {
        let raw = self.def().cost as f32 * LV_COST[(to as usize).clamp(1, 3) - 1];
        ((raw / 5.0).round() * 5.0) as u32
    }
}

// ---- biomes ---------------------------------------------------------------

pub struct BiomeDef {
    pub name: &'static str,
    /// What the creature who lives here calls home.
    pub home: &'static str,
    pub resident: Option<Cr>,
    pub towers: &'static [Tw],
}

pub const BIOME_COUNT: usize = 6;

pub static BIOMES: [BiomeDef; BIOME_COUNT] = [
    BiomeDef { name: "The Meadow", home: "warren", resident: Some(Cr::Rabbit), towers: &[Tw::Thistle, Tw::Puffball, Tw::HoneyPot, Tw::Beehive] },
    BiomeDef { name: "The Old Wood", home: "oak", resident: Some(Cr::Squirrel), towers: &[Tw::NeedlePine, Tw::AcornSling, Tw::SapTrap, Tw::Woodpecker] },
    BiomeDef { name: "Reedmere", home: "lodge", resident: Some(Cr::Beaver), towers: &[Tw::Cattail, Tw::FrogPond, Tw::MudWallow, Tw::EelPool] },
    BiomeDef { name: "Red Rock", home: "burrow", resident: Some(Cr::Tortoise), towers: &[Tw::Cactus, Tw::RockTumbler, Tw::DustDevil, Tw::RattlerDen] },
    BiomeDef { name: "High Crag", home: "den", resident: Some(Cr::Marmot), towers: &[Tw::Icicle, Tw::Snowball, Tw::FrostVent, Tw::Eyrie] },
    // The last stand borrows one structure from each friend's home.
    BiomeDef { name: "Road's End", home: "den", resident: None, towers: &[Tw::Beehive, Tw::AcornSling, Tw::MudWallow, Tw::RattlerDen, Tw::Eyrie] },
];
