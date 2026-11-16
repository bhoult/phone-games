//! A stand-in player, for testing stages and setting how tough they are.
//! It plays sensibly but plainly: builds on the mounds that see the most
//! trail, upgrades once the mounds are full, parks the pack near the home
//! and uses abilities when there is a crowd. It never calls a wave early.

use crate::campaign::{Profile, FOLLOWERS};
use crate::creature::{Cr, FRIENDS};
use crate::defs::{Mode, Tw, BIOMES};
use crate::level::{is_finale, PER_BIOME};
use crate::sim::{Phase, Sim};
use crate::v2::V2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// Builds, upgrades, uses abilities.
    Careful,
    /// Watches. Every stage should be lost this way.
    Idle,
}

pub struct Bot {
    pub style: Style,
    think: f32,
    placed: bool,
    built: usize,
    /// Mounds, best first.
    order: Vec<usize>,
}

impl Bot {
    pub fn new(style: Style) -> Bot {
        Bot { style, think: 0.0, placed: false, built: 0, order: Vec::new() }
    }

    /// Call once per tick.
    pub fn act(&mut self, sim: &mut Sim) {
        if self.style == Style::Idle {
            if sim.phase == Phase::Ready {
                sim.call();
            }
            return;
        }
        self.think -= crate::sim::TICK;
        if self.think > 0.0 {
            return;
        }
        self.think = 0.35;
        if !self.placed {
            self.placed = true;
            self.order = rank_mounds(sim);
            // The pack stands where the best mound is looking: fighters on
            // the trail in front of it, one behind the other, and anyone
            // perched just beside it.
            let best = sim.map.mounds[self.order[0]];
            let (route, along, _) = sim.map.nearest_trail(best);
            let n = sim.units.iter().filter(|u| !u.guest).count();
            let mut line = 0.0;
            for i in 0..n {
                let to: V2 = if sim.units[i].who.def().perched {
                    best + (sim.map.routes[route].at(along).0 - best).norm() * (0.45 + 0.1 * i as f32) + sim.map.routes[route].at(along).1 * (0.5 * i as f32 - 0.5)
                } else {
                    line += 0.75;
                    sim.map.routes[route].at(along + line - 0.75).0
                };
                sim.order(i, to);
            }
        }
        // spend
        loop {
            let empty = self.order.iter().copied().find(|&s| sim.towers[s].is_none());
            if let Some(spot) = empty {
                let kind = choose(sim, sim.map.nearest_trail(sim.map.mounds[spot]).2 <= 1.2);
                if sim.build(spot, kind) {
                    self.built += 1;
                    continue;
                }
                // with mounds still empty, only upgrade what is cheap
                if !self.upgrade(sim, sim.price(kind, 1) / 2) {
                    break;
                }
            } else if !self.upgrade(sim, u32::MAX) {
                break;
            }
        }
        if sim.phase == Phase::Ready {
            sim.call();
        }
        for i in 0..sim.units.len() {
            if !sim.units[i].guest && sim.worth_casting(i) {
                sim.cast(i);
            }
        }
    }

    /// Buys the cheapest upgrade going, if it costs no more than `limit`.
    fn upgrade(&mut self, sim: &mut Sim, limit: u32) -> bool {
        let mut best: Option<(usize, u32)> = None;
        for spot in 0..sim.towers.len() {
            if let Some(cost) = sim.upgrade_price(spot) {
                // fields gain least from levels: leave them for last
                let field = matches!(sim.towers[spot].as_ref().unwrap().kind.def().mode, Mode::Field { .. });
                let weighed = if field { cost * 2 } else { cost };
                if cost <= limit && cost <= sim.seeds && best.is_none_or(|b| weighed < b.1) {
                    best = Some((spot, weighed));
                }
            }
        }
        match best {
            Some((spot, _)) => sim.upgrade(spot),
            None => false,
        }
    }
}

/// What to build next: whichever structure does the most, per seed, to the
/// invaders of the next few waves. Structures of a kind already built
/// count for less, so the defence ends up mixed, and one slowing field
/// goes in for every few that do damage.
fn choose(sim: &Sim, close: bool) -> Tw {
    let towers = BIOMES[sim.def.biome].towers;
    let have = |k: Tw| sim.towers.iter().flatten().filter(|t| t.kind == k).count();
    let built = sim.towers.iter().flatten().count();
    let fields = sim.towers.iter().flatten().filter(|t| matches!(t.kind.def().mode, Mode::Field { .. })).count();
    if close && built >= 2 && fields * 4 < built {
        if let Some(f) = towers.iter().find(|t| matches!(t.def().mode, Mode::Field { .. })) {
            return *f;
        }
    }
    // Who is coming over the next few waves. Each wave counts the same
    // however big it is (nearer ones a little more), so the plan depends
    // on who is in a wave and in what proportion, never on its size.
    let mut mix: Vec<(crate::defs::Foe, f32)> = Vec::new();
    let from = sim.wave.min(sim.waves.len().saturating_sub(1));
    for (ahead, wave) in sim.waves.iter().skip(from).take(4).enumerate() {
        let whole: f32 = wave.groups.iter().map(|g| if g.foe.def().boss { 0.0 } else { g.foe.threat() * g.count as f32 }).sum::<f32>().max(0.01);
        for g in &wave.groups {
            let w = [1.0, 0.8, 0.6, 0.4][ahead] * if g.foe.def().boss { 0.8 } else { g.foe.threat() * g.count as f32 / whole };
            match mix.iter_mut().find(|(f, _)| *f == g.foe) {
                Some((_, x)) => *x += w,
                None => mix.push((g.foe, w)),
            }
        }
    }
    let total: f32 = mix.iter().map(|m| m.1).sum::<f32>().max(1.0);
    let mut best = (towers[0], f32::MIN);
    for &k in towers {
        let d = k.def();
        let many = match d.mode {
            Mode::Field { .. } => continue,
            Mode::Shot | Mode::Snipe => 1.0,
            Mode::Lob { .. } => 2.4,
            Mode::Swarm { targets } => targets as f32,
            Mode::Chain { jumps } => 1.0 + 0.8 * jumps as f32,
        };
        let mut worth = 0.0;
        for &(foe, share) in &mix {
            let f = foe.def();
            if f.flying && !d.air {
                continue;
            }
            let bonus = match d.bonus {
                crate::defs::Bonus::Vs(c, x) if c == f.class => x,
                crate::defs::Bonus::VsAir(x) if f.flying => x,
                _ => 1.0,
            };
            let raw = d.dmg * bonus;
            let hit = if d.pierce { raw } else { (raw - f.armor).max(raw * 0.2) };
            // a burst only finds a crowd when the invaders are small
            let crowd = if f.boss { 1.0 } else if f.size < 0.62 { many } else { 1.0 + (many - 1.0) * 0.5 };
            worth += share / total * hit * d.rate * crowd * (d.range / 2.2);
        }
        worth = worth / d.cost as f32 * 0.7f32.powi(have(k) as i32);
        if worth > best.1 {
            best = (k, worth);
        }
    }
    best.0
}

/// Mounds ordered by how much trail each can see, best first.
fn rank_mounds(sim: &Sim) -> Vec<usize> {
    let mut scored: Vec<(usize, f32)> = sim
        .map
        .mounds
        .iter()
        .enumerate()
        .map(|(i, &m)| {
            let mut score = 0.0;
            for route in &sim.map.routes {
                let mut d = 0.0;
                while d < route.len {
                    let p = route.at(d).0;
                    if p.dist(m) <= 2.1 {
                        // trail nearer the home matters a little more
                        score += (0.6 + 0.4 * d / route.len) / sim.map.routes.len() as f32;
                    }
                    d += 0.25;
                }
            }
            (i, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    scored.into_iter().map(|s| s.0).collect()
}

/// The state a player would plausibly be in on reaching a stage: every
/// earlier stage cleared, the two newest friends walking along, points
/// spent down one branch and then the next.
pub fn expected_profile(level: usize, hero: Cr) -> Profile {
    let mut p = Profile::new(hero);
    for l in 0..level {
        p.record(l, 3);
        if is_finale(l) {
            if let Some(who) = BIOMES[l / PER_BIOME].resident {
                let slot = p.free_slot().unwrap_or((l / PER_BIOME) % FOLLOWERS);
                p.join(who, slot);
            }
        }
        spend(&mut p);
    }
    p
}

/// Spends whatever points are free: the player's creature down its
/// branches in order, followers turn and turn about.
pub fn spend(p: &mut Profile) {
    let hero = p.hero;
    'hero: loop {
        for b in 0..hero.def().tree.len() {
            for t in 0..hero.tiers() {
                if !p.knows(hero, b, t) {
                    if p.learn(hero, b, t).is_ok() {
                        continue 'hero;
                    }
                    // cannot afford the next step of this branch: wait for it
                    break 'hero;
                }
            }
        }
        break;
    }
    loop {
        let mut any = false;
        for who in FRIENDS {
            if !p.in_party(who) {
                continue;
            }
            'one: for b in 0..who.def().tree.len() {
                for t in 0..who.tiers() {
                    if !p.knows(who, b, t) {
                        if p.learn(who, b, t).is_ok() {
                            any = true;
                        }
                        break 'one;
                    }
                }
            }
        }
        if !any {
            break;
        }
    }
}

/// Plays a stage to the end with the stand-in player. Returns the stage
/// as it finished (check `phase` and `stars()`).
pub fn play(mut sim: Sim, style: Style) -> Sim {
    let mut bot = Bot::new(style);
    // 20 minutes of game time is far longer than any stage lasts
    for _ in 0..(20.0 * 60.0 / crate::sim::TICK) as usize {
        bot.act(&mut sim);
        sim.tick();
        sim.events.clear();
        if !sim.playing() {
            break;
        }
    }
    sim
}

/// The structures a stage offers, for callers that only have the stage.
pub fn offered(sim: &Sim) -> &'static [Tw] {
    BIOMES[sim.def.biome].towers
}
