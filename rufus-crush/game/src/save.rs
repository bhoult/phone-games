//! Progress and settings, kept in the browser's localStorage as one short
//! line of text: `v2|sfx|music|paws per level|boops|difficulty`.
//!
//! `v1` saves are from before The Lake was added as the third chapter. They
//! are still read: the levels after the gap move along to make room.

use crate::platform;
use rufus_core::level::CHAPTER_LEN;
use rufus_core::{Difficulty, LEVELS};

/// Where The Lake's six levels were spliced into a `v1` save.
const LAKE_AT: usize = 2 * CHAPTER_LEN;

#[derive(Clone)]
pub struct Save {
    /// Best paw rating per level, 0 = not finished yet.
    pub paws: Vec<u8>,
    pub sfx: bool,
    pub music: bool,
    /// How many times Rufus's nose has been booped on the title screen.
    pub boops: u32,
    /// Easy, Normal or Hard: how many moves each level gives.
    pub difficulty: Difficulty,
    /// Set when a test switch faked this state: it is then never written.
    pub ephemeral: bool,
}

impl Default for Save {
    fn default() -> Save {
        Save { paws: vec![0; LEVELS.len()], sfx: true, music: true, boops: 0, difficulty: Difficulty::Normal, ephemeral: false }
    }
}

impl Save {
    pub fn load() -> Save {
        Save::parse(&platform::load()).unwrap_or_default()
    }

    fn parse(text: &str) -> Option<Save> {
        let mut parts = text.trim().split('|');
        let version = parts.next()?;
        if version != "v1" && version != "v2" {
            return None;
        }
        let sfx = parts.next()? == "1";
        let music = parts.next()? == "1";
        let mut paws: Vec<u8> = parts.next()?.bytes().map(|b| b.saturating_sub(b'0').min(3)).collect();
        if version == "v1" {
            paws.resize(LAKE_AT.max(paws.len()), 0);
            paws.splice(LAKE_AT..LAKE_AT, [0; CHAPTER_LEN]);
        }
        paws.resize(LEVELS.len(), 0);
        let boops = parts.next().and_then(|b| b.parse().ok()).unwrap_or(0);
        // saves from before the setting existed carry on at Normal
        let difficulty = parts.next().and_then(|d| d.parse().ok()).map(Difficulty::from_index).unwrap_or_default();
        Some(Save { paws, sfx, music, boops, difficulty, ephemeral: false })
    }

    pub fn store(&self) {
        if self.ephemeral {
            return;
        }
        let paws: String = self.paws.iter().map(|p| (b'0' + p) as char).collect();
        platform::save(&format!("v2|{}|{}|{}|{}|{}", self.sfx as u8, self.music as u8, paws, self.boops, self.difficulty.index()));
    }

    /// Index of the furthest level that can be played.
    pub fn frontier(&self) -> usize {
        self.paws.iter().position(|&p| p == 0).unwrap_or(LEVELS.len() - 1)
    }

    /// A level is open if everything before it is done, or if it has been
    /// beaten already (levels added in the middle later must not lock the
    /// ones after them).
    pub fn unlocked(&self, level: usize) -> bool {
        level <= self.frontier() || self.paws.get(level).is_some_and(|&p| p > 0)
    }

    pub fn total_paws(&self) -> u32 {
        self.paws.iter().map(|&p| p as u32).sum()
    }

    /// Records a finished level. Returns true if this beat the old rating.
    pub fn record(&mut self, level: usize, paws: u8) -> bool {
        let better = paws > self.paws[level];
        if better {
            self.paws[level] = paws;
            self.store();
        }
        better
    }
}
