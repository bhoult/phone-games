//! Which picture goes with which thing, and what each biome looks like.

use crate::gfx::*;
use biome_core::creature::{Cr, Icon};
use biome_core::defs::{Class, Look, Mode};
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

/// How an invader's picture was taken, which decides how it is moved.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    /// From straight above, pointing right: turned to face its heading.
    Above,
    /// From the side and above, facing right: mirrored to face its heading.
    Side,
    /// From above, with no front: left as it is (or rolled).
    Round,
}

pub fn foe_pose(f: Foe) -> Pose {
    match f {
        Foe::Surveyor | Foe::Camper | Foe::Trapper | Foe::Worker | Foe::Exterminator => Pose::Side,
        Foe::Creeper | Foe::SporePod | Foe::Sprout | Foe::Tumbleweed | Foe::Strangler => Pose::Round,
        _ => Pose::Above,
    }
}

/// What becomes of an invader once it is stopped.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// Lies where it fell and is gone.
    Dies,
    /// Turns and runs back the way it came.
    Flees,
    /// Stalls, smoking.
    Stalls,
}

pub fn foe_end(f: Foe) -> End {
    match f {
        Foe::Atv | Foe::Mower | Foe::Drone | Foe::PumpTruck | Foe::Buggy | Foe::Chopper | Foe::BigYellow => End::Stalls,
        _ if f.def().class == Class::People => End::Flees,
        _ => End::Dies,
    }
}

pub fn tower_spr(t: Tw) -> Spr {
    match t {
        Tw::Shrike => spr::SHRIKE,
        Tw::Puffball => spr::PUFFBALL,
        Tw::Bramble => spr::BRAMBLE,
        Tw::Hive => spr::HIVE,
        Tw::Rookery => spr::ROOKERY,
        Tw::BatRoost => spr::BAT_ROOST,
        Tw::SapRun => spr::SAP_RUN,
        Tw::Woodpecker => spr::WOODPECKER,
        Tw::Kingfisher => spr::KINGFISHER,
        Tw::Bullfrog => spr::BULLFROG,
        Tw::MudWallow => spr::MUD_WALLOW,
        Tw::EelPool => spr::EEL_POOL,
        Tw::Cholla => spr::CHOLLA,
        Tw::Bighorn => spr::BIGHORN,
        Tw::DustDevil => spr::DUST_DEVIL,
        Tw::RattlerDen => spr::RATTLER_DEN,
        Tw::Icefall => spr::ICEFALL,
        Tw::Cornice => spr::CORNICE,
        Tw::FrostVent => spr::FROST_VENT,
        Tw::Eyrie => spr::EYRIE,
    }
}

/// How big a structure is drawn (its longest side, in tiles), and whether
/// it lies flat on the ground (a pool, a wallow) or stands up from it.
pub fn tower_shape(t: Tw) -> (f32, bool) {
    match t {
        Tw::Bramble | Tw::MudWallow | Tw::EelPool | Tw::Bullfrog | Tw::DustDevil | Tw::SapRun | Tw::FrostVent => (1.12, true),
        Tw::BatRoost | Tw::Woodpecker | Tw::Rookery | Tw::Eyrie => (1.5, false),
        Tw::Bighorn | Tw::Shrike | Tw::Kingfisher | Tw::Cholla => (1.35, false),
        _ => (1.2, false),
    }
}

/// The colour a slowing field stains the ground, if this structure is one.
pub fn field_color(t: Tw) -> Option<Color> {
    if !matches!(t.def().mode, Mode::Field { .. }) {
        return None;
    }
    Some(match t {
        Tw::Bramble => rgb(0x4a5a2a),
        Tw::SapRun => rgb(0xc8862a),
        Tw::MudWallow => rgb(0x4a3420),
        Tw::DustDevil => rgb(0xd8b88a),
        _ => rgb(0xbfe0f0),
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

/// A creature's portrait: round, for cards and the story.
pub fn cr_face(c: Cr) -> Spr {
    match c {
        Cr::Fox => spr::FACE_FOX,
        Cr::Badger => spr::FACE_BADGER,
        Cr::Owl => spr::FACE_OWL,
        Cr::Rabbit => spr::FACE_RABBIT,
        Cr::Squirrel => spr::FACE_SQUIRREL,
        Cr::Beaver => spr::FACE_BEAVER,
        Cr::Tortoise => spr::FACE_TORTOISE,
        Cr::Marmot => spr::FACE_MARMOT,
    }
}

/// How long a creature is on the field, in tiles.
pub fn cr_size(c: Cr) -> f32 {
    match c {
        Cr::Fox => 1.4,
        Cr::Badger => 1.35,
        Cr::Owl => 1.55,
        Cr::Rabbit => 1.0,
        Cr::Squirrel => 0.9,
        Cr::Beaver => 1.25,
        Cr::Tortoise => 1.05,
        Cr::Marmot => 0.95,
    }
}

/// The full-length picture of each creature you can play as.
pub fn hero_scene(c: Cr) -> usize {
    match c {
        Cr::Badger => scene::HERO_BADGER,
        Cr::Owl => scene::HERO_OWL,
        _ => scene::HERO_FOX,
    }
}

/// A colour of its own for each creature: rings, name plates.
pub fn cr_color(c: Cr) -> Color {
    match c {
        Cr::Fox => rgb(0xc8743a),
        Cr::Badger => rgb(0x9a9fa6),
        Cr::Owl => rgb(0x9488b8),
        Cr::Rabbit => rgb(0xb89a72),
        Cr::Squirrel => rgb(0xb8603e),
        Cr::Beaver => rgb(0x9a7650),
        Cr::Tortoise => rgb(0x8f9c5e),
        Cr::Marmot => rgb(0xbfa05a),
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
pub const BRANCH_COLORS: [Color; 3] = [rgb(0xb8503e), rgb(0x5a86a8), rgb(0xc09a3e)];

/// What a shot looks like in the air: its picture, its length in tiles,
/// and whether it tumbles (a stone) or points the way it flies (a bird).
pub fn shot_spr(l: Look) -> Option<(Spr, f32, bool)> {
    Some(match l {
        Look::Shrike => (spr::SHOT_SHRIKE, 0.62, false),
        Look::Kingfisher => (spr::SHOT_KINGFISHER, 0.58, false),
        Look::Bat => (spr::SHOT_BAT, 0.5, false),
        Look::Spore => (spr::SHOT_SPORE, 0.42, true),
        Look::Bee => (spr::SHOT_BEE, 0.26, false),
        Look::Stone | Look::Pebble => (spr::SHOT_STONE, 0.3, true),
        Look::Rock => (spr::SHOT_BOULDER, 0.5, true),
        Look::Fang | Look::Spine => (spr::SHOT_SPINE, 0.42, false),
        Look::Icicle => (spr::SHOT_ICICLE, 0.5, false),
        Look::Snow => (spr::SHOT_SNOW, 0.46, true),
        Look::Acorn => (spr::SHOT_ACORN, 0.26, true),
        Look::Feather => (spr::SHOT_FEATHER, 0.42, false),
        Look::Peck | Look::Tongue | Look::Zap | Look::Dive | Look::None => return None,
    })
}

/// The colour of the bits that fly when a shot lands.
pub fn look_color(l: Look) -> Color {
    match l {
        Look::Spore => rgb(0xcfc4a4),
        Look::Bee => rgb(0xd9a520),
        Look::Tongue => rgb(0xc87888),
        Look::Zap => rgb(0xcfe8f0),
        Look::Rock => rgb(0xb06a44),
        Look::Fang => rgb(0xa8c060),
        Look::Icicle | Look::Snow => rgb(0xdcecf4),
        Look::Spine => rgb(0xe0d6b0),
        Look::Acorn => rgb(0xa07840),
        Look::Stone | Look::Pebble => rgb(0xa8a8a0),
        _ => rgb(0xe9e2d0),
    }
}

/// What drifts through the air in a biome.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    Seeds,
    Leaves,
    Midges,
    Dust,
    Snow,
    Smoke,
}

pub struct Palette {
    /// The ground and trail photographs (`tex::NAME`).
    pub ground: usize,
    pub trail: usize,
    /// How much to brighten the trail photograph: more where the ground is
    /// dark enough to lose it.
    pub trail_gain: f32,
    /// Multiplied into the ground: each biome's light.
    pub grade: Color,
    /// The colour of dust kicked up from the trail.
    pub dust: Color,
    pub tall: Spr,
    pub rock: Spr,
    pub home: Spr,
    /// The biome's own colour, for headings and the map.
    pub accent: Color,
    pub drift: Drift,
    /// The landscape behind the biome's story, and its stretch of the map.
    pub backdrop: usize,
    pub map: usize,
}

pub fn palette(biome: usize) -> Palette {
    match biome {
        0 => Palette {
            ground: tex::GROUND_MEADOW,
            trail: tex::TRAIL_MEADOW,
            trail_gain: 1.0,
            grade: rgb(0xf2f0e6),
            dust: rgb(0x8a6a48),
            tall: spr::DECO_MEADOW_TALL,
            rock: spr::DECO_MEADOW_ROCK,
            home: spr::HOME_WARREN,
            accent: rgb(0x9ab060),
            drift: Drift::Seeds,
            backdrop: scene::STORY_MEADOW,
            map: scene::MAP_MEADOW,
        },
        1 => Palette {
            ground: tex::GROUND_FOREST,
            trail: tex::TRAIL_FOREST,
            trail_gain: 1.75,
            grade: rgb(0xf0f2ea),
            dust: rgb(0x5a4632),
            tall: spr::DECO_FOREST_TALL,
            rock: spr::DECO_FOREST_ROCK,
            home: spr::HOME_OAK,
            accent: rgb(0x6f9a62),
            drift: Drift::Leaves,
            backdrop: scene::STORY_FOREST,
            map: scene::MAP_FOREST,
        },
        2 => Palette {
            ground: tex::GROUND_MARSH,
            trail: tex::TRAIL_MARSH,
            trail_gain: 1.05,
            grade: rgb(0xeef0ee),
            dust: rgb(0x5e5646),
            tall: spr::DECO_MARSH_TALL,
            rock: spr::DECO_MARSH_ROCK,
            home: spr::HOME_LODGE,
            accent: rgb(0x6fa0a0),
            drift: Drift::Midges,
            backdrop: scene::STORY_MARSH,
            map: scene::MAP_MARSH,
        },
        3 => Palette {
            ground: tex::GROUND_DESERT,
            trail: tex::TRAIL_DESERT,
            trail_gain: 1.0,
            grade: rgb(0xf6efe4),
            dust: rgb(0xc8a880),
            tall: spr::DECO_DESERT_TALL,
            rock: spr::DECO_DESERT_ROCK,
            home: spr::HOME_BURROW,
            accent: rgb(0xc8804a),
            drift: Drift::Dust,
            backdrop: scene::STORY_DESERT,
            map: scene::MAP_DESERT,
        },
        4 => Palette {
            ground: tex::GROUND_ALPINE,
            trail: tex::TRAIL_ALPINE,
            trail_gain: 0.95,
            grade: rgb(0xeef2f6),
            dust: rgb(0x8a8e94),
            tall: spr::DECO_ALPINE_TALL,
            rock: spr::DECO_ALPINE_ROCK,
            home: spr::HOME_DEN,
            accent: rgb(0x8ab0cc),
            drift: Drift::Snow,
            backdrop: scene::STORY_ALPINE,
            map: scene::MAP_ALPINE,
        },
        _ => Palette {
            ground: tex::GROUND_ROAD,
            trail: tex::TRAIL_ROAD,
            trail_gain: 1.0,
            grade: rgb(0xf0ece6),
            dust: rgb(0x7a746c),
            tall: spr::DECO_ROAD_TALL,
            rock: spr::DECO_ROAD_ROCK,
            home: spr::HOME_ROADSEND,
            accent: rgb(0xd0a040),
            drift: Drift::Smoke,
            backdrop: scene::STORY_ROAD,
            map: scene::MAP_ROAD,
        },
    }
}

/// The wash of light over a stage: (colour at the top, at the bottom).
/// Alpha is how strong it is.
pub fn mood_wash(mood: Mood) -> Option<(Color, Color)> {
    match mood {
        Mood::Day => None,
        Mood::Dawn => Some((Color::new(0.95, 0.66, 0.5, 0.2), Color::new(0.9, 0.78, 0.6, 0.06))),
        Mood::Dusk => Some((Color::new(0.34, 0.22, 0.42, 0.34), Color::new(0.8, 0.42, 0.2, 0.2))),
        Mood::Night => Some((Color::new(0.04, 0.07, 0.2, 0.52), Color::new(0.06, 0.1, 0.26, 0.42))),
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
