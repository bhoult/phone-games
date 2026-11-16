//! The journey between stages: which stages are done, who is in the pack,
//! and what everyone has learned.
//!
//! Points come from clearing a stage for the first time, one of each kind
//! per stage, on any difficulty. They cannot be farmed.
//!
//! - **Skill points** are the player creature's. Once spent they stay spent.
//! - **Bond points** are shared by the followers. When a follower leaves
//!   the pack its bond points come back, ready for whoever walks with you
//!   next.

use crate::creature::{skill_cost, spent, Cr, FRIENDS};
use crate::defs::BIOMES;
use crate::level::{is_finale, Difficulty, LEVEL_COUNT, PER_BIOME};

/// How many followers can walk with the player's creature.
pub const FOLLOWERS: usize = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub hero: Cr,
    pub difficulty: Difficulty,
    /// Best result per stage: 0 not cleared, else 1 to 3 stars.
    pub stars: [u8; LEVEL_COUNT],
    /// One bit per skill the player's creature has learned.
    pub hero_skills: u16,
    /// Who is walking with the player's creature.
    pub party: [Option<Cr>; FOLLOWERS],
    /// One bit per resident (in [`FRIENDS`] order) who has offered to join.
    pub friends: u8,
    /// Skills per resident. Only those in the pack have any.
    pub friend_skills: [u16; 5],
    /// One bit per story beat already told.
    pub seen: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Refusal {
    AlreadyKnown,
    /// The skill before it in the branch comes first.
    OutOfOrder,
    NotEnoughPoints,
    /// Not the player's creature and not in the pack.
    NotInPack,
}

impl Profile {
    pub fn new(hero: Cr) -> Profile {
        Profile { hero, difficulty: Difficulty::Normal, stars: [0; LEVEL_COUNT], hero_skills: 0, party: [None; FOLLOWERS], friends: 0, friend_skills: [0; 5], seen: 0 }
    }

    // ---- progress ---------------------------------------------------------

    pub fn cleared(&self) -> usize {
        self.stars.iter().filter(|&&s| s > 0).count()
    }

    /// The furthest stage that can be played.
    pub fn frontier(&self) -> usize {
        self.stars.iter().position(|&s| s == 0).unwrap_or(LEVEL_COUNT - 1)
    }

    pub fn unlocked(&self, level: usize) -> bool {
        level <= self.frontier()
    }

    pub fn finished(&self) -> bool {
        self.stars.iter().all(|&s| s > 0)
    }

    pub fn total_stars(&self) -> u32 {
        self.stars.iter().map(|&s| s as u32).sum()
    }

    /// Records a win. Returns true if it was the first time (which is what
    /// earns points), and makes the biome's resident a friend when the
    /// stage was the stand at their home.
    pub fn record(&mut self, level: usize, stars: u8) -> bool {
        let first = self.stars[level] == 0;
        self.stars[level] = self.stars[level].max(stars.clamp(1, 3));
        if is_finale(level) {
            if let Some(who) = BIOMES[level / PER_BIOME].resident {
                self.befriend(who);
            }
        }
        first
    }

    // ---- points -----------------------------------------------------------

    /// Skill points earned so far: one per stage cleared.
    pub fn skill_points(&self) -> u32 {
        self.cleared() as u32
    }

    pub fn skill_spent(&self) -> u32 {
        spent(self.hero, self.hero_skills)
    }

    pub fn skill_free(&self) -> u32 {
        self.skill_points() - self.skill_spent()
    }

    /// Bond points earned so far: one per stage cleared.
    pub fn bond_points(&self) -> u32 {
        self.cleared() as u32
    }

    pub fn bond_spent(&self) -> u32 {
        FRIENDS.iter().enumerate().map(|(i, &f)| spent(f, self.friend_skills[i])).sum()
    }

    pub fn bond_free(&self) -> u32 {
        self.bond_points() - self.bond_spent()
    }

    /// The skills a creature has learned, as a mask (see [`Cr::skill`]).
    pub fn skills_of(&self, who: Cr) -> u16 {
        if who == self.hero {
            self.hero_skills
        } else {
            who.friend_index().map_or(0, |i| self.friend_skills[i])
        }
    }

    pub fn knows(&self, who: Cr, branch: usize, tier: usize) -> bool {
        self.skills_of(who) & who.skill(branch, tier).1 != 0
    }

    /// Whether a skill can be learned right now, and if not, why not.
    pub fn can_learn(&self, who: Cr, branch: usize, tier: usize) -> Result<(), Refusal> {
        let hero = who == self.hero;
        if !hero && !self.in_party(who) {
            return Err(Refusal::NotInPack);
        }
        if self.knows(who, branch, tier) {
            return Err(Refusal::AlreadyKnown);
        }
        if tier > 0 && !self.knows(who, branch, tier - 1) {
            return Err(Refusal::OutOfOrder);
        }
        let free = if hero { self.skill_free() } else { self.bond_free() };
        if free < skill_cost(tier) {
            return Err(Refusal::NotEnoughPoints);
        }
        Ok(())
    }

    pub fn learn(&mut self, who: Cr, branch: usize, tier: usize) -> Result<(), Refusal> {
        self.can_learn(who, branch, tier)?;
        let bit = who.skill(branch, tier).1;
        if who == self.hero {
            self.hero_skills |= bit;
        } else if let Some(i) = who.friend_index() {
            self.friend_skills[i] |= bit;
        }
        Ok(())
    }

    // ---- the pack ---------------------------------------------------------

    pub fn is_friend(&self, who: Cr) -> bool {
        who.friend_index().is_some_and(|i| self.friends & (1 << i) != 0)
    }

    pub fn befriend(&mut self, who: Cr) {
        if let Some(i) = who.friend_index() {
            self.friends |= 1 << i;
        }
    }

    pub fn in_party(&self, who: Cr) -> bool {
        self.party.contains(&Some(who))
    }

    pub fn free_slot(&self) -> Option<usize> {
        self.party.iter().position(|p| p.is_none())
    }

    /// Friends who are not walking with the pack right now.
    pub fn at_home(&self) -> Vec<Cr> {
        FRIENDS.iter().copied().filter(|&f| self.is_friend(f) && !self.in_party(f)).collect()
    }

    /// Sends whoever is in a slot home. Their bond points come back.
    pub fn dismiss(&mut self, slot: usize) -> Option<Cr> {
        let who = self.party.get_mut(slot)?.take()?;
        if let Some(i) = who.friend_index() {
            self.friend_skills[i] = 0;
        }
        Some(who)
    }

    /// Brings a friend into a slot, sending home whoever was there. Does
    /// nothing if they are not a friend yet, or are already walking.
    pub fn join(&mut self, who: Cr, slot: usize) -> bool {
        if slot >= FOLLOWERS || !self.is_friend(who) || self.in_party(who) {
            return false;
        }
        self.dismiss(slot);
        self.party[slot] = Some(who);
        true
    }

    /// The pack as a stage wants it: the player's creature, then followers.
    pub fn pack(&self) -> Vec<(Cr, u16)> {
        let mut out = vec![(self.hero, self.hero_skills)];
        for p in self.party.iter().flatten() {
            out.push((*p, self.skills_of(*p)));
        }
        out
    }

    /// Residents who fight alongside on a stage without being in the pack:
    /// the one whose home it is, and at the very end, everyone.
    pub fn guests(&self, level: usize) -> Vec<Cr> {
        let biome = level / PER_BIOME;
        let mut out = Vec::new();
        match BIOMES[biome].resident {
            Some(who) => {
                if !self.in_party(who) {
                    out.push(who);
                }
            }
            None => {
                if level == LEVEL_COUNT - 1 {
                    out.extend(self.at_home());
                }
            }
        }
        out
    }

    // ---- saving -----------------------------------------------------------

    /// One short line of text: `b1|hero|difficulty|stars|skills|party|friends|friend skills|seen`.
    pub fn encode(&self) -> String {
        let stars: String = self.stars.iter().map(|s| (b'0' + s) as char).collect();
        let party: String = self.party.iter().map(|p| p.and_then(|c| c.friend_index()).map_or('-', |i| (b'0' + i as u8) as char)).collect();
        let fs: Vec<String> = self.friend_skills.iter().map(|m| format!("{m:x}")).collect();
        format!("b1|{}|{}|{}|{:x}|{}|{:x}|{}|{:x}", self.hero as u8, self.difficulty.index(), stars, self.hero_skills, party, self.friends, fs.join(","), self.seen)
    }

    /// Reads a saved line back. Anything that does not add up (a pack
    /// member who was never met, more points spent than earned) is put
    /// right rather than trusted.
    pub fn decode(text: &str) -> Option<Profile> {
        let mut parts = text.trim().split('|');
        if parts.next()? != "b1" {
            return None;
        }
        let hero = Cr::from_index(parts.next()?.parse().ok()?).filter(|c| c.is_hero())?;
        let mut p = Profile::new(hero);
        p.difficulty = Difficulty::from_index(parts.next()?.parse().ok()?);
        for (i, b) in parts.next()?.bytes().take(LEVEL_COUNT).enumerate() {
            p.stars[i] = b.saturating_sub(b'0').min(3);
        }
        p.hero_skills = u16::from_str_radix(parts.next()?, 16).ok()? & ((1u32 << hero.skill_count()) - 1) as u16;
        let party = parts.next()?;
        p.friends = u8::from_str_radix(parts.next()?, 16).ok()? & 0x1f;
        for (i, b) in party.bytes().take(FOLLOWERS).enumerate() {
            if b.is_ascii_digit() {
                let who = FRIENDS.get((b - b'0') as usize).copied();
                p.party[i] = who.filter(|&w| p.is_friend(w) && !p.in_party(w));
            }
        }
        for (i, m) in parts.next()?.split(',').take(5).enumerate() {
            let who = FRIENDS[i];
            let mask = u16::from_str_radix(m, 16).ok()? & ((1u32 << who.skill_count()) - 1) as u16;
            p.friend_skills[i] = if p.in_party(who) { mask } else { 0 };
        }
        p.seen = u32::from_str_radix(parts.next()?, 16).ok()?;
        // never more spent than earned
        if p.skill_spent() > p.skill_points() {
            p.hero_skills = 0;
        }
        if p.bond_spent() > p.bond_points() {
            p.friend_skills = [0; 5];
        }
        Some(p)
    }
}
