//! Rufus Crush rules. Pure logic: no rendering, no platform, no dependencies,
//! so it can be tested and simulated natively.

pub mod board;
pub mod bot;
pub mod chapters;
mod later;
pub mod level;
pub mod rng;
// Tuning plays thousands of games on every core: a build-time job.
#[cfg(not(target_arch = "wasm32"))]
pub mod tune;

pub use board::*;
pub use chapters::{Chapter, Place, CHAPTERS, CHAPTER_LEN};
pub use level::{Difficulty, Goal, LevelDef, Session, LEVELS};
pub use rng::Rng;
