//! Biome Defense rules: the battle simulation and the campaign around it.
//! Nothing in here draws or makes a sound, and it has no dependencies, so
//! whole stages can be played natively thousands of times a second.

pub mod bot;
pub mod campaign;
pub mod creature;
pub mod defs;
pub mod level;
pub mod map;
pub mod rng;
pub mod sim;
pub mod v2;

pub use campaign::Profile;
pub use creature::Cr;
pub use defs::{Foe, Tw, BIOMES};
pub use level::{Difficulty, LEVELS, LEVEL_COUNT};
pub use rng::Rng;
pub use sim::{Ev, Phase, Setup, Sim, TICK};
pub use v2::{v2, V2};
