//! One stage being played: invaders walking the trail, structures and the
//! pack stopping them. Time moves in fixed ticks, so a stage plays out the
//! same way wherever it runs, and can be run thousands of times a second
//! without a screen (which is how the stages are tested and balanced).
//!
//! Nothing here animates. Each tick appends what happened to `events`, and
//! the game layer turns those into pictures and sound.

use crate::creature::{muster, Ability, Cr, Flag, Perks, Stats, AURA_R};
use crate::defs::{Bonus, Class, Foe, Look, Mode, Perk, Quirk, Rider, Tw, LV_DMG, LV_RANGE, LV_RATE, LV_SLOW, SELL_BACK};
use crate::level::{waves, Difficulty, LevelDef, Wave, LEVELS};
use crate::map::{Map, COLS, ROWS};
use crate::rng::Rng;
use crate::v2::{v2, V2};

/// Length of one step of the simulation, in seconds.
pub const TICK: f32 = 1.0 / 60.0;
/// How close an invader has to come to a fighter to be held up by it.
const GRAB: f32 = 0.62;
/// How far a fighter will step from where it was sent to reach an invader.
pub const LEASH: f32 = 1.6;
/// Seconds between a wave's last invader appearing and the next wave.
pub const WAVE_GAP: f32 = 14.0;
const BITE_EVERY: f32 = 0.7;
/// The least room two of the pack leave between where they stand.
const ELBOW: f32 = 0.62;
/// A boss can be held this long, then shoulders past and cannot be held
/// again for a while.
const BOSS_HELD: f32 = 1.5;
const BOSS_LOOSE: f32 = 4.0;
/// Everything a stage pays out, as a share of what it would cost to build
/// on every mound and upgrade all of it.
pub const INCOME_SHARE: f32 = 0.65;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Set up; waiting to be told to start the first wave.
    Ready,
    Battle,
    Won,
    Lost,
}

/// Who gets the credit for a hit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Src {
    Tower(usize),
    Unit(usize),
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hold {
    Unit(usize),
    Block(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockKind {
    Dam,
    Decoy,
    Cousin,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ZoneKind {
    /// Cracked ground: slows what walks over it.
    Crack(f32),
    /// A whirlwind: shoves everything inside back every second.
    Twister(f32),
}

/// Something that happened this tick, for the game layer to show.
#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Spawn { foe: Foe, at: V2 },
    /// A structure let fly.
    Fire { spot: usize, to: V2 },
    /// Something instant: a peck, a tongue, a jolt, a stoop.
    Beam { look: Look, a: V2, b: V2 },
    /// One invader was hit.
    Hit { at: V2, look: Look, crit: bool },
    /// Something burst over an area.
    Burst { at: V2, radius: f32, look: Look },
    /// A creature swung at something.
    Strike { unit: usize, at: V2 },
    Shooed { at: V2, foe: Foe, seeds: u32 },
    /// An invader reached the home.
    Leak { at: V2, foe: Foe, hearts: i32 },
    Cast { unit: usize, who: Cr, at: V2, radius: f32 },
    Leap { unit: usize, from: V2, to: V2 },
    Ko { unit: usize },
    Up { unit: usize },
    /// Brock refused to go down.
    Stand { unit: usize },
    Built { spot: usize },
    Upgraded { spot: usize },
    Sold { spot: usize, seeds: u32 },
    Wave { n: usize, seeds: u32 },
    /// Structures in a circle were sprayed out of action.
    Jam { at: V2, radius: f32 },
    /// A rattle sent people running.
    Scare { at: V2, radius: f32 },
    /// Roots or frost held everything in a circle.
    Root { at: V2, radius: f32 },
    BlockUp { at: V2, kind: BlockKind },
    BlockDown { at: V2, kind: BlockKind, flood: bool },
    Swallow { at: V2 },
    Haste { spot: usize },
    Mend,
    Boss { foe: Foe },
    Won { stars: u8 },
    Lost,
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub id: u32,
    pub foe: Foe,
    pub hp: f32,
    pub max_hp: f32,
    pub route: usize,
    /// How far along its route it has come.
    pub dist: f32,
    lane: f32,
    pub pos: V2,
    pub prev: V2,
    pub dir: V2,
    pub slow: f32,
    pub slow_t: f32,
    field_slow: f32,
    pub poison: f32,
    pub poison_t: f32,
    pub stun: f32,
    pub fear: f32,
    pub shred: f32,
    pub shred_t: f32,
    pub mark: f32,
    pub mark_t: f32,
    pub held: Option<Hold>,
    /// Bosses only: how long it has been held, and how long until it can
    /// be held again. Nothing holds a boss for long.
    held_for: f32,
    loose: f32,
    bite_cd: f32,
    timers: [f32; 2],
    /// Seconds since last hurt; the game flashes it white while small.
    pub hurt_t: f32,
    /// Seeds it drops when shooed.
    seeds: f32,
    /// Its wave's health multiplier, passed on to anything it lets out.
    tough: f32,
    /// The wave it came in (counting from 0).
    wave: usize,
    by: Src,
    leaked: bool,
}

impl Enemy {
    /// How slowed it is right now, as a fraction.
    pub fn slowed(&self) -> f32 {
        let s = if self.slow_t > 0.0 { self.slow } else { 0.0 };
        s.max(self.field_slow).min(0.8)
    }
}

#[derive(Clone, Debug)]
pub struct Tower {
    pub kind: Tw,
    /// 1 to 3.
    pub level: u8,
    pub pos: V2,
    cd: f32,
    /// Seconds left out of action.
    pub jam: f32,
    /// Seconds left of a whistle's hurry, and how much hurry.
    pub haste: f32,
    haste_amt: f32,
    pulse: f32,
    scare: f32,
    /// Seeds spent on it so far.
    pub spent: u32,
    /// Where it last aimed.
    pub aim: V2,
    /// Seconds since it last fired (for the recoil).
    pub fired: f32,
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub who: Cr,
    pub st: Stats,
    /// A resident fighting for their own home: acts on their own.
    pub guest: bool,
    pub pos: V2,
    pub prev: V2,
    /// Where it has been told to stand.
    pub post: V2,
    pub hp: f32,
    /// Seconds left knocked out (0 = up).
    pub ko: f32,
    atk_cd: f32,
    /// Seconds until the ability is ready.
    pub cd: f32,
    ordered: bool,
    /// +1 facing right, -1 facing left.
    pub face: f32,
    /// Seconds left pulled into a shell.
    pub shell: f32,
    pub frenzy: u8,
    frenzy_t: f32,
    strikes: u32,
    stand_cd: f32,
    /// Seconds left of refusing to go down.
    pub stand: f32,
    calm: f32,
    /// Seconds since it last attacked (for the swing).
    pub swung: f32,
    pub holding: u8,
}

impl Unit {
    pub fn alive(&self) -> bool {
        self.ko <= 0.0
    }
    fn cap(&self) -> u8 {
        if self.who.def().perched {
            0
        } else {
            self.st.block + if self.shell > 0.0 { 3 } else { 0 }
        }
    }
    /// Ability cooldown as a fraction still to go.
    pub fn cooling(&self) -> f32 {
        (self.cd / self.st.cd.max(0.1)).clamp(0.0, 1.0)
    }
}

/// What a hit does to whatever it lands on.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub dmg: f32,
    pub pierce: bool,
    pub splash: f32,
    pub bonus: Bonus,
    pub slow: (f32, f32),
    pub poison: (f32, f32),
    pub shred: (f32, f32),
    pub stun: f32,
    pub mark: (f32, f32),
    pub shove: f32,
    /// Carries on into one more invader near the first.
    pub through: bool,
    /// A burst that reaches fliers.
    pub air: bool,
    pub crit: bool,
}

impl Hit {
    pub fn plain(dmg: f32) -> Hit {
        Hit { dmg, pierce: false, splash: 0.0, bonus: Bonus::None, slow: (0.0, 0.0), poison: (0.0, 0.0), shred: (0.0, 0.0), stun: 0.0, mark: (0.0, 0.0), shove: 0.0, through: false, air: false, crit: false }
    }
}

#[derive(Clone, Debug)]
pub struct Shot {
    pub look: Look,
    pub from: V2,
    pub to: V2,
    pub pos: V2,
    pub prev: V2,
    pub t: f32,
    pub dur: f32,
    /// Arcs through the air and bursts where it lands.
    pub lob: bool,
    target: Option<u32>,
    hit: Hit,
    src: Src,
}

#[derive(Clone, Debug)]
pub struct Block {
    pub id: u32,
    pub kind: BlockKind,
    pub pos: V2,
    pub hp: f32,
    pub max_hp: f32,
    pub t: f32,
    slots: u8,
    holding: u8,
    splinters: bool,
    flood: bool,
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub kind: ZoneKind,
    pub pos: V2,
    pub radius: f32,
    pub t: f32,
    tick: f32,
}

/// Everything needed to start a stage.
pub struct Setup {
    pub level: usize,
    pub difficulty: Difficulty,
    /// The pack: the player's creature first, then followers, each with
    /// the skills it has learned.
    pub pack: Vec<(Cr, u16)>,
    /// Residents fighting alongside, on their own initiative.
    pub guests: Vec<Cr>,
    pub seed: u64,
}

pub struct Sim {
    pub level: usize,
    pub def: &'static LevelDef,
    pub map: Map,
    pub waves: Vec<Wave>,
    /// How many waves have started.
    pub wave: usize,
    /// The stage ends once this many waves are dealt with (all of them,
    /// unless the balance tool has cut it short).
    last_wave: usize,
    pending: Vec<(f32, Foe, u8, f32, f32, usize)>,
    /// Seconds until the next wave starts by itself.
    pub next_in: f32,
    pub seeds: u32,
    pub hearts: i32,
    pub max_hearts: i32,
    pub foes: Vec<Enemy>,
    /// One slot per mound.
    pub towers: Vec<Option<Tower>>,
    pub units: Vec<Unit>,
    pub shots: Vec<Shot>,
    pub blocks: Vec<Block>,
    pub zones: Vec<Zone>,
    pub events: Vec<Ev>,
    pub phase: Phase,
    pub time: f32,
    pub perks: Perks,
    hp_mult: f32,
    boss_mult: f32,
    /// Seeds all the stage's invaders drop between them.
    payout: f32,
    seed_frac: f32,
    rng: Rng,
    next_id: u32,
    pub shooed: u32,
    /// Seeds earned from invaders, bonuses and early calls.
    pub earned: u32,
    /// Damage done by structures, and by creatures: for the balance tool.
    pub dealt: [f32; 2],
    /// Invaders that got through, by the wave they came in.
    pub leaks: Vec<u32>,
}

impl Sim {
    pub fn new(s: &Setup) -> Sim {
        let def = &LEVELS[s.level];
        let map = def.map();
        let (stats, perks) = muster(&s.pack);
        let max_hearts = s.difficulty.hearts() + perks.hearts as i32;
        let mut units = Vec::new();
        let trail = &map.routes[0];
        // the pack lines up along the last stretch of trail before the home
        for (i, ((who, _), st)) in s.pack.iter().zip(stats).enumerate() {
            let (p, dir) = trail.at(trail.len - 2.4 - 1.4 * i as f32);
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let pos = if who.def().perched { clamp_map(p + dir.perp() * (0.8 * side)) } else { p };
            units.push(new_unit(*who, st, false, pos));
        }
        // residents stay by their own door
        let mut perch = 0;
        let mut door = 0;
        for who in &s.guests {
            let pos = if who.def().perched {
                perch += 1;
                clamp_map(map.home + v2(if perch % 2 == 1 { -0.95 } else { 0.95 }, -0.25 - 0.5 * ((perch - 1) / 2) as f32))
            } else {
                door += 1;
                trail.at(trail.len - 0.55 - 0.75 * door as f32).0
            };
            units.push(new_unit(*who, Stats::base(*who), true, pos));
        }
        let mounds = map.mounds.len();
        // What invaders drop is fixed per stage: with the starting seeds
        // and the wave bonuses it comes to a set share of what it would
        // cost to build on every mound and upgrade all of it. Each wave
        // gets a fixed part of that to share out, so how many invaders a
        // wave has says nothing about how rich it is.
        let waves = waves(s.level);
        let offered = crate::defs::BIOMES[def.biome].towers;
        let full: u32 = offered.iter().map(|t| t.price(1) + t.price(2) + t.price(3)).sum::<u32>() / offered.len() as u32;
        let bonuses: u32 = (1..def.waves as u32).map(|n| 8 + 2 * n).sum();
        let payout = (INCOME_SHARE * (full * mounds as u32) as f32 - def.seeds as f32 - bonuses as f32).max(50.0);
        Sim {
            level: s.level,
            def,
            last_wave: waves.len(),
            waves,
            map,
            wave: 0,
            pending: Vec::new(),
            next_in: 0.0,
            seeds: def.seeds + perks.seeds,
            hearts: max_hearts,
            max_hearts,
            foes: Vec::new(),
            towers: vec![None; mounds],
            units,
            shots: Vec::new(),
            blocks: Vec::new(),
            zones: Vec::new(),
            events: Vec::new(),
            phase: Phase::Ready,
            time: 0.0,
            perks,
            hp_mult: s.difficulty.toughness(),
            boss_mult: def.boss_tough * s.difficulty.toughness(),
            payout,
            seed_frac: 0.0,
            rng: Rng::new(s.seed),
            next_id: 1,
            shooed: 0,
            earned: 0,
            dealt: [0.0; 2],
            leaks: vec![0; def.waves as usize],
        }
    }

    /// Overrides how tough the boss is (the balance tool searches on this).
    pub fn set_boss(&mut self, mult: f32) {
        self.boss_mult = mult;
    }

    /// Replaces the waves before the stage starts (the balance tool tries
    /// different crowd sizes, and stops the stage short).
    pub fn set_waves(&mut self, waves: Vec<Wave>, stop_after: usize) {
        if self.phase == Phase::Ready {
            self.last_wave = stop_after.min(waves.len());
            self.waves = waves;
        }
    }

    // ---- what the player can do ---------------------------------------------

    /// What building (`to` = 1) or upgrading a structure costs right now.
    pub fn price(&self, kind: Tw, to: u8) -> u32 {
        let raw = kind.price(to) as f32 * (1.0 - self.perks.cheap).max(0.3);
        ((raw / 5.0).round() * 5.0).max(5.0) as u32
    }

    pub fn build(&mut self, spot: usize, kind: Tw) -> bool {
        if spot >= self.towers.len() || self.towers[spot].is_some() || !self.playing() {
            return false;
        }
        let cost = self.price(kind, 1);
        if self.seeds < cost {
            return false;
        }
        self.seeds -= cost;
        let pos = self.map.mounds[spot];
        self.towers[spot] = Some(Tower { kind, level: 1, pos, cd: 0.3, jam: 0.0, haste: 0.0, haste_amt: 0.0, pulse: 2.0, scare: 2.0, spent: cost, aim: pos + v2(0.0, -1.0), fired: 9.0 });
        self.events.push(Ev::Built { spot });
        true
    }

    /// Cost of the next level of the structure on a mound, if it has one.
    pub fn upgrade_price(&self, spot: usize) -> Option<u32> {
        let t = self.towers.get(spot)?.as_ref()?;
        (t.level < 3).then(|| self.price(t.kind, t.level + 1))
    }

    pub fn upgrade(&mut self, spot: usize) -> bool {
        let Some(cost) = self.upgrade_price(spot) else { return false };
        if self.seeds < cost || !self.playing() {
            return false;
        }
        self.seeds -= cost;
        let t = self.towers[spot].as_mut().unwrap();
        t.level += 1;
        t.spent += cost;
        self.events.push(Ev::Upgraded { spot });
        true
    }

    /// What selling the structure on a mound would give back.
    pub fn sell_value(&self, spot: usize) -> u32 {
        let Some(Some(t)) = self.towers.get(spot) else { return 0 };
        if self.perks.refund {
            t.spent
        } else {
            (t.spent as f32 * SELL_BACK).round() as u32
        }
    }

    pub fn sell(&mut self, spot: usize) -> bool {
        if !self.playing() || !matches!(self.towers.get(spot), Some(Some(_))) {
            return false;
        }
        let seeds = self.sell_value(spot);
        self.seeds += seeds;
        self.towers[spot] = None;
        self.events.push(Ev::Sold { spot, seeds });
        true
    }

    /// How far the structure on a mound reaches, with any help it is getting.
    pub fn reach(&self, spot: usize) -> f32 {
        match &self.towers[spot] {
            Some(t) => t.kind.def().range * LV_RANGE[t.level as usize - 1] * (1.0 + self.aura(t.pos).2),
            None => 0.0,
        }
    }

    /// Sends a pack member somewhere. Residents do not take orders.
    pub fn order(&mut self, unit: usize, to: V2) {
        if unit >= self.units.len() || self.units[unit].guest {
            return;
        }
        // nobody stands on anybody else: a spot already taken is shifted
        // to just beside it
        let mut to = clamp_map(to);
        for (i, other) in self.units.iter().enumerate() {
            if i != unit && other.post.dist(to) < ELBOW {
                let away = (to - other.post).norm();
                let away = if away.len() < 0.5 { v2(1.0, 0.0) } else { away };
                to = clamp_map(other.post + away * ELBOW);
            }
        }
        let u = &mut self.units[unit];
        u.post = to;
        u.ordered = true;
    }

    pub fn playing(&self) -> bool {
        matches!(self.phase, Phase::Ready | Phase::Battle)
    }

    /// Can the next wave be called right now?
    pub fn can_call(&self) -> bool {
        match self.phase {
            Phase::Ready => true,
            Phase::Battle => self.wave < self.last_wave && self.pending.is_empty(),
            _ => false,
        }
    }

    /// Seeds offered for calling the next wave before it comes by itself.
    pub fn call_bonus(&self) -> u32 {
        if self.phase == Phase::Battle && self.can_call() {
            (self.next_in * 1.5).ceil() as u32
        } else {
            0
        }
    }

    /// Starts the next wave now. Returns false if it cannot be.
    pub fn call(&mut self) -> bool {
        if !self.can_call() {
            return false;
        }
        let bonus = self.call_bonus();
        self.start_wave(bonus);
        true
    }

    /// The wave that will start next, if there is one.
    pub fn upcoming(&self) -> Option<&Wave> {
        self.waves.get(self.wave)
    }

    fn start_wave(&mut self, early: u32) {
        let n = self.wave;
        let mut seeds = early + self.perks.wave_seeds;
        if n > 0 {
            seeds += 8 + 2 * n as u32;
            if self.perks.mend && self.hearts < self.max_hearts {
                self.hearts += 1;
                self.events.push(Ev::Mend);
            }
        }
        self.gain(seeds);
        let wave = &self.waves[n];
        // the wave's pot is shared between its invaders by how much trouble each is
        let trouble: f32 = wave.groups.iter().filter(|g| !g.foe.def().boss).map(|g| g.foe.own_threat() * g.count as f32).sum();
        let per = self.payout * wave.pot / trouble.max(0.01);
        for g in &wave.groups {
            let seeds = if g.foe.def().boss { 50.0 } else { g.foe.own_threat() * per };
            for k in 0..g.count {
                self.pending.push((self.time + g.delay + g.gap * k as f32, g.foe, g.spawn, seeds, wave.tough, n));
            }
        }
        self.pending.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        self.wave += 1;
        self.next_in = WAVE_GAP;
        self.phase = Phase::Battle;
        self.events.push(Ev::Wave { n: self.wave, seeds });
    }

    fn gain(&mut self, seeds: u32) {
        self.seeds += seeds;
        self.earned += seeds;
    }

    /// Stars for a win with the hearts left right now.
    pub fn stars(&self) -> u8 {
        if self.hearts * 10 >= self.max_hearts * 9 {
            3
        } else if self.hearts * 2 >= self.max_hearts {
            2
        } else {
            1
        }
    }

    // ---- the tick ---------------------------------------------------------------

    /// Advances the stage by [`TICK`] seconds.
    pub fn tick(&mut self) {
        if self.phase != Phase::Battle && self.phase != Phase::Ready {
            return;
        }
        let dt = TICK;
        self.time += dt;
        if self.phase == Phase::Battle {
            self.step_waves(dt);
        }
        self.step_zones(dt);
        self.step_foes(dt);
        self.step_units(dt);
        self.step_towers(dt);
        self.step_shots(dt);
        self.reap();
        if self.phase == Phase::Battle {
            if self.hearts <= 0 {
                self.hearts = 0;
                self.phase = Phase::Lost;
                self.events.push(Ev::Lost);
            } else if self.wave == self.last_wave && self.pending.is_empty() && self.foes.is_empty() {
                self.phase = Phase::Won;
                self.events.push(Ev::Won { stars: self.stars() });
            }
        }
    }

    fn step_waves(&mut self, dt: f32) {
        while let Some(&(at, foe, spawn, seeds, tough, wave)) = self.pending.last() {
            if at > self.time {
                break;
            }
            self.pending.pop();
            let routes = self.map.routes_from(spawn);
            let route = routes[self.rng.below(routes.len() as u32) as usize];
            let i = self.spawn(foe, route, 0.0, tough);
            self.foes[i].seeds = seeds;
            self.foes[i].wave = wave;
            if foe.def().boss {
                self.events.push(Ev::Boss { foe });
            }
        }
        if self.pending.is_empty() && self.wave < self.last_wave {
            // an empty field brings the next wave on sooner
            let hurry = if self.foes.is_empty() { 3.0 } else { 1.0 };
            self.next_in -= dt * hurry;
            if self.next_in <= 0.0 {
                self.start_wave(0);
            }
        }
    }

    fn spawn(&mut self, foe: Foe, route: usize, dist: f32, tough: f32) -> usize {
        let d = foe.def();
        let lane = if d.boss { 0.0 } else { self.rng.range(-0.2, 0.2) };
        let (p, dir) = self.map.routes[route].at(dist);
        let pos = p + dir.perp() * lane;
        let hp = d.hp * if d.boss { self.boss_mult } else { self.hp_mult * tough };
        let id = self.next_id;
        self.next_id += 1;
        let mut timers = [0.0; 2];
        for (i, q) in d.quirks.iter().enumerate().take(2) {
            timers[i] = match *q {
                Quirk::Jam { every, .. } => every * 0.6,
                Quirk::Brood { every, .. } => every,
                _ => 0.0,
            };
        }
        self.foes.push(Enemy {
            id,
            foe,
            hp,
            max_hp: hp,
            route,
            dist,
            lane,
            pos,
            prev: pos,
            dir,
            slow: 0.0,
            slow_t: 0.0,
            field_slow: 0.0,
            poison: 0.0,
            poison_t: 0.0,
            stun: 0.0,
            fear: 0.0,
            shred: 0.0,
            shred_t: 0.0,
            mark: 0.0,
            mark_t: 0.0,
            held: None,
            held_for: 0.0,
            loose: 0.0,
            bite_cd: BITE_EVERY * 0.5,
            timers,
            hurt_t: 9.0,
            seeds: 0.0,
            tough,
            wave: self.wave.saturating_sub(1),
            by: Src::Other,
            leaked: false,
        });
        self.events.push(Ev::Spawn { foe, at: pos });
        self.foes.len() - 1
    }

    fn step_zones(&mut self, dt: f32) {
        for zi in 0..self.zones.len() {
            self.zones[zi].t -= dt;
            let z = self.zones[zi].clone();
            match z.kind {
                ZoneKind::Crack(slow) => {
                    for e in &mut self.foes {
                        if !e.foe.def().flying && e.pos.dist(z.pos) <= z.radius {
                            let s = if e.foe.def().boss { slow * 0.5 } else { slow };
                            e.slow = e.slow.max(s);
                            e.slow_t = e.slow_t.max(0.25);
                        }
                    }
                }
                ZoneKind::Twister(push) => {
                    self.zones[zi].tick -= dt;
                    if self.zones[zi].tick <= 0.0 {
                        self.zones[zi].tick = 1.0;
                        for i in 0..self.foes.len() {
                            if self.foes[i].pos.dist(z.pos) <= z.radius {
                                self.shove(i, push);
                            }
                        }
                    }
                }
            }
        }
        self.zones.retain(|z| z.t > 0.0);

        let mut down = Vec::new();
        for b in &mut self.blocks {
            b.t -= dt;
            if b.t <= 0.0 || b.hp <= 0.0 {
                down.push((b.pos, b.kind, b.flood));
            }
        }
        self.blocks.retain(|b| b.t > 0.0 && b.hp > 0.0);
        for (pos, kind, flood) in down {
            if flood {
                for i in 0..self.foes.len() {
                    if !self.foes[i].foe.def().flying && self.foes[i].pos.dist(pos) <= 1.7 {
                        self.hurt(i, 30.0, true, Src::Other);
                        self.shove(i, 2.0);
                    }
                }
            }
            self.events.push(Ev::BlockDown { at: pos, kind, flood });
        }
    }

    // ---- invaders ------------------------------------------------------------------

    fn step_foes(&mut self, dt: f32) {
        // Who is holding whom. Holds that no longer make sense are dropped,
        // then anyone on foot who has walked into a fighter is stopped.
        for u in &mut self.units {
            u.holding = 0;
        }
        for b in &mut self.blocks {
            b.holding = 0;
        }
        for i in 0..self.foes.len() {
            let (held, pos, free) = {
                let e = &mut self.foes[i];
                if e.foe.def().boss {
                    e.loose = (e.loose - dt).max(0.0);
                    if e.held.is_some() {
                        e.held_for += dt;
                        if e.held_for >= BOSS_HELD {
                            e.held_for = 0.0;
                            e.loose = BOSS_LOOSE;
                        }
                    }
                }
                (e.held, e.pos, e.fear > 0.0 || e.hp <= 0.0 || e.loose > 0.0)
            };
            let keep = !free
                && match held {
                    Some(Hold::Unit(u)) => {
                        let un = &self.units[u];
                        un.alive() && un.pos.dist(pos) < 1.25 && un.holding < un.cap()
                    }
                    Some(Hold::Block(id)) => self.blocks.iter().any(|b| b.id == id && b.holding < b.slots),
                    None => false,
                };
            if keep {
                match held {
                    Some(Hold::Unit(u)) => self.units[u].holding += 1,
                    Some(Hold::Block(id)) => self.blocks.iter_mut().find(|b| b.id == id).unwrap().holding += 1,
                    None => {}
                }
            } else {
                self.foes[i].held = None;
            }
        }
        for i in 0..self.foes.len() {
            let e = &self.foes[i];
            if e.held.is_some() || e.fear > 0.0 || e.hp <= 0.0 || e.loose > 0.0 || e.foe.def().flying || e.foe.rolls() {
                continue;
            }
            let mut best: Option<(Hold, f32)> = None;
            for (ui, u) in self.units.iter().enumerate() {
                if !u.alive() || u.holding >= u.cap() {
                    continue;
                }
                let d = u.pos.dist(e.pos);
                if d < GRAB + e.foe.def().size * 0.25 && best.is_none_or(|b| d < b.1) {
                    best = Some((Hold::Unit(ui), d));
                }
            }
            for b in &self.blocks {
                if b.holding >= b.slots {
                    continue;
                }
                let d = b.pos.dist(e.pos);
                if d < GRAB + e.foe.def().size * 0.25 && best.is_none_or(|x| d < x.1) {
                    best = Some((Hold::Block(b.id), d));
                }
            }
            if let Some((h, _)) = best {
                self.foes[i].held = Some(h);
                match h {
                    Hold::Unit(u) => self.units[u].holding += 1,
                    Hold::Block(id) => self.blocks.iter_mut().find(|b| b.id == id).unwrap().holding += 1,
                }
            }
        }

        // Each invader: wear off what has been done to it, then walk.
        let mut quirks: Vec<(usize, Quirk)> = Vec::new();
        let mut leaks: Vec<usize> = Vec::new();
        for i in 0..self.foes.len() {
            let route_len = self.map.routes[self.foes[i].route].len;
            let e = &mut self.foes[i];
            let d = e.foe.def();
            e.prev = e.pos;
            e.hurt_t += dt;
            e.slow_t = (e.slow_t - dt).max(0.0);
            e.shred_t = (e.shred_t - dt).max(0.0);
            if e.shred_t <= 0.0 {
                e.shred = 0.0;
            }
            e.mark_t = (e.mark_t - dt).max(0.0);
            let poisoned = e.poison_t > 0.0;
            if poisoned {
                e.hp -= e.poison * dt;
                e.poison_t -= dt;
            }
            for (qi, q) in d.quirks.iter().enumerate().take(2) {
                match *q {
                    Quirk::Regen(frac) => {
                        // poison is the cure for things that grow back
                        if !poisoned && e.hp > 0.0 && e.hp < e.max_hp {
                            e.hp = (e.hp + e.max_hp * frac * dt).min(e.max_hp);
                        }
                    }
                    Quirk::Jam { every, .. } | Quirk::Brood { every, .. } => {
                        if e.stun <= 0.0 && e.dist > 1.0 {
                            e.timers[qi] -= dt;
                            if e.timers[qi] <= 0.0 {
                                e.timers[qi] = every;
                                quirks.push((i, *q));
                            }
                        }
                    }
                    _ => {}
                }
            }
            if e.stun > 0.0 {
                e.stun -= dt;
            } else if e.fear > 0.0 {
                e.fear -= dt;
                e.dist = (e.dist - d.speed * 1.2 * dt).max(0.0);
            } else if e.held.is_none() {
                e.dist += d.speed * (1.0 - e.slowed()) * dt;
            }
            e.field_slow = 0.0;
            let (p, dir) = self.map.routes[e.route].at(e.dist);
            e.dir = dir;
            e.pos = p + dir.perp() * e.lane;
            if e.dist >= route_len && e.hp > 0.0 {
                leaks.push(i);
            }
        }
        for i in leaks {
            let e = &mut self.foes[i];
            e.leaked = true;
            e.hp = 0.0;
            if let Some(n) = self.leaks.get_mut(e.wave) {
                *n += 1;
            }
            let lost = (e.foe.def().hearts as i32).min(self.hearts.max(0));
            self.hearts -= e.foe.def().hearts as i32;
            self.events.push(Ev::Leak { at: e.pos, foe: e.foe, hearts: lost });
        }
        for (i, q) in quirks {
            let (pos, route, dist, tough) = (self.foes[i].pos, self.foes[i].route, self.foes[i].dist, self.foes[i].tough);
            match q {
                Quirk::Jam { radius, secs, .. } => {
                    let mut any = false;
                    for t in self.towers.iter_mut().flatten() {
                        if t.pos.dist(pos) <= radius {
                            t.jam = t.jam.max(secs);
                            any = true;
                        }
                    }
                    if any {
                        self.events.push(Ev::Jam { at: pos, radius });
                    }
                }
                Quirk::Brood { what, n, .. } => {
                    for k in 0..n {
                        self.spawn(what, route, (dist - 0.4 - 0.35 * k as f32).max(0.0), tough);
                    }
                }
                _ => {}
            }
        }

        // Held invaders chew on whoever is holding them.
        for i in 0..self.foes.len() {
            let (held, bite, stunned, pos) = {
                let e = &self.foes[i];
                (e.held, e.foe.def().bite, e.stun > 0.0 || e.hp <= 0.0, e.pos)
            };
            match held {
                Some(Hold::Unit(u)) => {
                    if self.units[u].st.has(Flag::Thorns) {
                        self.hurt_flat(i, 8.0 * dt, Src::Unit(u));
                    }
                    if stunned || bite <= 0.0 {
                        continue;
                    }
                    self.foes[i].bite_cd -= dt;
                    if self.foes[i].bite_cd <= 0.0 {
                        self.foes[i].bite_cd = BITE_EVERY;
                        let un = &mut self.units[u];
                        if un.shell <= 0.0 && un.stand <= 0.0 {
                            let raw = bite * BITE_EVERY;
                            un.hp -= (raw - un.st.armor).max(raw * 0.2);
                            un.calm = 0.0;
                        }
                        un.face = if pos.x >= un.pos.x { 1.0 } else { -1.0 };
                    }
                }
                Some(Hold::Block(id)) => {
                    let splinters = match self.blocks.iter_mut().find(|b| b.id == id) {
                        Some(b) => {
                            if !stunned {
                                b.hp -= bite * dt;
                            }
                            b.splinters
                        }
                        None => false,
                    };
                    if splinters {
                        self.hurt_flat(i, 10.0 * dt, Src::Other);
                    }
                }
                None => {}
            }
        }
        for u in 0..self.units.len() {
            if self.units[u].alive() && self.units[u].hp <= 0.0 {
                let un = &mut self.units[u];
                if un.st.has(Flag::LastStand) && un.stand_cd <= 0.0 {
                    un.hp = 1.0;
                    un.stand = 6.0;
                    un.stand_cd = 45.0;
                    self.events.push(Ev::Stand { unit: u });
                } else {
                    un.hp = 0.0;
                    un.ko = un.st.revive;
                    un.shell = 0.0;
                    self.events.push(Ev::Ko { unit: u });
                }
            }
        }
    }

    /// Takes health off an invader. `pierce` ignores armor. Returns what
    /// actually got through.
    fn hurt(&mut self, i: usize, raw: f32, pierce: bool, src: Src) -> f32 {
        let e = &mut self.foes[i];
        if e.hp <= 0.0 {
            return 0.0;
        }
        let mut d = raw;
        if e.mark_t > 0.0 {
            d *= 1.0 + e.mark;
        }
        if !pierce {
            let armor = (e.foe.def().armor - e.shred).max(0.0);
            d = (d - armor).max(d * 0.2);
        }
        let d = d.min(e.hp);
        e.hp -= d;
        e.hurt_t = 0.0;
        e.by = src;
        match src {
            Src::Tower(_) => self.dealt[0] += d,
            _ => self.dealt[1] += d,
        }
        d
    }

    /// A trickle of damage (thorns, splinters, grit) that armor is no help against.
    fn hurt_flat(&mut self, i: usize, d: f32, src: Src) {
        let e = &mut self.foes[i];
        if e.hp > 0.0 {
            let d = d.min(e.hp);
            e.hp -= d;
            e.by = src;
            match src {
                Src::Tower(_) => self.dealt[0] += d,
                _ => self.dealt[1] += d,
            }
        }
    }

    /// Pushes an invader back along its route. Bosses barely notice.
    fn shove(&mut self, i: usize, tiles: f32) {
        let e = &mut self.foes[i];
        let k = if e.foe.def().boss { 0.25 } else { 1.0 };
        e.dist = (e.dist - tiles * k).max(0.0);
        e.held = None;
    }

    fn stun(&mut self, i: usize, secs: f32) {
        let e = &mut self.foes[i];
        let k = if e.foe.def().boss { 0.5 } else { 1.0 };
        e.stun = e.stun.max(secs * k);
    }

    fn chill(&mut self, i: usize, frac: f32, secs: f32) {
        let e = &mut self.foes[i];
        let f = if e.foe.def().boss { frac * 0.5 } else { frac };
        if e.slow_t <= 0.0 || f >= e.slow {
            e.slow = f;
        }
        e.slow_t = e.slow_t.max(secs);
    }

    /// Lands a hit on one invader, with everything that rides on it.
    fn land(&mut self, i: usize, h: &Hit, src: Src) -> f32 {
        if self.foes[i].hp <= 0.0 {
            return 0.0;
        }
        let d = self.foes[i].foe.def();
        let mut dmg = h.dmg;
        match h.bonus {
            Bonus::Vs(class, k) if d.class == class => dmg *= k,
            Bonus::VsAir(k) if d.flying => dmg *= k,
            _ => {}
        }
        let dealt = self.hurt(i, dmg, h.pierce, src);
        if h.slow.0 > 0.0 {
            self.chill(i, h.slow.0, h.slow.1);
        }
        if h.poison.0 > 0.0 {
            let e = &mut self.foes[i];
            e.poison = e.poison.max(h.poison.0);
            e.poison_t = e.poison_t.max(h.poison.1);
        }
        if h.shred.0 > 0.0 {
            let e = &mut self.foes[i];
            e.shred = e.shred.max(h.shred.0);
            e.shred_t = e.shred_t.max(h.shred.1);
        }
        if h.mark.0 > 0.0 {
            let e = &mut self.foes[i];
            e.mark = e.mark.max(h.mark.0);
            e.mark_t = e.mark_t.max(h.mark.1);
        }
        if h.stun > 0.0 {
            self.stun(i, h.stun);
        }
        if h.shove > 0.0 && !d.boss {
            let e = &mut self.foes[i];
            e.dist = (e.dist - h.shove).max(0.0);
        }
        dealt
    }

    /// Lands a hit on everything within `h.splash` of a point.
    fn burst(&mut self, at: V2, h: &Hit, src: Src, skip: Option<u32>) -> f32 {
        let mut total = 0.0;
        for i in 0..self.foes.len() {
            let e = &self.foes[i];
            if Some(e.id) == skip || (e.foe.def().flying && !h.air) {
                continue;
            }
            if e.pos.dist(at) <= h.splash + e.foe.def().size * 0.3 {
                total += self.land(i, h, src);
            }
        }
        total
    }

    /// Clears away invaders that have been shooed (or got through).
    fn reap(&mut self) {
        let mut i = 0;
        while i < self.foes.len() {
            if self.foes[i].hp > 0.0 {
                i += 1;
                continue;
            }
            let e = self.foes.swap_remove(i);
            if e.leaked {
                continue;
            }
            // fractions of a seed carry over to the next invader
            let exact = e.seeds * (1.0 + self.perks.bounty) + self.seed_frac;
            let seeds = exact.floor() as u32;
            self.seed_frac = exact - seeds as f32;
            self.gain(seeds);
            self.shooed += 1;
            self.events.push(Ev::Shooed { at: e.pos, foe: e.foe, seeds });
            if let Src::Unit(u) = e.by {
                let un = &mut self.units[u];
                if un.st.has(Flag::Frenzy) {
                    un.frenzy = (un.frenzy + 1).min(5);
                    un.frenzy_t = 6.0;
                    un.cd = (un.cd - 2.0).max(0.0);
                }
            }
            for q in e.foe.def().quirks {
                if let Quirk::Split(what, n) = *q {
                    for k in 0..n {
                        self.spawn(what, e.route, (e.dist + 0.3 - 0.3 * k as f32).max(0.0), e.tough);
                    }
                }
            }
        }
    }

    /// The invader furthest along within `range` of a point.
    fn first_in(&self, from: V2, range: f32, air: bool, ground: bool, skip: &[u32]) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.foes.iter().enumerate() {
            let d = e.foe.def();
            if e.hp <= 0.0 || (d.flying && !air) || (!d.flying && !ground) || skip.contains(&e.id) {
                continue;
            }
            if e.pos.dist(from) > range + d.size * 0.25 {
                continue;
            }
            let left = self.map.routes[e.route].len - e.dist;
            if best.is_none_or(|b| left < b.1) {
                best = Some((i, left));
            }
        }
        best.map(|b| b.0)
    }

    fn nearest_to(&self, from: V2, range: f32, air: bool, skip: &[u32]) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, e) in self.foes.iter().enumerate() {
            if e.hp <= 0.0 || (e.foe.def().flying && !air) || skip.contains(&e.id) {
                continue;
            }
            let d = e.pos.dist(from);
            if d <= range && best.is_none_or(|b| d < b.1) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    fn find(&self, id: u32) -> Option<usize> {
        self.foes.iter().position(|e| e.id == id)
    }

    // ---- the pack ------------------------------------------------------------------

    fn step_units(&mut self, dt: f32) {
        let home = self.map.home;
        for i in 0..self.units.len() {
            {
                let u = &mut self.units[i];
                u.prev = u.pos;
                u.swung += dt;
                u.stand_cd = (u.stand_cd - dt).max(0.0);
                if u.ko > 0.0 {
                    u.ko -= dt;
                    if u.ko <= 0.0 {
                        u.ko = 0.0;
                        u.hp = u.st.hp;
                        u.pos = home;
                        u.prev = home;
                        u.calm = 0.0;
                        self.events.push(Ev::Up { unit: i });
                    }
                    continue;
                }
                u.cd = (u.cd - dt).max(0.0);
                u.atk_cd = (u.atk_cd - dt).max(0.0);
                u.shell = (u.shell - dt).max(0.0);
                u.calm += dt;
                if u.stand > 0.0 {
                    u.stand -= dt;
                    if u.stand <= 0.0 {
                        u.hp = u.st.hp;
                    }
                }
                if u.frenzy_t > 0.0 {
                    u.frenzy_t -= dt;
                    if u.frenzy_t <= 0.0 {
                        u.frenzy = 0;
                    }
                }
                if u.calm > 3.0 && u.hp < u.st.hp {
                    u.hp = (u.hp + u.st.hp * 0.05 * u.st.regen * dt).min(u.st.hp);
                }
            }
            if self.units[i].guest && self.units[i].cd <= 0.0 && self.worth_casting(i) {
                self.cast(i);
            }
            if self.units[i].who.def().perched {
                self.step_perched(i, dt);
            } else {
                self.step_fighter(i, dt);
            }
        }
    }

    /// Someone who fights from a perch: goes where sent, shoots what is in range.
    fn step_perched(&mut self, i: usize, dt: f32) {
        let (pos, post, speed, range, ready) = {
            let u = &self.units[i];
            (u.pos, u.post, u.st.speed, u.st.range, u.atk_cd <= 0.0)
        };
        if pos != post {
            let u = &mut self.units[i];
            if (post.x - pos.x).abs() > 0.05 {
                u.face = (post.x - pos.x).signum();
            }
            u.pos.approach(post, speed * dt);
            if u.pos == post {
                u.ordered = false;
            }
        }
        if !ready {
            return;
        }
        let Some(first) = self.first_in(pos, range, true, true, &[]) else { return };
        let mut targets = vec![first];
        let mut seen = vec![self.foes[first].id];
        for _ in 0..self.units[i].st.extra {
            match self.first_in(pos, range, true, true, &seen) {
                Some(t) => {
                    seen.push(self.foes[t].id);
                    targets.push(t);
                }
                None => break,
            }
        }
        let tpos = self.foes[first].pos;
        let (hit, dive, look) = {
            let u = &mut self.units[i];
            u.atk_cd = 1.0 / u.st.rate.max(0.05);
            u.swung = 0.0;
            u.strikes += 1;
            if (tpos.x - pos.x).abs() > 0.05 {
                u.face = (tpos.x - pos.x).signum();
            }
            let dive = u.st.has(Flag::Dive) && u.strikes % 4 == 0;
            (Sim::unit_hit(&u.st, &mut self.rng), dive, u.who.def().look)
        };
        if dive {
            let mut h = hit;
            h.dmg *= 4.0;
            h.splash = 0.9;
            h.air = true;
            self.events.push(Ev::Beam { look: Look::Dive, a: pos, b: tpos });
            self.events.push(Ev::Burst { at: tpos, radius: 0.9, look: Look::Dive });
            self.burst(tpos, &h, Src::Unit(i), None);
            return;
        }
        for t in targets {
            let (id, to) = (self.foes[t].id, self.foes[t].pos);
            self.shots.push(Shot { look, from: pos, to, pos, prev: pos, t: 0.0, dur: (pos.dist(to) / 9.0).max(0.08), lob: false, target: Some(id), hit, src: Src::Unit(i) });
        }
    }

    /// A creature's own attack, with its skills riding on it.
    fn unit_hit(st: &Stats, rng: &mut Rng) -> Hit {
        let mut h = Hit::plain(st.dmg);
        if st.crit.0 > 0.0 && rng.chance(st.crit.0) {
            h.dmg *= st.crit.1;
            h.crit = true;
        }
        h.slow = st.slow;
        h.poison = st.poison;
        h.shred = st.shred;
        h.mark = st.mark;
        h.shove = st.shove;
        h
    }

    /// Someone who fights up close: stands where sent, steps out to meet
    /// whatever comes near, and holds up as many as it can.
    fn step_fighter(&mut self, i: usize, dt: f32) {
        let (pos, post, speed, reach, ordered, air) = {
            let u = &self.units[i];
            (u.pos, u.post, u.st.speed, u.st.range, u.ordered, u.who.def().air)
        };
        // told to go somewhere: go, and ignore everything on the way
        if ordered {
            let u = &mut self.units[i];
            if (post.x - pos.x).abs() > 0.05 {
                u.face = (post.x - pos.x).signum();
            }
            if u.pos.approach(post, speed * dt) {
                u.ordered = false;
            }
            return;
        }
        // otherwise: whoever I am holding, or failing that whoever is nearest my post
        let mut target: Option<usize> = None;
        let mut best = f32::MAX;
        for (ei, e) in self.foes.iter().enumerate() {
            if e.hp <= 0.0 || (e.foe.def().flying && !air) {
                continue;
            }
            let mine = e.held == Some(Hold::Unit(i));
            if !mine && e.pos.dist(post) > LEASH {
                continue;
            }
            let score = e.pos.dist(pos) - if mine { 100.0 } else { 0.0 };
            if score < best {
                best = score;
                target = Some(ei);
            }
        }
        let Some(t) = target else {
            if pos != post {
                let u = &mut self.units[i];
                if (post.x - pos.x).abs() > 0.05 {
                    u.face = (post.x - pos.x).signum();
                }
                u.pos.approach(post, speed * dt);
            }
            return;
        };
        let tpos = self.foes[t].pos;
        let gap = pos.dist(tpos);
        let touch = reach + self.foes[t].foe.def().size * 0.25;
        if gap > touch * 0.85 {
            // step toward it, but not off the leash
            let u = &mut self.units[i];
            let mut next = u.pos;
            next.approach(tpos, speed * dt);
            // invaders walk in from beyond the edge; the pack stays on the map
            let next = clamp_map(next);
            if next.dist(post) <= LEASH {
                u.pos = next;
            }
            if (tpos.x - pos.x).abs() > 0.05 {
                u.face = (tpos.x - pos.x).signum();
            }
        }
        if gap > touch || self.units[i].atk_cd > 0.0 {
            return;
        }
        // swing
        let (hit, extra, leech, splash) = {
            let u = &mut self.units[i];
            let mut rate = u.st.rate * (1.0 + 0.15 * u.frenzy as f32);
            let mut leech = u.st.leech;
            if u.st.has(Flag::Rampage) && u.holding >= 2 {
                rate *= 1.7;
                leech += 0.2;
            }
            u.atk_cd = 1.0 / rate.max(0.05);
            u.swung = 0.0;
            u.face = if tpos.x >= pos.x { 1.0 } else { -1.0 };
            (Sim::unit_hit(&u.st, &mut self.rng), u.st.extra, leech, u.st.splash)
        };
        self.events.push(Ev::Strike { unit: i, at: tpos });
        let mut dealt = self.land(t, &hit, Src::Unit(i));
        let first = self.foes[t].id;
        if splash > 0.0 {
            let mut h = hit;
            h.splash = splash;
            dealt += self.burst(tpos, &h, Src::Unit(i), Some(first));
        }
        let mut seen = vec![first];
        for _ in 0..extra {
            match self.nearest_to(pos, touch + 0.3, air, &seen) {
                Some(o) => {
                    seen.push(self.foes[o].id);
                    dealt += self.land(o, &hit, Src::Unit(i));
                }
                None => break,
            }
        }
        if leech > 0.0 {
            let u = &mut self.units[i];
            u.hp = (u.hp + dealt * leech).min(u.st.hp);
        }
    }

    /// Is it a good moment for this creature to use its ability? Residents
    /// (and the stand-in player) decide with this.
    pub fn worth_casting(&self, i: usize) -> bool {
        let u = &self.units[i];
        if !u.alive() || u.cd > 0.0 {
            return false;
        }
        let near = |r: f32, air: bool| {
            let mut n = 0;
            for e in &self.foes {
                if e.hp > 0.0 && (air || !e.foe.def().flying) && e.pos.dist(u.pos) <= r {
                    n += if e.foe.def().boss { 3 } else { 1 };
                }
            }
            n
        };
        match u.who.def().ability {
            Ability::Pounce { range, .. } => {
                // the leap goes to the toughest thing in reach: only worth it
                // if that is close to where he was told to stand
                let toughest = self.foes.iter().filter(|e| e.hp > 0.0 && e.pos.dist(u.pos) <= range * u.st.area).max_by(|a, b| a.hp.partial_cmp(&b.hp).unwrap());
                toughest.is_some_and(|e| (e.hp > 40.0 || e.foe.def().boss) && e.pos.dist(u.post) <= 2.4)
            }
            Ability::Slam { radius, .. } | Ability::Thump { radius, .. } => near(radius * u.st.area, false) >= 2,
            Ability::Gust { radius, .. } => near(radius * u.st.area, true) >= 3,
            Ability::Storm { range, .. } => near(range * u.st.area, true) >= 3,
            Ability::Dam { .. } => near(2.2, false) >= 3,
            Ability::Shell { .. } => u.holding >= 2 && u.hp < u.st.hp * 0.6,
            Ability::Whistle { radius, .. } => {
                let r = radius * u.st.area;
                self.towers.iter().flatten().filter(|t| t.pos.dist(u.pos) <= r).count() >= 2 && self.foes.len() >= 4
            }
        }
    }

    /// Uses a creature's ability. Returns false if it is not ready, or
    /// there is nothing for it to do.
    pub fn cast(&mut self, i: usize) -> bool {
        if !self.playing() || i >= self.units.len() || !self.units[i].alive() || self.units[i].cd > 0.0 {
            return false;
        }
        let (who, pos, st) = {
            let u = &self.units[i];
            (u.who, u.pos, u.st.clone())
        };
        let src = Src::Unit(i);
        match who.def().ability {
            Ability::Pounce { range, dmg, stun } => {
                let reach = range * st.area;
                // the toughest thing in reach
                let mut best: Option<(usize, f32)> = None;
                for (ei, e) in self.foes.iter().enumerate() {
                    if e.hp > 0.0 && e.pos.dist(pos) <= reach && best.is_none_or(|b| e.hp > b.1) {
                        best = Some((ei, e.hp));
                    }
                }
                let Some((mut t, _)) = best else { return false };
                let mut from = pos;
                let mut seen = Vec::new();
                let hops = if st.has(Flag::Bounce) { 3 } else { 1 };
                for hop in 0..hops {
                    let to = self.foes[t].pos;
                    seen.push(self.foes[t].id);
                    self.events.push(Ev::Leap { unit: i, from, to });
                    let mut h = Hit::plain(dmg * st.power * if hop == 0 { 1.0 } else { 0.6 });
                    h.stun = stun;
                    self.land(t, &h, src);
                    if st.has(Flag::Foxfire) {
                        let mut f = Hit::plain(h.dmg * 0.6);
                        f.splash = 1.1;
                        f.air = true;
                        f.poison = (10.0, 3.0);
                        f.bonus = Bonus::Vs(Class::Plant, 1.5);
                        self.burst(to, &f, src, None);
                        self.events.push(Ev::Burst { at: to, radius: 1.1, look: Look::Fang });
                    }
                    from = to;
                    match self.nearest_to(to, 2.6, true, &seen) {
                        Some(next) if hop + 1 < hops => t = next,
                        _ => break,
                    }
                }
                if st.has(Flag::Decoy) {
                    self.add_block(BlockKind::Decoy, pos, 200.0, 5.0, 3, false, false);
                }
                let u = &mut self.units[i];
                u.face = if from.x >= pos.x { 1.0 } else { -1.0 };
                // lands on the last one, then makes his own way back
                u.pos = clamp_map(from + v2(-0.35 * u.face, 0.0));
                u.prev = u.pos;
                u.atk_cd = 0.2;
                self.events.push(Ev::Cast { unit: i, who, at: from, radius: 0.8 });
            }
            Ability::Slam { radius, dmg, stun } => {
                let r = radius * st.area;
                let mut h = Hit::plain(dmg * st.power);
                h.splash = r;
                h.stun = stun;
                self.burst(pos, &h, src, None);
                if st.has(Flag::Sinkhole) {
                    for ei in 0..self.foes.len() {
                        if !self.foes[ei].foe.def().flying && self.foes[ei].pos.dist(pos) <= r + 0.2 {
                            self.shove(ei, 1.5);
                        }
                    }
                }
                if st.has(Flag::Aftershock) {
                    self.zones.push(Zone { kind: ZoneKind::Crack(0.45), pos, radius: r, t: 4.0, tick: 0.0 });
                }
                self.events.push(Ev::Cast { unit: i, who, at: pos, radius: r });
            }
            Ability::Gust { radius, push, dmg } => {
                let r = radius * st.area;
                let mut any = false;
                for ei in 0..self.foes.len() {
                    if self.foes[ei].hp > 0.0 && self.foes[ei].pos.dist(pos) <= r + self.foes[ei].foe.def().size * 0.25 {
                        any = true;
                        self.hurt(ei, dmg * st.power, false, src);
                        self.shove(ei, push * st.power);
                        if st.has(Flag::ColdFront) {
                            self.chill(ei, 0.45, 4.0);
                        }
                    }
                }
                if !any {
                    return false;
                }
                if st.has(Flag::Twister) {
                    self.zones.push(Zone { kind: ZoneKind::Twister(0.55), pos, radius: r, t: 4.0, tick: 1.0 });
                }
                self.events.push(Ev::Cast { unit: i, who, at: pos, radius: r });
            }
            Ability::Thump { radius, stun } => {
                let r = radius * st.area;
                let mut h = Hit::plain(if st.has(Flag::Quake) { 60.0 * st.power } else { 0.0 });
                h.splash = r;
                h.stun = stun;
                self.burst(pos, &h, src, None);
                if st.has(Flag::Cousins) {
                    let (route, along, _) = self.map.nearest_trail(pos);
                    for k in [-0.45f32, 0.45] {
                        let p = self.map.routes[route].at((along + k).max(0.3)).0;
                        self.add_block(BlockKind::Cousin, p, 160.0, 6.0, 1, false, false);
                    }
                }
                self.events.push(Ev::Cast { unit: i, who, at: pos, radius: r });
            }
            Ability::Storm { range, count, dmg } => {
                let r = range * st.area;
                let ids: Vec<usize> = (0..self.foes.len()).filter(|&ei| self.foes[ei].hp > 0.0 && self.foes[ei].pos.dist(pos) <= r).collect();
                if ids.is_empty() {
                    return false;
                }
                let n = count as usize + if st.has(Flag::BigStorm) { 5 } else { 0 };
                let mut h = Hit::plain(dmg * st.power);
                h.air = true;
                h.splash = if st.has(Flag::Conkers) { 0.6 } else { 0.25 };
                if st.has(Flag::Conkers) {
                    h.stun = 0.5;
                }
                for k in 0..n {
                    let e = &self.foes[ids[k % ids.len()]];
                    let to = e.pos + v2(self.rng.range(-0.15, 0.15), self.rng.range(-0.15, 0.15));
                    let from = to + v2(self.rng.range(-0.6, 0.6), -3.5);
                    // staggered, so they patter down rather than land as one
                    self.shots.push(Shot { look: Look::Acorn, from, to, pos: from, prev: from, t: -0.07 * k as f32, dur: 0.32, lob: true, target: None, hit: h, src });
                }
                self.events.push(Ev::Cast { unit: i, who, at: pos, radius: r });
            }
            Ability::Dam { hp, secs } => {
                let (route, along, off) = self.map.nearest_trail(pos);
                if off > 2.6 {
                    return false;
                }
                let p = self.map.routes[route].at(along).0;
                self.add_block(BlockKind::Dam, p, hp * st.power, secs + st.last, 99, st.has(Flag::Splinters), st.has(Flag::Flood));
                self.events.push(Ev::Cast { unit: i, who, at: p, radius: 0.7 });
            }
            Ability::Shell { secs } => {
                let u = &mut self.units[i];
                u.shell = secs + st.last;
                if st.has(Flag::ShellHeal) {
                    u.hp = u.st.hp;
                }
                self.events.push(Ev::Cast { unit: i, who, at: pos, radius: 0.8 });
            }
            Ability::Whistle { radius, haste, secs } => {
                let r = if st.has(Flag::Rally) { 99.0 } else { radius * st.area };
                for (spot, t) in self.towers.iter_mut().enumerate() {
                    if let Some(t) = t {
                        if t.pos.dist(pos) <= r {
                            t.haste = secs + st.last;
                            t.haste_amt = haste;
                            self.events.push(Ev::Haste { spot });
                        }
                    }
                }
                if st.has(Flag::Shrill) {
                    let reach = radius * st.area;
                    for ei in 0..self.foes.len() {
                        if self.foes[ei].foe.def().flying && self.foes[ei].pos.dist(pos) <= reach {
                            self.stun(ei, 2.0);
                        }
                    }
                }
                if st.has(Flag::Rally) {
                    for (o, u) in self.units.iter_mut().enumerate() {
                        if o != i {
                            u.cd = (u.cd - 4.0).max(0.0);
                        }
                    }
                }
                self.events.push(Ev::Cast { unit: i, who, at: pos, radius: radius * st.area });
            }
        }
        self.units[i].cd = st.cd;
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn add_block(&mut self, kind: BlockKind, pos: V2, hp: f32, secs: f32, slots: u8, splinters: bool, flood: bool) {
        let id = self.next_id;
        self.next_id += 1;
        self.blocks.push(Block { id, kind, pos, hp, max_hp: hp, t: secs, slots, holding: 0, splinters, flood });
        self.events.push(Ev::BlockUp { at: pos, kind });
    }

    // ---- structures ------------------------------------------------------------------

    /// What the pack standing nearby adds to a structure: (damage, rate,
    /// range), each as a fraction more.
    fn aura(&self, at: V2) -> (f32, f32, f32) {
        let mut a = (self.perks.all_dmg, 0.0, 0.0);
        for u in &self.units {
            if u.alive() && u.pos.dist(at) <= AURA_R {
                a.0 += u.st.aura_dmg;
                a.1 += u.st.aura_rate;
                a.2 += u.st.aura_range;
            }
        }
        a
    }

    fn step_towers(&mut self, dt: f32) {
        for spot in 0..self.towers.len() {
            let Some(t) = self.towers[spot].as_mut() else { continue };
            t.fired += dt;
            t.haste = (t.haste - dt).max(0.0);
            if t.jam > 0.0 {
                t.jam -= dt;
                continue;
            }
            let (kind, level, pos, haste) = (t.kind, t.level as usize, t.pos, if t.haste > 0.0 { t.haste_amt } else { 0.0 });
            let def = kind.def();
            let top = level == 3;
            let (a_dmg, a_rate, a_range) = self.aura(pos);
            let range = def.range * LV_RANGE[level - 1] * (1.0 + a_range);
            let dmg = def.dmg * LV_DMG[level - 1] * (1.0 + a_dmg);
            let rate = def.rate * LV_RATE[level - 1] * (1.0 + a_rate) * (1.0 + haste);
            let src = Src::Tower(spot);

            // a rattle that sends people running
            for r in def.riders {
                if let Rider::Fear { every, secs } = *r {
                    let dread = top && def.perk == Perk::Dread;
                    let t = self.towers[spot].as_mut().unwrap();
                    t.scare -= dt * (1.0 + haste);
                    if t.scare <= 0.0 {
                        let mut any = false;
                        for e in &mut self.foes {
                            let d = e.foe.def();
                            if e.hp > 0.0 && !d.boss && !d.flying && (d.class == Class::People || dread) && e.pos.dist(pos) <= range {
                                e.fear = secs + if dread { 1.0 } else { 0.0 };
                                e.held = None;
                                any = true;
                            }
                        }
                        if any {
                            self.towers[spot].as_mut().unwrap().scare = every;
                            self.events.push(Ev::Scare { at: pos, radius: range });
                        }
                    }
                }
            }

            match def.mode {
                Mode::Field { slow } => {
                    let s = slow + LV_SLOW * (level as f32 - 1.0);
                    let air = def.air || (top && def.perk == Perk::SlowAir);
                    let mut shred = 0.0;
                    for r in def.riders {
                        if let Rider::Shred(a, _) = *r {
                            shred = a * if top && def.perk == Perk::Deep { 2.0 } else { 1.0 };
                        }
                    }
                    if top {
                        if let Perk::Shred(a) = def.perk {
                            shred = a;
                        }
                    }
                    let root = top && def.perk == Perk::Root;
                    let mut pulse = false;
                    if root {
                        let t = self.towers[spot].as_mut().unwrap();
                        t.pulse -= dt;
                        if t.pulse <= 0.0 {
                            pulse = true;
                        }
                    }
                    let mut any = false;
                    for i in 0..self.foes.len() {
                        let e = &mut self.foes[i];
                        let d = e.foe.def();
                        if e.hp <= 0.0 || (d.flying && !air) || e.pos.dist(pos) > range + d.size * 0.25 {
                            continue;
                        }
                        any = true;
                        e.field_slow = e.field_slow.max(if d.boss { s * 0.5 } else { s });
                        if shred > 0.0 {
                            e.shred = e.shred.max(shred);
                            e.shred_t = e.shred_t.max(0.5);
                        }
                        if dmg > 0.0 {
                            self.hurt_flat(i, dmg * dt, src);
                        }
                        if pulse {
                            self.stun(i, 0.8);
                        }
                    }
                    if pulse && any {
                        self.towers[spot].as_mut().unwrap().pulse = 5.0;
                        self.events.push(Ev::Root { at: pos, radius: range });
                    }
                }
                _ => {
                    let t = self.towers[spot].as_mut().unwrap();
                    t.cd -= dt;
                    if t.cd > 0.0 {
                        continue;
                    }
                    let Some(first) = self.first_in(pos, range, def.air, def.ground, &[]) else { continue };
                    let t = self.towers[spot].as_mut().unwrap();
                    t.cd = 1.0 / rate.max(0.05);
                    t.fired = 0.0;
                    let aim = self.foes[first].pos;
                    self.towers[spot].as_mut().unwrap().aim = aim;
                    self.events.push(Ev::Fire { spot, to: aim });

                    let mut hit = Hit::plain(dmg);
                    hit.pierce = def.pierce;
                    hit.bonus = def.bonus;
                    for r in def.riders {
                        match *r {
                            Rider::Chill(a, b) => hit.slow = if top && def.perk == Perk::DeepChill { (0.45, b + 1.0) } else { (a, b) },
                            Rider::Poison(a, b) => hit.poison = (a, b),
                            Rider::Shred(a, b) => hit.shred = (a, b),
                            Rider::Fear { .. } => {}
                        }
                    }
                    if top {
                        match def.perk {
                            Perk::Pierce => hit.through = true,
                            Perk::Linger => hit.poison = (6.0, 3.0),
                            Perk::Stun(s) => hit.stun = s,
                            Perk::Bleed => hit.poison = (5.0, 3.0),
                            _ => {}
                        }
                    }
                    match def.mode {
                        Mode::Shot => {
                            let mut targets = vec![first];
                            if top && def.perk == Perk::Double {
                                if let Some(second) = self.first_in(pos, range, def.air, def.ground, &[self.foes[first].id]) {
                                    targets.push(second);
                                }
                            }
                            let speed = if def.look == Look::Needle { 13.0 } else { 9.5 };
                            for ti in targets {
                                let (id, to) = (self.foes[ti].id, self.foes[ti].pos);
                                self.shots.push(Shot { look: def.look, from: pos, to, pos, prev: pos, t: 0.0, dur: (pos.dist(to) / speed).max(0.06), lob: false, target: Some(id), hit, src });
                            }
                        }
                        Mode::Lob { splash } => {
                            hit.splash = splash * if top && def.perk == Perk::Wide { 1.3 } else { 1.0 };
                            let e = &self.foes[first];
                            let dur = 0.42 + pos.dist(e.pos) * 0.09;
                            // aim where it will be, not where it is
                            let moving = e.held.is_none() && e.stun <= 0.0 && e.fear <= 0.0;
                            let lead = if moving { e.foe.def().speed * (1.0 - e.slowed()) * dur } else { 0.0 };
                            let to = self.map.routes[e.route].at(e.dist + lead).0;
                            self.shots.push(Shot { look: def.look, from: pos, to, pos, prev: pos, t: 0.0, dur, lob: true, target: None, hit, src });
                        }
                        Mode::Swarm { targets } => {
                            let n = targets + if top { if let Perk::More(m) = def.perk { m } else { 0 } } else { 0 };
                            let mut seen: Vec<u32> = Vec::new();
                            let mut ti = Some(first);
                            for _ in 0..n {
                                let Some(cur) = ti else { break };
                                let (id, to) = (self.foes[cur].id, self.foes[cur].pos);
                                seen.push(id);
                                self.shots.push(Shot { look: def.look, from: pos, to, pos, prev: pos, t: 0.0, dur: (pos.dist(to) / 6.5).max(0.1), lob: false, target: Some(id), hit, src });
                                ti = self.first_in(pos, range, def.air, def.ground, &seen);
                            }
                        }
                        Mode::Snipe => {
                            let mut targets = vec![first];
                            if top && def.perk == Perk::Double {
                                if let Some(second) = self.first_in(pos, range, def.air, def.ground, &[self.foes[first].id]) {
                                    targets.push(second);
                                }
                            }
                            for ti in targets {
                                let to = self.foes[ti].pos;
                                self.events.push(Ev::Beam { look: def.look, a: pos, b: to });
                                let e = &self.foes[ti];
                                let d = e.foe.def();
                                let gulp = match def.perk {
                                    Perk::Execute(frac) if top => d.class == Class::Insect && !d.boss && e.hp <= e.max_hp * frac,
                                    _ => false,
                                };
                                if gulp {
                                    let hp = self.foes[ti].hp;
                                    self.hurt_flat(ti, hp, src);
                                    self.events.push(Ev::Swallow { at: to });
                                } else {
                                    self.land(ti, &hit, src);
                                    self.events.push(Ev::Hit { at: to, look: def.look, crit: false });
                                }
                            }
                        }
                        Mode::Chain { jumps } => {
                            let n = jumps + if top { if let Perk::Jumps(m) = def.perk { m } else { 0 } } else { 0 };
                            let mut seen: Vec<u32> = Vec::new();
                            let mut from = pos;
                            let mut cur = first;
                            let mut h = hit;
                            for hop in 0..=n {
                                let to = self.foes[cur].pos;
                                seen.push(self.foes[cur].id);
                                self.events.push(Ev::Beam { look: def.look, a: from, b: to });
                                self.land(cur, &h, src);
                                from = to;
                                h.dmg *= 0.8;
                                if hop == n {
                                    break;
                                }
                                match self.nearest_to(to, 1.5, def.air, &seen) {
                                    Some(next) => cur = next,
                                    None => break,
                                }
                            }
                        }
                        Mode::Field { .. } => {}
                    }
                }
            }
        }
    }

    fn step_shots(&mut self, dt: f32) {
        let mut landed: Vec<Shot> = Vec::new();
        let mut i = 0;
        while i < self.shots.len() {
            let target_pos = self.shots[i].target.and_then(|id| self.find(id)).map(|ei| self.foes[ei].pos);
            let s = &mut self.shots[i];
            s.prev = s.pos;
            s.t += dt;
            if let Some(p) = target_pos {
                s.to = p;
            }
            let k = (s.t / s.dur).clamp(0.0, 1.0);
            s.pos = s.from.lerp(s.to, k);
            if s.t >= s.dur {
                landed.push(self.shots.swap_remove(i));
            } else {
                i += 1;
            }
        }
        for s in landed {
            if s.lob {
                self.events.push(Ev::Burst { at: s.to, radius: s.hit.splash, look: s.look });
                self.burst(s.to, &s.hit, s.src, None);
                continue;
            }
            // a homing shot whose target has gone just lands where it was going
            let Some(ei) = s.target.and_then(|id| self.find(id)) else { continue };
            self.land(ei, &s.hit, s.src);
            self.events.push(Ev::Hit { at: s.to, look: s.look, crit: s.hit.crit });
            if s.hit.through {
                let id = self.foes[ei].id;
                if let Some(o) = self.nearest_to(s.to, 1.1, true, &[id]) {
                    let at = self.foes[o].pos;
                    self.land(o, &s.hit, s.src);
                    self.events.push(Ev::Hit { at, look: s.look, crit: false });
                }
            }
        }
    }
}

fn new_unit(who: Cr, st: Stats, guest: bool, pos: V2) -> Unit {
    Unit {
        who,
        hp: st.hp,
        st,
        guest,
        pos,
        prev: pos,
        post: pos,
        ko: 0.0,
        atk_cd: 0.5,
        cd: 3.0,
        ordered: false,
        face: 1.0,
        shell: 0.0,
        frenzy: 0,
        frenzy_t: 0.0,
        strikes: 0,
        stand_cd: 0.0,
        stand: 0.0,
        calm: 9.0,
        swung: 9.0,
        holding: 0,
    }
}

/// Keeps a point on the map, a little in from the edges.
pub fn clamp_map(p: V2) -> V2 {
    v2(p.x.clamp(0.3, COLS as f32 - 0.3), p.y.clamp(0.3, ROWS as f32 - 0.3))
}
