//! Which picture goes with which thing, and what each biome looks like.

use crate::gfx::*;
use biome_core::creature::{Cr, Icon};
use biome_core::defs::{Look, Mode};
use biome_core::level::Mood;
use biome_core::{Foe, Tw};
use macroquad::prelude::Color;

pub fn foe_spr(f: Foe) -> Spr {
    match f {
        Foe::Ant => spr::ANT,
        Foe::Beetle => spr::BEETLE,
        Foe::Locust => spr::LOCUST,
        Foe::Wasp => spr::WASP,
        Foe::Moth => spr::MOTH,
        Foe::Mosquito => spr::MOSQUITO,
        Foe::Slug => spr::SLUG,
        Foe::FireAnt => spr::FIRE_ANT,
        Foe::Scorpion => spr::SCORPION,
        Foe::Creeper => spr::CREEPER,
        Foe::SporePod => spr::SPORE_POD,
        Foe::Sprout => spr::SPROUT,
        Foe::Tumbleweed => spr::TUMBLEWEED,
        Foe::Surveyor => spr::SURVEYOR,
        Foe::Camper => spr::CAMPER,
        Foe::Trapper => spr::TRAPPER,
        Foe::Worker => spr::WORKER,
        Foe::Exterminator => spr::EXTERMINATOR,
        Foe::Atv => spr::ATV,
        Foe::Mower => spr::MOWER,
        Foe::Drone => spr::DRONE,
        Foe::StagBeetle => spr::STAG_BEETLE,
        Foe::Strangler => spr::STRANGLER,
        Foe::PumpTruck => spr::PUMP_TRUCK,
        Foe::Buggy => spr::BUGGY,
        Foe::Chopper => spr::CHOPPER,
        Foe::BigYellow => spr::BIG_YELLOW,
    }
}

pub fn tower_spr(t: Tw) -> Spr {
    match t {
        Tw::Thistle => spr::THISTLE,
        Tw::Puffball => spr::PUFFBALL,
        Tw::HoneyPot => spr::HONEY_POT,
        Tw::Beehive => spr::BEEHIVE,
        Tw::AcornSling => spr::ACORN_SLING,
        Tw::NeedlePine => spr::NEEDLE_PINE,
        Tw::SapTrap => spr::SAP_TRAP,
        Tw::Woodpecker => spr::WOODPECKER,
        Tw::Cattail => spr::CATTAIL,
        Tw::FrogPond => spr::FROG_POND,
        Tw::MudWallow => spr::MUD_WALLOW,
        Tw::EelPool => spr::EEL_POOL,
        Tw::Cactus => spr::CACTUS,
        Tw::RockTumbler => spr::ROCK_TUMBLER,
        Tw::DustDevil => spr::DUST_DEVIL,
        Tw::RattlerDen => spr::RATTLER_DEN,
        Tw::Icicle => spr::ICICLE,
        Tw::Snowball => spr::SNOWBALL,
        Tw::FrostVent => spr::FROST_VENT,
        Tw::Eyrie => spr::EYRIE,
    }
}

/// The colour a slowing field stains the ground, if this structure is one.
pub fn field_color(t: Tw) -> Option<Color> {
    if !matches!(t.def().mode, Mode::Field { .. }) {
        return None;
    }
    Some(match t {
        Tw::HoneyPot => rgb(0xffc21a),
        Tw::SapTrap => rgb(0xe08a1a),
        Tw::MudWallow => rgb(0x6a4424),
        Tw::DustDevil => rgb(0xe8cf9a),
        _ => rgb(0xbfe8ff),
    })
}

pub fn cr_spr(c: Cr) -> Spr {
    match c {
        Cr::Fox => spr::FOX,
        Cr::Badger => spr::BADGER,
        Cr::Owl => spr::OWL,
        Cr::Rabbit => spr::RABBIT,
        Cr::Squirrel => spr::SQUIRREL,
        Cr::Beaver => spr::BEAVER,
        Cr::Tortoise => spr::TORTOISE,
        Cr::Marmot => spr::MARMOT,
    }
}

/// A colour of its own for each creature: card backs, rings, name plates.
pub fn cr_color(c: Cr) -> Color {
    match c {
        Cr::Fox => rgb(0xf0843a),
        Cr::Badger => rgb(0x8a94a8),
        Cr::Owl => rgb(0x7a6ad0),
        Cr::Rabbit => rgb(0xe8b98a),
        Cr::Squirrel => rgb(0xd9603a),
        Cr::Beaver => rgb(0xa8744a),
        Cr::Tortoise => rgb(0x8fa84a),
        Cr::Marmot => rgb(0xd9a84a),
    }
}

pub fn ability_icon(c: Cr) -> Spr {
    match c {
        Cr::Fox => spr::AB_POUNCE,
        Cr::Badger => spr::AB_SLAM,
        Cr::Owl => spr::AB_GUST,
        Cr::Rabbit => spr::AB_THUMP,
        Cr::Squirrel => spr::AB_STORM,
        Cr::Beaver => spr::AB_DAM,
        Cr::Tortoise => spr::AB_SHELL,
        Cr::Marmot => spr::AB_WHISTLE,
    }
}

pub fn skill_icon(i: Icon) -> Spr {
    match i {
        Icon::Fang => spr::SK_FANG,
        Icon::Paw => spr::SK_PAW,
        Icon::Heart => spr::SK_HEART,
        Icon::Shield => spr::SK_SHIELD,
        Icon::Wind => spr::SK_WIND,
        Icon::Seed => spr::SK_SEED,
        Icon::Star => spr::SK_STAR,
        Icon::Hammer => spr::SK_HAMMER,
        Icon::Eye => spr::SK_EYE,
        Icon::Bolt => spr::SK_BOLT,
        Icon::Flame => spr::SK_FLAME,
        Icon::Pack => spr::SK_PACK,
    }
}

/// The colour of each branch of a skill tree, left to right.
pub const BRANCH_COLORS: [Color; 3] = [rgb(0xe8504a), rgb(0x4a9ae8), rgb(0xe8b030)];

pub fn shot_spr(l: Look) -> Option<Spr> {
    Some(match l {
        Look::Dart => spr::SHOT_DART,
        Look::Spore => spr::SHOT_SPORE,
        Look::Bee => spr::SHOT_BEE,
        Look::Acorn => spr::SHOT_ACORN,
        Look::Needle => spr::SHOT_NEEDLE,
        Look::Reed => spr::SHOT_REED,
        Look::Rock => spr::SHOT_ROCK,
        Look::Fang => spr::SHOT_FANG,
        Look::Icicle => spr::SHOT_ICICLE,
        Look::Snowball => spr::SHOT_SNOWBALL,
        Look::Feather => spr::SHOT_FEATHER,
        Look::Pebble => spr::SHOT_PEBBLE,
        Look::Peck | Look::Tongue | Look::Zap | Look::Dive | Look::None => return None,
    })
}

/// The colour of the bits that fly when a shot lands.
pub fn look_color(l: Look) -> Color {
    match l {
        Look::Dart => rgb(0xd070e8),
        Look::Spore => rgb(0xf4ecc8),
        Look::Bee => rgb(0xffd23a),
        Look::Acorn => rgb(0xb07a3c),
        Look::Needle => rgb(0x6fc04a),
        Look::Peck => rgb(0xfff2c0),
        Look::Reed => rgb(0xa87444),
        Look::Tongue => rgb(0xff8aa8),
        Look::Zap => rgb(0xfff07a),
        Look::Rock => rgb(0xc8703c),
        Look::Fang => rgb(0x9be04a),
        Look::Icicle => rgb(0xbfe8ff),
        Look::Snowball => rgb(0xffffff),
        Look::Dive => rgb(0xfff2c0),
        Look::Feather => rgb(0xd9b98a),
        Look::Pebble => rgb(0xb8bcc4),
        Look::None => WHITE,
    }
}

/// What drifts through the air in a biome.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    Petals,
    Leaves,
    Fireflies,
    Dust,
    Snow,
    Smoke,
}

pub struct Palette {
    pub ground: (Color, Color),
    pub trail: Color,
    pub trail_edge: Color,
    pub trail_light: Color,
    pub water: Color,
    /// Multiplied into the pale mound art.
    pub mound: Color,
    pub tall: Spr,
    pub rock: Spr,
    pub tufts: [Spr; 2],
    pub home: Spr,
    /// The biome's own colour, for buttons, ribbons and the map.
    pub accent: Color,
    pub drift: Drift,
}

pub fn palette(biome: usize) -> Palette {
    match biome {
        0 => Palette {
            ground: (rgb(0x9bd95f), rgb(0x74bd4a)),
            trail: rgb(0xe2c184),
            trail_edge: rgb(0xa88448),
            trail_light: rgb(0xf2dba8),
            water: rgb(0x5cb8e8),
            mound: rgb(0xd6a868),
            tall: spr::DECO_MEADOW_TALL,
            rock: spr::DECO_MEADOW_ROCK,
            tufts: [spr::TUFT_MEADOW_A, spr::TUFT_MEADOW_B],
            home: spr::HOME_WARREN,
            accent: rgb(0x6cbf3a),
            drift: Drift::Petals,
        },
        1 => Palette {
            ground: (rgb(0x6aa850), rgb(0x44833e)),
            trail: rgb(0xb89264),
            trail_edge: rgb(0x6f5230),
            trail_light: rgb(0xd0ae80),
            water: rgb(0x4a9cc8),
            mound: rgb(0xa88458),
            tall: spr::DECO_FOREST_TALL,
            rock: spr::DECO_FOREST_ROCK,
            tufts: [spr::TUFT_FOREST_A, spr::TUFT_FOREST_B],
            home: spr::HOME_OAK,
            accent: rgb(0x3f8f4a),
            drift: Drift::Leaves,
        },
        2 => Palette {
            ground: (rgb(0x86c0a0), rgb(0x559a84)),
            trail: rgb(0xa89670),
            trail_edge: rgb(0x625640),
            trail_light: rgb(0xc4b48c),
            water: rgb(0x3f8fb4),
            mound: rgb(0x988c68),
            tall: spr::DECO_MARSH_TALL,
            rock: spr::DECO_MARSH_ROCK,
            tufts: [spr::TUFT_MARSH_B, spr::TUFT_MARSH_B],
            home: spr::HOME_LODGE,
            accent: rgb(0x3fa0a8),
            drift: Drift::Fireflies,
        },
        3 => Palette {
            ground: (rgb(0xedb878), rgb(0xd48f50)),
            trail: rgb(0xf6dcae),
            trail_edge: rgb(0xb4803e),
            trail_light: rgb(0xfdeecb),
            water: rgb(0x6ac0d8),
            mound: rgb(0xd07840),
            tall: spr::DECO_DESERT_TALL,
            rock: spr::DECO_DESERT_ROCK,
            tufts: [spr::TUFT_DESERT_A, spr::TUFT_DESERT_B],
            home: spr::HOME_BURROW,
            accent: rgb(0xe07a3a),
            drift: Drift::Dust,
        },
        4 => Palette {
            ground: (rgb(0xeef5fb), rgb(0xbdd3e6)),
            trail: rgb(0xaab6c6),
            trail_edge: rgb(0x6c7c92),
            trail_light: rgb(0xc6d0dc),
            water: rgb(0x8fd0f0),
            mound: rgb(0xa4b2c4),
            tall: spr::DECO_ALPINE_TALL,
            rock: spr::DECO_ALPINE_ROCK,
            tufts: [spr::TUFT_ALPINE_A, spr::TUFT_ALPINE_B],
            home: spr::HOME_DEN,
            accent: rgb(0x5a9ae0),
            drift: Drift::Snow,
        },
        _ => Palette {
            ground: (rgb(0xbcaa86), rgb(0x8e8264)),
            trail: rgb(0x7a7672),
            trail_edge: rgb(0x403c3a),
            trail_light: rgb(0x94908a),
            water: rgb(0x6a94a8),
            mound: rgb(0x96845e),
            tall: spr::DECO_ROAD_TALL,
            rock: spr::DECO_ROAD_ROCK,
            tufts: [spr::TUFT_ROAD_A, spr::TUFT_ROAD_B],
            home: spr::HOME_ROADSEND,
            accent: rgb(0xe8a030),
            drift: Drift::Smoke,
        },
    }
}

/// The wash of light over a stage: (colour at the top, at the bottom).
/// Alpha is how strong it is.
pub fn mood_wash(mood: Mood) -> Option<(Color, Color)> {
    match mood {
        Mood::Day => None,
        Mood::Dawn => Some((Color::new(1.0, 0.72, 0.55, 0.22), Color::new(1.0, 0.86, 0.6, 0.06))),
        Mood::Dusk => Some((Color::new(0.55, 0.3, 0.62, 0.26), Color::new(1.0, 0.55, 0.25, 0.2))),
        Mood::Night => Some((Color::new(0.07, 0.1, 0.3, 0.5), Color::new(0.1, 0.16, 0.38, 0.42))),
    }
}

/// How much things should glow under this light (0 by day, 1 at night).
pub fn mood_glow(mood: Mood) -> f32 {
    match mood {
        Mood::Day => 0.0,
        Mood::Dawn => 0.25,
        Mood::Dusk => 0.5,
        Mood::Night => 1.0,
    }
}
