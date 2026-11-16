//! A stand-in player, for testing stages and setting how tough they are.
//! It plays sensibly but plainly: picks as many places to build as the
//! stage is sized for, each where it sees the most trail that nothing else
//! is already watching, builds on them, then upgrades. It parks the pack
//! beside the first of them and uses abilities when there is a crowd. It
//! never calls a wave early, never sells, and never squeezes extra
//! structures in: a person who places well should do better than it does.

use crate::campaign::{Profile, FOLLOWERS};
use crate::creature::{Cr, FRIENDS};
use crate::defs::{Mode, Tw, BIOMES};
use crate::level::{is_finale, PER_BIOME};
use crate::sim::{Phase, Sim, FOOTPRINT};
use crate::v2::{v2, V2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// Builds, upgrades, uses abilities.
    Careful,
    /// Never upgrades: spends everything on more structures, wherever one
    /// will fit. Not how anyone is meant to play; it is here to check that
    /// carpeting the trail is not simply better than tending what is built.
    Carpet,
    /// Watches. Every stage should be lost this way.
    Idle,
}

pub struct Bot {
    pub style: Style,
    think: f32,
    placed: bool,
    /// Where it means to build, in order.
    spots: Vec<V2>,
}

impl Bot {
    pub fn new(style: Style) -> Bot {
        Bot { style, think: 0.0, placed: false, spots: Vec::new() }
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
            self.spots = if self.style == Style::Carpet { plan(sim, 60) } else { plan_spots(sim) };
            // The pack stands where the first structure will be looking:
            // fighters on the trail in front of it, one behind the other,
            // and anyone perched just beside it.
            let best = self.spots.first().copied().unwrap_or(sim.map.home);
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
            let built = sim.towers.iter().flatten().count();
            if let Some(&pos) = self.spots.get(built) {
                let kind = choose(sim, sim.map.nearest_trail(pos).2 <= 1.2);
                if sim.build(kind, pos).is_some() {
                    continue;
                }
                // with places still empty, only upgrade what is cheap
                if self.style == Style::Carpet || !self.upgrade(sim, sim.price(kind, 1) / 2) {
                    break;
                }
            } else if self.style == Style::Carpet || !self.upgrade(sim, u32::MAX) {
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

/// Where to build: as many places as the stage is sized for, picked one at
/// a time from a half-tile grid. Each pick takes the open ground that sees
/// the most trail, counting trail that an earlier pick already watches for
/// less, so the defence spreads along the trail instead of piling up at
/// one bend.
pub fn plan_spots(sim: &Sim) -> Vec<V2> {
    plan(sim, sim.def.plots as usize)
}

/// The same, for any number of places.
pub fn plan(sim: &Sim, count: usize) -> Vec<V2> {
    const SEES: f32 = 2.1;
    let map = &sim.map;
    // points along every route, with how much each matters
    let mut trail: Vec<(V2, f32)> = Vec::new();
    for route in &map.routes {
        let mut d = 0.0;
        while d < route.len {
            // trail nearer the home matters a little more
            trail.push((route.at(d).0, (0.6 + 0.4 * d / route.len) / map.routes.len() as f32));
            d += 0.25;
        }
    }
    let mut open: Vec<V2> = Vec::new();
    for r in 0..crate::map::ROWS * 2 {
        for c in 0..crate::map::COLS * 2 {
            let p = v2(c as f32 * 0.5 + 0.25, r as f32 * 0.5 + 0.25);
            if map.open(p).is_ok() {
                open.push(p);
            }
        }
    }
    let mut watched = vec![0u32; trail.len()];
    let mut picks: Vec<V2> = Vec::new();
    for _ in 0..count {
        let mut best: Option<(V2, f32)> = None;
        for &p in &open {
            if picks.iter().any(|q| q.dist(p) < FOOTPRINT) {
                continue;
            }
            let mut score = 0.0;
            for (i, &(t, w)) in trail.iter().enumerate() {
                if t.dist(p) <= SEES {
                    score += w / (1 + watched[i]) as f32;
                }
            }
            // nearer the trail is better: slowing ground only reaches so far
            score *= 1.0 - 0.12 * map.nearest_trail(p).2;
            if best.is_none_or(|b| score > b.1 + 1e-6) {
                best = Some((p, score));
            }
        }
        let Some((p, _)) = best else { break };
        for (i, &(t, _)) in trail.iter().enumerate() {
            if t.dist(p) <= SEES {
                watched[i] += 1;
            }
        }
        picks.push(p);
    }
    picks
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
