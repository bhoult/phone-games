//! Rufus Crush rules. Pure logic: no rendering, no platform, no dependencies,
//! so it can be tested and simulated natively.

pub mod board;
pub mod bot;
pub mod level;
pub mod rng;

pub use board::*;
pub use level::{Difficulty, Goal, LevelDef, Session, LEVELS};
pub use rng::Rng;
