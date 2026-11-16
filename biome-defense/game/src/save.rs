//! Progress and settings, kept in the browser's localStorage as one short
//! line of text: `bd1;sfx;music;<the journey>`, where the journey is
//! `Profile::encode` or `-` before a creature has been chosen.

use crate::platform;
use biome_core::Profile;

#[derive(Clone)]
pub struct Save {
    /// None until a creature has been chosen.
    pub profile: Option<Profile>,
    pub sfx: bool,
    pub music: bool,
    /// Set when a test switch faked this state: it is then never written.
    pub ephemeral: bool,
}

impl Default for Save {
    fn default() -> Save {
        Save { profile: None, sfx: true, music: true, ephemeral: false }
    }
}

impl Save {
    pub fn load() -> Save {
        Save::parse(&platform::load()).unwrap_or_default()
    }

    fn parse(text: &str) -> Option<Save> {
        let mut parts = text.trim().splitn(4, ';');
        if parts.next()? != "bd1" {
            return None;
        }
        let sfx = parts.next()? == "1";
        let music = parts.next()? == "1";
        let profile = Profile::decode(parts.next()?);
        Some(Save { profile, sfx, music, ephemeral: false })
    }

    pub fn store(&self) {
        if self.ephemeral {
            return;
        }
        let journey = self.profile.as_ref().map_or("-".to_string(), |p| p.encode());
        platform::save(&format!("bd1;{};{};{}", self.sfx as u8, self.music as u8, journey));
    }

    /// The journey so far. Only call once a creature has been chosen.
    pub fn p(&self) -> &Profile {
        self.profile.as_ref().expect("no journey yet")
    }

    pub fn p_mut(&mut self) -> &mut Profile {
        self.profile.as_mut().expect("no journey yet")
    }
}
