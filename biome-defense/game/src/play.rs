//! A stage being played. The rules live in `biome_core::sim`; this screen
//! steps them, turns what they report into pictures and sound, and passes
//! on what the player does.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::content::{LOSE_WORDS, WIN_WORDS};
use crate::field::{draw_ground, Scenery, View};
use crate::fx::Fx;
use crate::gfx::*;
use crate::ui::*;
use crate::widgets::*;
use crate::{App, Nav};
use biome_core::bot::{plan_spots, Bot, Style};
use biome_core::creature::Cr;
use biome_core::defs::{Look, Mode, LV_DMG, LV_RANGE, LV_RATE, LV_SLOW};
use biome_core::map::Blocked;
use biome_core::sim::{BlockKind, Ev, Phase, Setup, Sim, ZoneKind, FOOTPRINT, LEASH, TICK};
use biome_core::{v2, Foe, Tw, BIOMES, V2};
use macroquad::prelude::{draw_circle, draw_circle_lines, draw_line, vec2, Color, Rect, Vec2};
use std::f32::consts::{PI, TAU};

const HUD_H: f32 = 46.0;
/// Who comes next.
const NEXT_H: f32 = 28.0;
/// What can be built, and the button that calls the next wave.
const TRAY_H: f32 = 62.0;
/// The pack's cards.
const PACK_H: f32 = 66.0;
/// How far above a finger a structure being placed floats, in tiles, so
/// the finger does not cover it.
const GHOST_LIFT: f32 = 1.15;

#[derive(Clone, Copy, PartialEq)]
enum Menu {
    None,
    Tower(usize),
    Foe(Foe),
}

struct Beam {
    look: Look,
    a: Vec2,
    b: Vec2,
    age: f32,
}

struct Leap {
    unit: usize,
    from: V2,
    to: V2,
    age: f32,
}

/// An invader that has been stopped, for the moment it is still in view.
struct Remain {
    foe: Foe,
    pos: V2,
    /// The way it was heading.
    dir: V2,
    age: f32,
}

struct Over {
    won: bool,
    stars: u8,
    t: f32,
    first: bool,
    word: &'static str,
    shown: u8,
}

enum Item {
    Prop(usize),
    Tower(usize),
    Home,
    Block(usize),
    Unit(usize),
    Foe(usize),
    Remain(usize),
}

pub struct Play {
    pub level: usize,
    sim: Sim,
    bot: Option<Bot>,
    auto: u8,
    acc: f32,
    speed: u8,
    paused: bool,
    fx: Fx,
    /// Dice for the look of things only; the rules have their own.
    rng: biome_core::Rng,
    sc: Scenery,
    pal: Palette,
    t: f32,
    /// The pack member orders go to.
    sel: usize,
    menu: Menu,
    /// The structure chosen from the tray, waiting for a place.
    placing: Option<Tw>,
    /// Where it would stand, while a finger is on the field.
    ghost: Option<V2>,
    /// The tray item the current press began on, and whether it was
    /// already chosen then (a second tap puts it back).
    tray_press: Option<(Tw, bool)>,
    /// How long the current press has lasted.
    hold: f32,
    /// Why the last place was refused, and how long to go on saying so.
    why: Option<(&'static str, f32)>,
    /// Half-tile points a structure could stand on, shown while placing.
    open: Vec<V2>,
    open_for: usize,
    /// Good places to build, for the test driver to aim at.
    hints: Vec<V2>,
    beams: Vec<Beam>,
    leaps: Vec<Leap>,
    remains: Vec<Remain>,
    /// Where creatures have been sent: (place, seconds left to show it).
    marks: Vec<(V2, f32)>,
    /// How many invaders have been stopped in quick succession (the sound climbs).
    run: u8,
    run_t: f32,
    banner: Option<(String, f32, f32)>,
    intro: f32,
    over: Option<Over>,
    seed_pop: f32,
    heart_pop: f32,
    home_hit: f32,
    drift_cd: f32,
    /// Whether this is the very first stage of a new journey: show the ropes.
    teach: bool,
    moved: bool,
    cast_once: bool,
    nope: f32,
}

fn refusal(b: Blocked) -> &'static str {
    match b {
        Blocked::OffMap => "Off the map",
        Blocked::Trail => "Too close to the trail",
        Blocked::Home => "Too close to the home",
        Blocked::Water => "Water",
        Blocked::Scenery => "Something is in the way",
        Blocked::Crowded => "Too close to another",
    }
}

impl Play {
    pub fn new(app: &mut App, level: usize) -> Play {
        let p = app.save.p();
        let setup = Setup { level, difficulty: p.difficulty, pack: p.pack(), guests: p.guests(level), seed: app.rng.next_u64() };
        let teach = level == 0 && p.stars[0] == 0;
        let sim = Sim::new(&setup);
        let pal = palette(sim.def.biome);
        let sc = Scenery::new(&sim.map, &pal, level, 77 + level as u64 * 13);
        // only this biome's ground stays decoded
        app.g.keep_only(&[pal.ground, pal.trail, tex::WATER]);
        let auto = app.q.num("auto").unwrap_or(0.0) as u8;
        let hints = plan_spots(&sim);
        app.au.track(Track::Defend);
        app.log(&format!("SCREEN play {} {}", level + 1, setup.difficulty.name()));
        Play {
            level,
            sim,
            bot: (auto > 0).then(|| Bot::new(Style::Careful)),
            auto,
            acc: 0.0,
            speed: 1,
            paused: false,
            fx: Fx::new(app.rng.next_u64()),
            rng: biome_core::Rng::new(app.rng.next_u64()),
            sc,
            pal,
            t: 0.0,
            sel: 0,
            menu: Menu::None,
            placing: None,
            ghost: None,
            tray_press: None,
            hold: 0.0,
            why: None,
            open: Vec::new(),
            open_for: usize::MAX,
            hints,
            beams: Vec::new(),
            leaps: Vec::new(),
            remains: Vec::new(),
            marks: Vec::new(),
            run: 0,
            run_t: 0.0,
            banner: None,
            intro: 0.0,
            over: None,
            seed_pop: 0.0,
            heart_pop: 0.0,
            home_hit: 0.0,
            drift_cd: 0.0,
            teach,
            moved: false,
            cast_once: false,
            nope: 0.0,
        }
    }

    /// The top and bottom of the part of the screen the field shows through.
    fn field_span(&self, app: &App) -> (f32, f32) {
        let u = app.u;
        (app.top + HUD_H * u, app.h - app.bot - (NEXT_H + TRAY_H + PACK_H) * u)
    }

    fn view(&self, app: &App) -> View {
        let (top, bottom) = self.field_span(app);
        View::fit(Rect::new(app.x0, top, app.pw, (bottom - top).max(50.0)))
    }

    fn say(&mut self, text: String, secs: f32) {
        self.banner = Some((text, secs, secs));
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        self.t += dt;
        let v = self.view(app);
        let mut nav = None;

        // ---- time ---------------------------------------------------------
        let busy = self.over.is_some() || self.paused;
        if !busy {
            self.acc = (self.acc + dt * self.speed as f32).min(TICK * 10.0);
            while self.acc >= TICK {
                self.acc -= TICK;
                if let Some(bot) = &mut self.bot {
                    bot.act(&mut self.sim);
                }
                self.sim.tick();
                let events: Vec<Ev> = self.sim.events.drain(..).collect();
                for ev in events {
                    self.on_event(app, &v, ev);
                }
            }
            let step = dt * self.speed as f32;
            self.remains.retain_mut(|r| {
                r.age += step;
                r.age < if r.foe.def().boss { 2.6 } else { 1.3 }
            });
        }
        self.fx.update(dt);
        self.run_t -= dt;
        if self.run_t <= 0.0 {
            self.run = 0;
        }
        self.seed_pop = (self.seed_pop - dt * 3.0).max(0.0);
        self.heart_pop = (self.heart_pop - dt * 2.5).max(0.0);
        self.home_hit = (self.home_hit - dt * 2.5).max(0.0);
        self.nope = (self.nope - dt * 3.0).max(0.0);
        self.beams.retain_mut(|b| {
            b.age += dt;
            b.age < 0.24
        });
        self.leaps.retain_mut(|l| {
            l.age += dt;
            l.age < 0.22
        });
        self.marks.retain_mut(|m| {
            m.1 -= dt;
            m.1 > 0.0
        });
        if let Some((_, life, _)) = &mut self.banner {
            *life -= dt;
            if *life <= 0.0 {
                self.banner = None;
            }
        }
        if let Some((_, life)) = &mut self.why {
            *life -= dt;
            if *life <= 0.0 {
                self.why = None;
            }
        }
        if self.sim.phase == Phase::Ready {
            self.intro += dt;
        }
        if app.ui.ptr.pressed {
            self.hold = 0.0;
        } else if app.ui.ptr.down {
            self.hold += dt;
        }
        self.ambient(app, &v, dt);
        let k = (self.acc / TICK).clamp(0.0, 1.0);

        // ---- the field ----------------------------------------------------
        let shake = self.fx.offset();
        let mut sv = v;
        sv.origin += shake;
        let full = Rect::new(app.x0 - 20.0, -20.0, app.pw + 40.0, app.h + 40.0);
        draw_ground(&app.g, &sv, &self.sc, full, self.t);
        self.draw_world(app, &sv, k);
        self.fx.draw(&app.g);
        if let Some((top, bottom)) = mood_wash(self.sim.def.mood) {
            app.g.vgrad(Rect::new(0.0, 0.0, app.w, app.h), top, bottom);
            self.draw_lights(app, &sv, k);
        }
        self.draw_bars(&sv, k);
        self.draw_ghost(app, &sv);
        self.fx.draw_text(&app.g);
        if self.home_hit > 0.0 {
            app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(BLOOD, 0.2 * self.home_hit));
        }
        // letterbox on wide screens
        if app.x0 > 1.0 {
            app.g.rect(Rect::new(0.0, 0.0, app.x0, app.h), SHADE);
            app.g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), SHADE);
        }

        // ---- the interface ------------------------------------------------
        let used = self.hud(app, &v) | self.bottom(app, &v);
        if !busy {
            if self.placing.is_some() {
                self.place_input(app, &v);
            } else if !used {
                self.field_taps(app, &v);
            }
        }
        if app.ui.ptr.released {
            self.tray_press = None;
        }
        self.draw_banners(app, &v);
        if self.teach && !busy {
            self.draw_hints(app, &v);
        }
        if app.debug && app.tick(20) {
            app.log(&format!("GRID {:.1}", v.cell));
            for (i, p) in self.hints.iter().enumerate() {
                let c = v.at(*p);
                app.log(&format!("SPOT {} {:.0} {:.0} {}", i, c.x, c.y, self.sim.can_build(*p).is_ok() as u8));
            }
            for (i, tw) in self.sim.towers.iter().enumerate() {
                if let Some(tw) = tw {
                    let c = v.at(tw.pos);
                    app.log(&format!("TOWER {} {:.0} {:.0} {}", i, c.x, c.y, tw.level));
                }
            }
            app.log(&format!(
                "STATE phase={:?} wave={}/{} hearts={} seeds={} foes={} towers={}",
                self.sim.phase,
                self.sim.wave,
                self.sim.waves.len(),
                self.sim.hearts,
                self.sim.seeds,
                self.sim.foes.len(),
                self.sim.towers.iter().flatten().count()
            ));
        }

        if self.paused {
            nav = self.pause_card(app);
        } else if self.over.is_some() {
            nav = self.over_card(app);
        }
        // Escape / back: pause
        if macroquad::prelude::is_key_pressed(macroquad::prelude::KeyCode::Escape) && self.over.is_none() {
            self.paused = !self.paused;
        }
        nav
    }

    // ---- what the rules report ----------------------------------------------

    fn dust(&mut self, c: Vec2, n: usize, size: f32, a: f32) {
        for _ in 0..n {
            let off = vec2(self.rng.range(-1.0, 1.0), self.rng.range(-0.6, 0.6)) * size * 0.7;
            self.fx.puff(c + off, size, vec2(0.0, -size * 0.3), alpha(mix(self.pal.dust, INK, 0.25), a));
        }
    }

    fn on_event(&mut self, app: &mut App, v: &View, ev: Ev) {
        let u = app.u;
        let cell = v.cell;
        match ev {
            Ev::Spawn { .. } => {}
            Ev::Fire { spot, .. } => {
                let look = self.sim.towers.get(spot).and_then(|t| t.as_ref()).map(|t| t.kind.def().look).unwrap_or(Look::None);
                let sfx = match look {
                    Look::Spore | Look::Stone | Look::Rock | Look::Snow => Sfx::Lob,
                    Look::Zap => Sfx::Zap,
                    Look::Peck | Look::Dive => Sfx::Peck,
                    Look::Tongue => Sfx::Tongue,
                    _ => Sfx::Dart,
                };
                app.au.play(sfx, 0.3);
            }
            Ev::Beam { look, a, b } => self.beams.push(Beam { look, a: v.at(a), b: v.at(b), age: 0.0 }),
            Ev::Hit { at, look, crit } => {
                let c = v.at(at);
                self.fx.burst(c, look_color(look), if crit { 6 } else { 2 }, 60.0 * u, 4.0 * u, 0.0);
                if crit {
                    self.fx.ring(c, cell * 0.1, cell * 0.45, 0.22, INK, 0.0);
                }
            }
            Ev::Burst { at, radius, look } => {
                let c = v.at(at);
                let col = look_color(look);
                self.fx.ring(c, cell * 0.2, cell * radius.max(0.4), 0.3, alpha(col, 0.8), 0.0);
                self.fx.burst(c, col, 7, 110.0 * u, 5.0 * u, 0.0);
                for _ in 0..3 {
                    self.fx.puff(c, 11.0 * u, vec2(0.0, 0.0), alpha(mix(col, self.pal.dust, 0.5), 0.6));
                }
                app.au.play(Sfx::Burst, 0.4);
                if radius > 0.9 {
                    self.fx.shake(2.0 * u);
                }
            }
            Ev::Strike { at, .. } => {
                let c = v.at(at);
                self.fx.add(crate::fx::Particle {
                    spr: spr::FX_SLASH,
                    pos: c - vec2(0.0, cell * 0.1),
                    vel: vec2(0.0, 0.0),
                    gravity: 0.0,
                    drag: 0.0,
                    rot: self.rng.range(-0.8, 0.8),
                    spin: 0.0,
                    size: cell * 0.55,
                    grow: 1.25,
                    age: 0.0,
                    life: 0.13,
                    color: alpha(INK, 0.85),
                });
            }
            Ev::Down { at, foe, seeds } => {
                let c = v.at(at);
                let d = foe.def();
                self.run = (self.run + 1).min(8);
                self.run_t = 0.5;
                app.au.play(Sfx::Down(self.run - 1), if d.boss { 0.9 } else { 0.4 });
                let (route, along, _) = self.sim.map.nearest_trail(at);
                let dir = self.sim.map.routes[route].at(along).1;
                self.remains.push(Remain { foe, pos: at, dir, age: 0.0 });
                if foe_end(foe) != End::Flees {
                    self.dust(c, if d.boss { 12 } else { 2 }, cell * 0.28 * d.size.max(0.6), 0.6);
                }
                if seeds > 0 {
                    self.fx.text(&format!("+{seeds}"), c - vec2(0.0, cell * 0.35), (if d.boss { 17.0 } else { 11.0 }) * u, GOLD, SHADE, 0.7);
                    self.seed_pop = 1.0;
                }
                if d.boss {
                    self.fx.shake(9.0 * u);
                    self.fx.ring(c, cell * 0.3, cell * 3.0, 0.7, INK, 0.0);
                    let how = if foe_end(foe) == End::Stalls { "IS STOPPED" } else { "FALLS" };
                    self.say(format!("{} {how}", d.name.to_uppercase()), 2.8);
                }
            }
            Ev::Leak { at, hearts, .. } => {
                let c = v.at(at);
                app.au.play(Sfx::Leak, 0.8);
                crate::platform::vibrate(40);
                self.heart_pop = 1.0;
                self.home_hit = 1.0;
                self.fx.shake(6.0 * u);
                self.fx.text(&format!("-{}", hearts.max(1)), c - vec2(0.0, cell * 0.9), 19.0 * u, BLOOD, SHADE, 0.9);
            }
            Ev::Cast { who, at, radius, .. } => {
                let c = v.at(at);
                let r = radius * cell;
                match who {
                    Cr::Fox => {
                        app.au.play(Sfx::Pounce, 0.8);
                        self.fx.ring(c, cell * 0.2, cell * 0.9, 0.3, cr_color(who), 0.0);
                        self.dust(c, 4, cell * 0.3, 0.7);
                    }
                    Cr::Badger => {
                        app.au.play(Sfx::Slam, 0.9);
                        self.fx.shake(10.0 * u);
                        self.fx.ring(c, cell * 0.3, r, 0.4, INK, 0.0);
                        for i in 0..14 {
                            let a = i as f32 / 14.0 * TAU;
                            self.fx.puff(c + vec2(a.cos(), a.sin() * 0.7) * r * 0.6, 12.0 * u, vec2(a.cos(), a.sin()) * 40.0 * u, alpha(mix(self.pal.dust, INK, 0.2), 0.8));
                        }
                    }
                    Cr::Owl => {
                        app.au.play(Sfx::Gust, 0.8);
                        self.fx.ring(c, cell * 0.3, r, 0.45, alpha(INK, 0.7), 0.0);
                        for i in 0..16 {
                            let a = i as f32 / 16.0 * TAU;
                            self.fx.drift(spr::STREAK, c + vec2(a.cos(), a.sin()) * cell * 0.3, vec2(a.cos(), a.sin()) * 190.0 * u, 14.0 * u, 0.4, alpha(INK, 0.6), 0.0);
                        }
                    }
                    Cr::Rabbit => {
                        app.au.play(Sfx::Thump, 0.9);
                        self.fx.shake(6.0 * u);
                        self.fx.ring(c, cell * 0.2, r, 0.35, INK, 0.0);
                        self.dust(c, 5, cell * 0.3, 0.7);
                    }
                    Cr::Squirrel => {
                        app.au.play(Sfx::Storm, 0.8);
                        self.fx.ring(c, cell * 0.3, r, 0.5, cr_color(who), 0.0);
                    }
                    Cr::Beaver => {
                        app.au.play(Sfx::Dam, 0.9);
                        self.fx.shake(4.0 * u);
                        self.fx.chunks(spr::P_BIT, c, rgb(0x7a5a3a), 10, 130.0 * u, 6.0 * u, 300.0 * u);
                    }
                    Cr::Tortoise => {
                        app.au.play(Sfx::Shell, 0.9);
                        self.fx.ring(c, cell * 0.2, cell * 0.9, 0.35, cr_color(who), 0.0);
                    }
                    Cr::Marmot => {
                        app.au.play(Sfx::Whistle, 0.8);
                        for i in 0..3 {
                            self.fx.ring(c, cell * 0.2, r, 0.6, alpha(GOLD, 0.8), i as f32 * 0.12);
                        }
                    }
                }
                self.cast_once = true;
            }
            Ev::Leap { unit, from, to } => self.leaps.push(Leap { unit, from, to, age: 0.0 }),
            Ev::Ko { .. } => app.au.play(Sfx::Ko, 0.7),
            Ev::Up { unit } => {
                let c = v.at(self.sim.units[unit].pos);
                app.au.play(Sfx::Up, 0.6);
                self.fx.ring(c, cell * 0.2, cell * 0.7, 0.4, MOSS, 0.0);
            }
            Ev::Stand { unit } => {
                let c = v.at(self.sim.units[unit].pos);
                app.au.play(Sfx::Slam, 0.6);
                self.fx.ring(c, cell * 0.3, cell * 1.4, 0.5, GOLD, 0.0);
            }
            Ev::Built { spot } => {
                if let Some(Some(tw)) = self.sim.towers.get(spot) {
                    let c = v.at(tw.pos);
                    app.au.play(Sfx::Build, 0.8);
                    crate::platform::vibrate(12);
                    for i in 0..8 {
                        let a = i as f32 / 8.0 * TAU;
                        self.fx.puff(c + vec2(a.cos(), a.sin() * 0.5) * cell * 0.4, 9.0 * u, vec2(a.cos() * 30.0, -10.0) * u, alpha(mix(self.pal.dust, INK, 0.3), 0.75));
                    }
                }
                app.log(&format!("BUILT {spot}"));
            }
            Ev::Upgraded { spot } => {
                if let Some(Some(tw)) = self.sim.towers.get(spot) {
                    let c = v.at(tw.pos);
                    app.au.play(Sfx::Upgrade, 0.8);
                    self.fx.ring(c, cell * 0.2, cell * 0.9, 0.4, GOLD, 0.0);
                    self.fx.text(if tw.level >= 3 { "III" } else { "II" }, c - vec2(0.0, cell * 0.9), 14.0 * u, GOLD, SHADE, 0.8);
                }
                app.log(&format!("UPGRADED {spot}"));
            }
            Ev::Sold { seeds, .. } => {
                app.au.play(Sfx::Sell, 0.8);
                self.seed_pop = 1.0;
                let at = vec2(app.x0 + 118.0 * u, app.top + 58.0 * u);
                self.fx.text(&format!("+{seeds}"), at, 13.0 * u, GOLD, SHADE, 0.9);
                app.log("SOLD");
            }
            Ev::Wave { n, seeds } => {
                app.au.play(Sfx::Wave, 0.8);
                let last = n == self.sim.waves.len();
                self.say(if last { "FINAL WAVE".to_string() } else { format!("WAVE {n}") }, 1.7);
                if seeds > 0 {
                    self.seed_pop = 1.0;
                    let at = vec2(app.x0 + 118.0 * u, app.top + 58.0 * u);
                    self.fx.text(&format!("+{seeds}"), at, 13.0 * u, GOLD, SHADE, 1.0);
                }
                app.log(&format!("WAVE {n}"));
            }
            Ev::Jam { at, radius } => {
                let c = v.at(at);
                app.au.play(Sfx::Jam, 0.7);
                self.fx.ring(c, cell * 0.3, radius * cell, 0.5, rgb(0xa8b878), 0.0);
                for _ in 0..10 {
                    let off = vec2(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)) * radius * cell * 0.6;
                    self.fx.puff(c + off, 14.0 * u, vec2(0.0, -12.0 * u), alpha(rgb(0xb8c890), 0.45));
                }
            }
            Ev::Scare { at, radius } => {
                let c = v.at(at);
                app.au.play(Sfx::Rattle, 0.7);
                self.fx.ring(c, cell * 0.3, radius * cell, 0.45, GOLD, 0.0);
            }
            Ev::Root { at, radius } => {
                let c = v.at(at);
                app.au.play(Sfx::Root, 0.6);
                self.fx.ring(c, cell * 0.3, radius * cell, 0.4, FROST, 0.0);
            }
            Ev::BlockUp { at, kind } => {
                if kind != BlockKind::Dam {
                    self.dust(v.at(at), 3, cell * 0.25, 0.6);
                }
            }
            Ev::BlockDown { at, kind, flood } => {
                let c = v.at(at);
                match kind {
                    BlockKind::Dam => {
                        self.fx.chunks(spr::P_BIT, c, rgb(0x7a5a3a), 12, 150.0 * u, 6.0 * u, 320.0 * u);
                        app.au.play(Sfx::Dam, 0.6);
                    }
                    _ => self.dust(c, 3, cell * 0.25, 0.6),
                }
                if flood {
                    self.fx.spray(c, 26, 220.0 * u, 6.0 * u);
                    self.fx.ring(c, cell * 0.3, cell * 1.7, 0.5, FROST, 0.0);
                    app.au.play(Sfx::Gust, 0.6);
                }
            }
            Ev::Swallow { at } => self.dust(v.at(at), 1, cell * 0.2, 0.5),
            Ev::Haste { spot } => {
                if let Some(Some(tw)) = self.sim.towers.get(spot) {
                    let c = v.at(tw.pos);
                    self.fx.ring(c, cell * 0.2, cell * 0.6, 0.3, alpha(GOLD, 0.7), 0.0);
                }
            }
            Ev::Mend => {
                self.heart_pop = 1.0;
                let c = v.at(self.sim.map.home);
                self.fx.text("+1", c - vec2(0.0, cell * 0.9), 15.0 * u, MOSS, SHADE, 1.0);
            }
            Ev::Boss { foe } => {
                app.au.play(Sfx::Boss, 1.0);
                crate::platform::vibrate(80);
                self.fx.shake(8.0 * u);
                self.say(foe.def().name.to_uppercase(), 3.0);
            }
            Ev::Won { stars } => self.finish(app, true, stars),
            Ev::Lost => self.finish(app, false, 0),
        }
    }

    fn finish(&mut self, app: &mut App, won: bool, stars: u8) {
        let mut first = false;
        if won {
            first = app.save.p_mut().record(self.level, stars);
            app.save.store();
            app.walk_from = Some(self.level);
            app.au.play(Sfx::Win, 0.9);
            app.log(&format!("WON {stars}"));
        } else {
            app.au.play(Sfx::Lose, 0.9);
            app.log("LOST");
        }
        app.au.set_music_gain(0.35);
        let words: &[&'static str] = if won { &WIN_WORDS } else { &LOSE_WORDS };
        self.over = Some(Over { won, stars, t: 0.0, first, word: words[self.rng.below(words.len() as u32) as usize], shown: 0 });
        self.menu = Menu::None;
        self.placing = None;
        self.ghost = None;
    }

    // ---- what drifts through the air -----------------------------------------

    fn ambient(&mut self, app: &App, v: &View, dt: f32) {
        self.drift_cd -= dt;
        if self.drift_cd > 0.0 {
            return;
        }
        let u = app.u;
        let r = v.rect();
        let x = r.x + self.rng.range(-0.1, 1.1) * r.w;
        let y = r.y + self.rng.unit() * r.h;
        match self.pal.drift {
            Drift::Seeds => {
                self.drift_cd = 0.9;
                self.fx.drift(spr::P_FLUFF, vec2(r.x - 8.0, y), vec2(self.rng.range(25.0, 50.0), self.rng.range(-8.0, 10.0)) * u, 6.0 * u, 8.0, alpha(INK, 0.5), 0.0);
            }
            Drift::Leaves => {
                self.drift_cd = 0.8;
                let col = [rgb(0x9a6a2a), rgb(0x8a4a2a), rgb(0x6a7a3a)][self.rng.below(3) as usize];
                self.fx.drift(spr::P_LEAF, vec2(x, r.y - 10.0), vec2(self.rng.range(-25.0, 25.0), self.rng.range(35.0, 60.0)) * u, 9.0 * u, 7.0, alpha(col, 0.85), 5.0 * u);
            }
            Drift::Midges => {
                self.drift_cd = 0.3;
                self.fx.drift(spr::P_DOT, vec2(x, y), vec2(self.rng.range(-30.0, 30.0), self.rng.range(-24.0, 14.0)) * u, 2.4 * u, 1.6, alpha(SHADE, 0.55), 0.0);
            }
            Drift::Dust => {
                self.drift_cd = 0.3;
                self.fx.drift(spr::P_DOT, vec2(r.x - 6.0, y), vec2(self.rng.range(70.0, 130.0), self.rng.range(-8.0, 8.0)) * u, 3.5 * u, 4.5, alpha(rgb(0xe8d0b0), 0.4), 0.0);
            }
            Drift::Snow => {
                self.drift_cd = 0.16;
                self.fx.drift(spr::P_SNOW, vec2(x, r.y - 10.0), vec2(self.rng.range(-20.0, 10.0), self.rng.range(35.0, 70.0)) * u, self.rng.range(3.5, 7.0) * u, 9.0, alpha(WHITE, 0.8), 0.0);
            }
            Drift::Smoke => {
                self.drift_cd = 0.7;
                self.fx.drift(spr::P_SMOKE, vec2(x, y), vec2(self.rng.range(10.0, 30.0), self.rng.range(-30.0, -12.0)) * u, 24.0 * u, 3.5, alpha(rgb(0x5a5a5a), 0.2), 0.0);
            }
        }
    }

    // ---- drawing -----------------------------------------------------------------

    fn unit_pos(&self, i: usize, k: f32) -> (V2, f32) {
        let un = &self.sim.units[i];
        if let Some(l) = self.leaps.iter().rev().find(|l| l.unit == i) {
            let t = (l.age / 0.22).clamp(0.0, 1.0);
            return (l.from.lerp(l.to, ease_io(t)), (t * PI).sin() * 0.9);
        }
        (un.prev.lerp(un.pos, k), 0.0)
    }

    /// The half-tile points where the chosen structure could stand right
    /// now: worked out again whenever what is built changes.
    fn open_ground(&mut self) {
        let stamp = self.sim.towers.iter().flatten().count() * 31 + self.sim.towers.len();
        if self.open_for == stamp {
            return;
        }
        self.open_for = stamp;
        self.open.clear();
        for r in 0..26 {
            for c in 0..18 {
                let p = v2(c as f32 * 0.5 + 0.25, r as f32 * 0.5 + 0.25);
                if self.sim.can_build(p).is_ok() {
                    self.open.push(p);
                }
            }
        }
    }

    fn draw_world(&mut self, app: &App, v: &View, k: f32) {
        if self.placing.is_some() {
            self.open_ground();
        }
        let g = &app.g;
        let sim = &self.sim;
        let cell = v.cell;
        let t = self.t;

        // where invaders come in
        for (i, &s) in sim.map.spawns.iter().enumerate() {
            let c = v.at(s);
            let coming = sim.upcoming().is_some_and(|w| w.groups.iter().any(|gr| gr.spawn as usize == i)) && sim.can_call();
            if coming {
                let pulse = 0.5 + 0.5 * (t * 3.0).sin();
                g.spr(spr::GLOW, c.x, c.y, cell * (1.7 + 0.3 * pulse), cell * (1.7 + 0.3 * pulse), 0.0, alpha(BLOOD, 0.3 + 0.2 * pulse));
                // pointing the way they will come
                let dir = sim.map.routes.iter().find(|r| r.spawn as usize == i).map(|r| r.at(1.2).1).unwrap_or(v2(0.0, 1.0));
                let slide = (t * 1.2).fract();
                let at = c + vec2(dir.x, dir.y) * cell * (slide - 0.3) * 0.5;
                g.spr_fit(spr::SPAWN_SIGN, at.x, at.y, cell * 0.46, dir.y.atan2(dir.x), alpha(BLOOD, (1.0 - (slide - 0.5).abs() * 2.0) * 0.9));
            }
        }

        // the ground stained by slowing fields, and lingering effects
        for (spot, tw) in sim.towers.iter().enumerate() {
            let Some(tw) = tw else { continue };
            if let Some(col) = field_color(tw.kind) {
                let c = v.at(tw.pos);
                let r = sim.reach(spot) * cell;
                let a = if tw.jam > 0.0 { 0.05 } else { 0.16 };
                draw_circle(c.x, c.y, r, alpha(col, a));
                draw_circle_lines(c.x, c.y, r, cell * 0.025, alpha(col, a * 2.2));
            }
        }
        for z in &sim.zones {
            let c = v.at(z.pos);
            let r = z.radius * cell;
            let fade = (z.t / 0.5).min(1.0);
            if let ZoneKind::Crack(_) = z.kind {
                g.spr(spr::CRACK, c.x, c.y, r * 2.1, r * 2.1, 0.4, alpha(WHITE, 0.9 * fade));
            }
        }

        // ground that would take the structure being placed
        if self.placing.is_some() {
            for p in &self.open {
                let c = v.at(*p);
                draw_circle(c.x, c.y, cell * 0.07, alpha(SHADE, 0.35));
                draw_circle(c.x, c.y, cell * 0.042, alpha(INK, 0.6));
            }
        }

        // reach of whatever the open menu is about, or of the chosen creature
        if let Menu::Tower(spot) = self.menu {
            if let Some(Some(tw)) = sim.towers.get(spot) {
                ring(v.at(tw.pos), sim.reach(spot) * cell, cell, INK);
            }
        }
        if let Some(un) = sim.units.get(self.sel) {
            if !un.guest && self.menu == Menu::None && self.placing.is_none() {
                let c = v.at(un.post);
                let d = un.who.def();
                let r = if d.perched { un.st.range } else { LEASH };
                ring(c, r * cell, cell, alpha(cr_color(un.who), 0.5));
            }
        }
        for &(p, life) in &self.marks {
            let c = v.at(p);
            let a = (life / 0.3).min(1.0);
            g.spr(spr::RING, c.x, c.y, cell * 0.7, cell * 0.42, 0.0, alpha(INK, 0.8 * a));
        }

        // everything standing on the ground, back to front
        let mut items: Vec<(f32, Item)> = Vec::with_capacity(64 + sim.foes.len());
        for (i, p) in self.sc.props.iter().enumerate() {
            items.push((p.pos.y, Item::Prop(i)));
        }
        for (i, tw) in sim.towers.iter().enumerate() {
            if let Some(tw) = tw {
                // things lying flat go under whatever walks past them
                let flat = tower_shape(tw.kind).1;
                items.push((if flat { tw.pos.y - 0.6 } else { tw.pos.y + 0.3 }, Item::Tower(i)));
            }
        }
        items.push((sim.map.home.y + 0.35, Item::Home));
        for (i, b) in sim.blocks.iter().enumerate() {
            items.push((b.pos.y, Item::Block(i)));
        }
        for i in 0..sim.units.len() {
            items.push((self.unit_pos(i, k).0.y + 0.05, Item::Unit(i)));
        }
        for (i, e) in sim.foes.iter().enumerate() {
            if !e.foe.def().flying {
                // seen from above, an invader lies on the ground: under anything standing
                let lie = if foe_pose(e.foe) == Pose::Side { 0.0 } else { -0.35 };
                items.push((e.pos.y + lie, Item::Foe(i)));
            }
        }
        for (i, r) in self.remains.iter().enumerate() {
            items.push((r.pos.y - 0.5, Item::Remain(i)));
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (_, item) in &items {
            match *item {
                Item::Prop(i) => {
                    let p = &self.sc.props[i];
                    let c = v.at(p.pos);
                    let kk = p.size * cell / p.spr.w.max(p.spr.h);
                    let (w, h) = (p.spr.w * kk, p.spr.h * kk);
                    g.spr(spr::SHADOW, c.x, c.y, w * 0.9, w * 0.3, 0.0, alpha(WHITE, 0.75));
                    g.spr_ex(p.spr, c.x, c.y - h * 0.46, w, h, 0.0, WHITE, p.flip);
                }
                Item::Tower(i) => self.draw_tower(app, v, i),
                Item::Home => {
                    let c = v.at(sim.map.home);
                    let s = self.pal.home;
                    let kk = cell * 2.0 / s.w.max(s.h);
                    let (w, h) = (s.w * kk, s.h * kk);
                    let jolt = self.home_hit * (t * 50.0).sin() * cell * 0.05;
                    g.spr(spr::SHADOW, c.x, c.y + h * 0.3, w * 0.95, w * 0.3, 0.0, alpha(WHITE, 0.7));
                    g.spr(s, c.x + jolt, c.y - h * 0.08, w, h, 0.0, mix(WHITE, BLOOD, self.home_hit * 0.5));
                }
                Item::Block(i) => {
                    let b = &sim.blocks[i];
                    let c = v.at(b.pos);
                    let fade = (b.t / 0.6).min(1.0);
                    let (s, size, flat) = match b.kind {
                        BlockKind::Dam => (spr::DAM, 1.3, true),
                        BlockKind::Decoy => (spr::DECOY, 1.1, false),
                        BlockKind::Cousin => (spr::COUSIN, 0.85, false),
                    };
                    let kk = size * cell / s.w.max(s.h);
                    let (w, h) = (s.w * kk, s.h * kk);
                    if flat {
                        // across the trail, whichever way it runs
                        let (route, along, _) = sim.map.nearest_trail(b.pos);
                        let dir = sim.map.routes[route].at(along).1;
                        g.spr(s, c.x, c.y, w, h, dir.y.atan2(dir.x) + PI / 2.0, alpha(WHITE, fade));
                    } else {
                        g.spr(spr::SHADOW, c.x, c.y + h * 0.3, w * 0.8, w * 0.26, 0.0, alpha(WHITE, 0.6 * fade));
                        g.spr(s, c.x, c.y - h * 0.2, w, h, 0.0, alpha(WHITE, fade));
                    }
                }
                Item::Unit(i) => self.draw_unit(app, v, i, k),
                Item::Foe(i) => self.draw_foe(app, v, i, k),
                Item::Remain(i) => self.draw_remain(app, v, i),
            }
        }
        // whirlwinds, then fliers above everything
        for z in &sim.zones {
            if let ZoneKind::Twister(_) = z.kind {
                let c = v.at(z.pos);
                let s = cell * 1.6 * (z.t / 0.4).min(1.0);
                g.spr(spr::TWISTER, c.x + (t * 9.0).sin() * cell * 0.08, c.y - s * 0.35, s * (0.9 + 0.1 * (t * 13.0).sin()), s, (t * 6.0).sin() * 0.1, alpha(INK, 0.6));
            }
        }
        for i in 0..sim.foes.len() {
            if sim.foes[i].foe.def().flying {
                self.draw_foe(app, v, i, k);
            }
        }
        self.draw_shots(app, v, k);
    }

    /// A structure's picture: standing on `c`, or lying on it.
    #[allow(clippy::too_many_arguments)]
    fn tower_pic(g: &Gfx, kind: Tw, c: Vec2, cell: f32, scale: f32, squash: f32, tint: Color, shadow: f32) {
        let (size, flat) = tower_shape(kind);
        let s = tower_spr(kind);
        let kk = size * cell * scale / s.w.max(s.h);
        let (w, h) = (s.w * kk * (1.0 + squash), s.h * kk * (1.0 - squash));
        if flat {
            g.spr(s, c.x, c.y, w, h, 0.0, tint);
        } else {
            // its foot a little below the middle of its ground
            let foot = c.y + cell * 0.36;
            g.spr(spr::SHADOW, c.x, foot - h * 0.04, w * 0.85, w * 0.26, 0.0, alpha(WHITE, shadow));
            g.spr(s, c.x, foot - h * 0.5, w, h, 0.0, tint);
        }
    }

    fn draw_tower(&self, app: &App, v: &View, i: usize) {
        let g = &app.g;
        let cell = v.cell;
        let t = self.t;
        let Some(Some(tw)) = self.sim.towers.get(i) else { return };
        let c = v.at(tw.pos);
        let chosen = matches!(self.menu, Menu::Tower(s) if s == i);
        // a flinch when it lets fly
        let kick = (1.0 - tw.fired / 0.16).clamp(0.0, 1.0);
        let scale = (1.0 + 0.06 * (tw.level as f32 - 1.0)) * if chosen { 1.05 } else { 1.0 };
        let jammed = tw.jam > 0.0;
        let tint = if jammed {
            rgb(0x7c8478)
        } else if tw.haste > 0.0 {
            mix(WHITE, GOLD, 0.2 + 0.2 * (t * 12.0).sin().abs())
        } else {
            WHITE
        };
        Self::tower_pic(g, tw.kind, c, cell, scale, 0.035 * kick, tint, 0.75);
        if jammed {
            g.spr_fit(spr::P_SMOKE, c.x + (t * 3.0).sin() * cell * 0.1, c.y - cell * 0.5, cell * 0.55, 0.0, alpha(rgb(0xb8c890), 0.6));
        }
        // its level, as pips under it
        for n in 0..tw.level {
            let x = c.x + (n as f32 - (tw.level as f32 - 1.0) / 2.0) * cell * 0.15;
            draw_circle(x, c.y + cell * 0.5, cell * 0.052, alpha(SHADE, 0.8));
            draw_circle(x, c.y + cell * 0.5, cell * 0.034, GOLD);
        }
    }

    fn draw_unit(&self, app: &App, v: &View, i: usize, k: f32) {
        let g = &app.g;
        let cell = v.cell;
        let t = self.t;
        let un = &self.sim.units[i];
        let d = un.who.def();
        let (p, hop) = self.unit_pos(i, k);
        let c = v.at(p);
        let chosen = i == self.sel && !un.guest;
        let sprite = if un.shell > 0.0 { spr::TORTOISE_SHELL } else { cr_spr(un.who) };
        let long = cr_size(un.who) * cell * if un.guest { 0.88 } else { 1.0 };
        let kk = long / sprite.w.max(sprite.h);
        let (w, h) = (sprite.w * kk, sprite.h * kk);
        let foot = c.y + cell * 0.2;
        if chosen {
            g.spr(spr::RING, c.x, foot - cell * 0.06, cell * 1.3, cell * 0.72, 0.0, alpha(cr_color(un.who), 0.95));
        }
        if !un.alive() {
            // down, and getting its breath back
            g.spr_ex(sprite, c.x, foot - h * 0.3, w, h, if un.face < 0.0 { -1.2 } else { 1.2 }, alpha(rgb(0x4a4844), 0.85), un.face < 0.0);
            cooldown_ring(c.x, foot - cell * 0.2, cell * 0.44, cell * 0.06, un.ko / un.st.revive.max(0.1), alpha(INK, 0.75));
            return;
        }
        let moving = un.pos.dist(un.prev) > 1e-4;
        let flying = un.who == Cr::Owl;
        let lift = if flying { cell * 0.55 } else if d.perched { cell * 0.3 } else { 0.0 } + hop * cell;
        let bob = if flying {
            (t * 2.6 + i as f32).sin() * cell * 0.07
        } else if moving {
            (t * 13.0).sin().abs() * cell * 0.045
        } else {
            0.0
        };
        // a lunge on every attack
        let lunge = (1.0 - un.swung / 0.16).clamp(0.0, 1.0) * cell * 0.16 * un.face;
        let breathe = 1.0 + 0.012 * (t * 2.2 + i as f32 * 1.3).sin();
        if flying {
            g.spr(spr::SHADOW, c.x + cell * 0.2, foot + cell * 0.25, w * 0.55, w * 0.2, 0.0, alpha(WHITE, 0.45));
        } else {
            g.spr(spr::SHADOW, c.x, foot - h * 0.02, w * (0.85 - lift / cell * 0.3), w * 0.26, 0.0, alpha(WHITE, 0.8));
        }
        if d.perched && !flying {
            // something to keep watch from
            let r = self.pal.rock;
            let rk = cell * 0.8 / r.w.max(r.h);
            g.spr(r, c.x, foot - r.h * rk * 0.3, r.w * rk, r.h * rk, 0.0, WHITE);
        }
        let y = foot - h * 0.5 - lift - bob;
        if un.stand > 0.0 || un.shell > 0.0 {
            let col = if un.stand > 0.0 { GOLD } else { MOSS };
            g.lights(|| g.spr(spr::GLOW, c.x, y, long * 1.6, long * 1.6, 0.0, alpha(col, 0.3 + 0.1 * (t * 10.0).sin())));
        }
        let rock = if moving && !flying { (t * 13.0).sin() * 0.03 } else { 0.0 };
        g.spr_ex(sprite, c.x + lunge, y, w, h * breathe, rock, WHITE, un.face < 0.0);
        if un.frenzy > 0 {
            for n in 0..un.frenzy {
                g.spr_fit(spr::SK_FLAME, c.x + (n as f32 - (un.frenzy as f32 - 1.0) / 2.0) * cell * 0.17, y - h * 0.62, cell * 0.2, 0.0, rgb(0xd87a3a));
            }
        }
    }

    /// Where and how an invader's picture goes: (centre, width, height,
    /// rotation, mirrored).
    fn foe_pic(foe: Foe, c: Vec2, dir: V2, cell: f32, t: f32, phase: f32, still: bool, dist: f32) -> (Vec2, f32, f32, f32, bool) {
        let d = foe.def();
        let s = foe_spr(foe);
        let kk = d.size * cell * 1.5 / s.w.max(s.h);
        let (w, h) = (s.w * kk, s.h * kk);
        let pace = d.speed * 9.0;
        match foe_pose(foe) {
            Pose::Above => {
                // a scuttle for what has legs, a shudder for what has wheels
                let wig = if still {
                    0.0
                } else if foe.rolls() || foe_end(foe) == End::Stalls {
                    (t * 31.0 + phase).sin() * 0.012
                } else {
                    (t * pace * 1.6 + phase).sin() * 0.09
                };
                (c, w, h, dir.y.atan2(dir.x) + wig, false)
            }
            Pose::Side => {
                let bob = if still { 0.0 } else { (t * pace + phase).sin().abs() * cell * 0.05 };
                let rock = if still { 0.0 } else { (t * pace + phase).sin() * 0.04 };
                (vec2(c.x, c.y + cell * 0.22 - h * 0.5 - bob), w, h, rock, dir.x < -0.25)
            }
            Pose::Round => {
                if foe == Foe::Tumbleweed {
                    (c, w, h, dist * 2.4, false)
                } else {
                    // it creeps: swelling and turning a little as it comes
                    let swell = 1.0 + if still { 0.0 } else { 0.05 * (t * 2.4 + phase).sin() };
                    (c, w * swell, h / swell, (t * 0.7 + phase).sin() * 0.2, false)
                }
            }
        }
    }

    fn draw_foe(&self, app: &App, v: &View, i: usize, k: f32) {
        let g = &app.g;
        let cell = v.cell;
        let t = self.t;
        let e = &self.sim.foes[i];
        let d = e.foe.def();
        let phase = e.id as f32 * 1.7;
        let fleeing = e.fear > 0.0;
        let dir = if fleeing { e.dir * -1.0 } else { e.dir };
        let ground = v.at(foe_at(e, k));
        let still = e.stun > 0.0;
        let (mut c, w, h, mut rot, flip) = Self::foe_pic(e.foe, ground, dir, cell, t, phase, still, e.dist);
        if still {
            rot += (t * 10.0 + phase).sin() * 0.06;
        }
        let sprite = foe_spr(e.foe);
        let mut tint = WHITE;
        let slowed = e.slowed();
        if slowed > 0.05 {
            tint = mix(tint, FROST, (slowed * 1.2).min(0.55));
        }
        if e.poison_t > 0.0 {
            tint = mix(tint, rgb(0x9ab860), 0.3 + 0.12 * (t * 8.0).sin());
        }
        if d.flying {
            // its shadow on the ground, and itself well above it
            let lift = cell * (0.5 + d.size * 0.25);
            g.spr_ex(sprite, ground.x + lift * 0.45, ground.y + lift * 0.8, w * 0.92, h * 0.92, rot, alpha(BLACK, 0.3), flip);
            c.y -= lift * 0.2 + (t * 4.0 + phase).sin() * cell * 0.04;
        } else if foe_pose(e.foe) == Pose::Side {
            g.spr(spr::SHADOW, ground.x, ground.y + cell * 0.2, w * 0.9, w * 0.3, 0.0, alpha(WHITE, 0.75));
        } else {
            // lying on the ground: the same shape, just off to one side, as its shadow
            g.spr_ex(sprite, c.x + cell * 0.05, c.y + cell * 0.07, w, h, rot, alpha(BLACK, 0.38), flip);
        }
        g.spr_ex(sprite, c.x, c.y, w, h, rot, tint, flip);
        if e.hurt_t < 0.08 {
            g.lights(|| g.spr_ex(sprite, c.x, c.y, w, h, rot, alpha(WHITE, 0.45), flip));
        }
        let top = c.y - w.max(h) * 0.5;
        if e.stun > 0.0 {
            g.spr_fit(spr::FX_STUN, c.x, top, cell * 0.5, (t * 6.0).sin() * 0.3, INK);
        }
        if e.mark_t > 0.0 {
            g.spr_fit(spr::SK_EYE, c.x, top - cell * 0.05, cell * 0.3, 0.0, BLOOD);
        }
    }

    fn draw_remain(&self, app: &App, v: &View, i: usize) {
        let g = &app.g;
        let cell = v.cell;
        let r = &self.remains[i];
        let d = r.foe.def();
        let sprite = foe_spr(r.foe);
        let life = if d.boss { 2.6 } else { 1.3 };
        let phase = i as f32;
        match foe_end(r.foe) {
            End::Flees => {
                // back the way it came, fast
                let pos = r.pos - r.dir * (r.age * 3.4);
                let (c, w, h, rot, flip) = Self::foe_pic(r.foe, v.at(pos), r.dir * -1.0, cell, self.t * 2.2, phase, false, 0.0);
                let a = (1.0 - (r.age - 0.35) / 0.6).clamp(0.0, 1.0);
                g.spr(spr::SHADOW, c.x, v.at(pos).y + cell * 0.2, w * 0.9, w * 0.3, 0.0, alpha(WHITE, 0.7 * a));
                g.spr_ex(sprite, c.x, c.y, w, h, rot, alpha(WHITE, a), flip);
            }
            End::Stalls => {
                let (c, w, h, rot, flip) = Self::foe_pic(r.foe, v.at(r.pos), r.dir, cell, 0.0, phase, true, 0.0);
                let a = (1.0 - (r.age - life * 0.55) / (life * 0.45)).clamp(0.0, 1.0);
                g.spr_ex(sprite, c.x, c.y, w, h, rot, alpha(mix(WHITE, rgb(0x33302c), (r.age / 0.4).min(0.75)), a), flip);
                let puff = 0.5 + 0.5 * (r.age * 5.0 + phase).sin();
                g.spr_fit(spr::P_SMOKE, c.x + (r.age * 2.0).sin() * cell * 0.1, c.y - cell * (0.2 + r.age * 0.35), cell * (0.5 + 0.25 * puff) * d.size.max(0.7), r.age, alpha(rgb(0x3a3a38), 0.55 * a));
            }
            End::Dies => {
                let (c, w, h, rot, flip) = Self::foe_pic(r.foe, v.at(r.pos), r.dir, cell, 0.0, phase, true, 0.0);
                let a = (1.0 - r.age / life).clamp(0.0, 1.0);
                let shrink = 1.0 - 0.12 * (r.age / 0.3).min(1.0);
                g.spr_ex(sprite, c.x, c.y, w * shrink, h * shrink, rot + 0.5 * (r.age / 0.3).min(1.0), alpha(mix(WHITE, rgb(0x2c2622), (r.age / 0.25).min(0.7)), a * a), flip);
            }
        }
    }

    fn draw_shots(&self, app: &App, v: &View, k: f32) {
        let g = &app.g;
        let cell = v.cell;
        let t = self.t;
        for s in &self.sim.shots {
            if s.t < 0.0 {
                continue;
            }
            let Some((sprite, long, tumbles)) = shot_spr(s.look) else { continue };
            let p = s.prev.lerp(s.pos, k);
            let c = v.at(p);
            let kk = long * cell / sprite.w.max(sprite.h);
            let (w, h) = (sprite.w * kk, sprite.h * kk);
            if s.lob {
                let f = (s.t / s.dur).clamp(0.0, 1.0);
                let height = (f * PI).sin() * (0.9 + s.from.dist(s.to) * 0.2) * cell;
                g.spr(spr::SHADOW, c.x, c.y, cell * 0.3, cell * 0.11, 0.0, alpha(WHITE, 0.55));
                let grow = 1.0 + 0.3 * (f * PI).sin();
                g.spr(sprite, c.x, c.y - height - cell * 0.1, w * grow, h * grow, if tumbles { t * 7.0 } else { 0.0 }, WHITE);
            } else {
                let dir = s.to - s.from;
                let wobble = if s.look == Look::Bee { (t * 40.0).sin() * cell * 0.04 } else { 0.0 };
                let tint = if s.look == Look::Fang { rgb(0xb8d070) } else { WHITE };
                // birds and bats beat their wings on the way
                let flap = if matches!(s.look, Look::Shrike | Look::Kingfisher | Look::Bat) { 1.0 - 0.3 * (t * 34.0).sin().abs() } else { 1.0 };
                g.spr(sprite, c.x, c.y - cell * 0.25 + wobble, w, h * flap, if tumbles { t * 7.0 } else { dir.y.atan2(dir.x) }, tint);
            }
        }
        for b in &self.beams {
            let f = (b.age / 0.24).min(1.0);
            let a = 1.0 - f;
            let lift = vec2(0.0, cell * 0.25);
            let (from, to) = (b.a - lift, b.b - lift * 0.5);
            let dir = to - from;
            match b.look {
                Look::Tongue => {
                    // out and back
                    let reach = (f * PI).sin();
                    let tip = from + dir * reach;
                    draw_line(from.x, from.y, tip.x, tip.y, cell * 0.07, rgb(0xb86a78));
                    draw_circle(tip.x, tip.y, cell * 0.07, rgb(0xc88090));
                }
                Look::Zap => {
                    // a crooked line that jumps about
                    let n = 5;
                    let mut prev = from;
                    let side = vec2(-dir.y, dir.x).normalize_or_zero();
                    for j in 1..=n {
                        let ff = j as f32 / n as f32;
                        let jit = if j == n { 0.0 } else { ((b.age * 90.0).floor() + j as f32 * 7.3 + from.x).sin() * cell * 0.16 };
                        let pt = from + dir * ff + side * jit;
                        draw_line(prev.x, prev.y, pt.x, pt.y, cell * 0.09, alpha(rgb(0x8ac8e0), 0.45 * a));
                        draw_line(prev.x, prev.y, pt.x, pt.y, cell * 0.035, alpha(rgb(0xf4fbff), a));
                        prev = pt;
                    }
                    g.lights(|| g.spr(spr::GLOW, to.x, to.y, cell * 0.9, cell * 0.9, 0.0, alpha(rgb(0xcfe8f0), 0.5 * a)));
                }
                Look::Dive | Look::Peck => {
                    // the bird itself, out to the invader and gone
                    let s = if b.look == Look::Dive { spr::SHOT_EAGLE } else { spr::SHOT_WOODPECKER };
                    let long = if b.look == Look::Dive { 1.25 } else { 0.62 } * cell;
                    let kk = long / s.w.max(s.h);
                    let at = from + dir * ease_out(f);
                    g.spr(spr::STREAK, (from.x + at.x) / 2.0, (from.y + at.y) / 2.0, (at - from).length(), cell * 0.1, dir.y.atan2(dir.x), alpha(INK, 0.35 * a));
                    g.spr(s, at.x, at.y, s.w * kk, s.h * kk * (1.0 - 0.25 * (b.age * 40.0).sin().abs()), dir.y.atan2(dir.x), alpha(WHITE, (a * 3.0).min(1.0)));
                }
                _ => {
                    let mid = (from + to) / 2.0;
                    g.spr(spr::STREAK, mid.x, mid.y, dir.length(), cell * 0.14, dir.y.atan2(dir.x), alpha(look_color(b.look), 0.8 * a));
                }
            }
        }
    }

    /// Glows that show once the light goes: the home, things firing.
    fn draw_lights(&self, app: &App, v: &View, _k: f32) {
        let g = &app.g;
        let cell = v.cell;
        let glow = mood_glow(self.sim.def.mood);
        let t = self.t;
        g.lights(|| {
            let c = v.at(self.sim.map.home);
            g.spr(spr::GLOW, c.x, c.y - cell * 0.1, cell * 3.2, cell * 3.2, 0.0, alpha(rgb(0xe0a860), 0.26 * glow * (0.9 + 0.1 * (t * 3.0).sin())));
            for tw in self.sim.towers.iter().flatten() {
                let c = v.at(tw.pos);
                let flare = (1.0 - tw.fired / 0.25).clamp(0.0, 1.0);
                g.spr(spr::GLOW, c.x, c.y - cell * 0.2, cell * 1.7, cell * 1.7, 0.0, alpha(rgb(0xe8dcc0), (0.07 + 0.22 * flare) * glow));
            }
            for (i, un) in self.sim.units.iter().enumerate() {
                if un.alive() {
                    let c = v.at(un.pos);
                    g.spr(spr::GLOW, c.x, c.y - cell * 0.2, cell * 1.6, cell * 1.6, 0.0, alpha(cr_color(un.who), 0.14 * glow * (1.0 + 0.2 * (t * 2.0 + i as f32).sin())));
                }
            }
        });
    }

    /// Health bars, drawn over everything on the field.
    fn draw_bars(&self, v: &View, k: f32) {
        let cell = v.cell;
        for e in &self.sim.foes {
            let d = e.foe.def();
            // a crowd of small things each with a bar is just noise: theirs
            // only show once they are properly hurt
            let frac = e.hp / e.max_hp;
            if !d.boss && (frac > 0.99 || (d.size < 0.56 && frac > 0.7)) {
                continue;
            }
            let c = v.at(foe_at(e, k));
            let size = d.size * cell * 1.5;
            let lift = if d.flying { cell * (0.5 + d.size * 0.25) * 0.2 } else { 0.0 };
            let w = if d.boss { cell * 1.5 } else { size * 0.6 };
            let h = if d.boss { cell * 0.09 } else if d.size < 0.56 { cell * 0.04 } else { cell * 0.055 };
            let up = if foe_pose(e.foe) == Pose::Side { size * 0.86 } else { size * 0.6 };
            tiny_bar(c.x, c.y - up - lift, w, h, frac, if d.boss { BLOOD } else { rgb(0xc8703a) });
        }
        for (i, un) in self.sim.units.iter().enumerate() {
            if !un.alive() || un.who.def().perched {
                continue;
            }
            if un.hp < un.st.hp - 0.5 || un.holding > 0 {
                let c = v.at(self.unit_pos(i, k).0);
                tiny_bar(c.x, c.y - cell * 0.95, cell * 0.7, cell * 0.06, un.hp / un.st.hp, MOSS);
            }
        }
        for b in &self.sim.blocks {
            if b.kind == BlockKind::Dam {
                let c = v.at(b.pos);
                tiny_bar(c.x, c.y - cell * 0.7, cell * 0.8, cell * 0.055, (b.hp / b.max_hp).min(b.t / 8.0 + 0.2), rgb(0xb89060));
            }
        }
    }

    /// The structure being placed, floating where it would stand.
    fn draw_ghost(&self, app: &App, v: &View) {
        let (Some(kind), Some(at)) = (self.placing, self.ghost) else { return };
        let g = &app.g;
        let cell = v.cell;
        let c = v.at(at);
        let fits = self.sim.can_build(at);
        let ok = fits.is_ok() && self.sim.seeds >= self.sim.price(kind, 1);
        let col = if ok { INK } else { BLOOD };
        ring(c, kind.def().range * cell, cell, col);
        draw_circle_lines(c.x, c.y, FOOTPRINT * 0.5 * cell, cell * 0.03, alpha(col, 0.9));
        Self::tower_pic(g, kind, c, cell, 1.0, 0.0, alpha(if ok { WHITE } else { rgb(0xd08070) }, 0.8), 0.4);
        if let Err(b) = fits {
            let u = app.u;
            let text = refusal(b);
            let w = g.text_w(text, 12.0 * u) + 16.0 * u;
            let r = Rect::new((c.x - w / 2.0).clamp(app.x0 + 4.0 * u, app.x0 + app.pw - w - 4.0 * u), c.y - cell * 1.25, w, 20.0 * u);
            chip(g, r, 1.0);
            g.label(text, r.x + r.w / 2.0, r.y + r.h / 2.0, 12.0 * u, 0.5, BLOOD, None, 0.0);
        }
    }

    // ---- the top of the screen ----------------------------------------------------

    fn hud(&mut self, app: &mut App, _v: &View) -> bool {
        let u = app.u;
        let y = app.top + 5.0 * u;
        let h = 36.0 * u;
        let sim = &self.sim;
        let g = &app.g;
        let mut used = false;
        g.fade(Rect::new(app.x0, 0.0, app.pw, app.top + HUD_H * u + 14.0 * u), SHADE, 0.8, 0.0);

        // what the home can still take, and what there is to spend
        let left = Rect::new(app.x0 + 6.0 * u, y, 158.0 * u, h);
        chip(g, left, 1.0);
        let hp = 1.0 + 0.3 * self.heart_pop;
        let low = sim.hearts * 3 <= sim.max_hearts;
        g.spr_fit(spr::IC_HEART, left.x + 18.0 * u, left.y + h / 2.0, 17.0 * u * hp, 0.0, BLOOD);
        g.label(&sim.hearts.to_string(), left.x + 32.0 * u, left.y + h / 2.0, 18.0 * u, 0.0, if low { BLOOD } else { INK }, None, 0.0);
        let sp = 1.0 + 0.25 * self.seed_pop;
        g.spr_fit(spr::IC_SEED, left.x + 84.0 * u, left.y + h / 2.0, 17.0 * u * sp, 0.0, GOLD);
        g.label(&sim.seeds.to_string(), left.x + 97.0 * u, left.y + h / 2.0, 18.0 * u, 0.0, GOLD, None, 0.0);

        // which wave
        let mid = Rect::new(app.x0 + 170.0 * u, y, 116.0 * u, h);
        chip(g, mid, 1.0);
        let label = if sim.phase == Phase::Ready { format!("{} WAVES", sim.waves.len()) } else { format!("WAVE {} / {}", sim.wave, sim.waves.len()) };
        g.head(&label, mid.x + mid.w / 2.0, mid.y + h / 2.0, 12.5 * u, 0.5, INK);

        // speed and pause
        let bs = 38.0 * u;
        let speed = Rect::new(app.x0 + app.pw - 2.0 * bs - 10.0 * u, y - 1.0 * u, bs, bs);
        let pause = Rect::new(app.x0 + app.pw - bs - 6.0 * u, y - 1.0 * u, bs, bs);
        app.mark("speed", speed);
        app.mark("pause", pause);
        if app.ui.button(&app.g, &mut app.au, 10, speed, if self.speed > 1 { EMBER } else { STEEL }, "", Some(spr::IC_SPEED), self.over.is_none()) {
            self.speed = if self.speed > 1 { 1 } else { 2 };
            used = true;
        }
        if app.ui.button(&app.g, &mut app.au, 11, pause, STEEL, "", Some(spr::IC_PAUSE), self.over.is_none() && !self.paused) {
            self.paused = true;
            used = true;
        }
        let ptr = &app.ui.ptr;
        used |= (ptr.down || ptr.released) && ptr.pos.y < app.top + HUD_H * u;
        used
    }

    // ---- the bottom of the screen ---------------------------------------------------

    /// Who is coming, what can be built, and the pack; or whichever card
    /// is open over them. Returns true if the pointer is busy down here.
    fn bottom(&mut self, app: &mut App, _v: &View) -> bool {
        let u = app.u;
        let top = app.h - app.bot - (NEXT_H + TRAY_H + PACK_H) * u;
        let area = Rect::new(app.x0, top, app.pw, app.h - top);
        app.g.fade(Rect::new(area.x, area.y - 14.0 * u, area.w, 14.0 * u), SHADE, 0.0, 0.6);
        app.g.vgrad(area, rgb(0x1b1e18), rgb(0x0e100c));
        app.g.rect(Rect::new(area.x, area.y, area.w, 1.5 * u), alpha(INK, 0.28));
        let ptr = &app.ui.ptr;
        let inside = area.contains(ptr.pos) && (ptr.down || ptr.released) && area.contains(ptr.press_pos);
        let busy = self.over.is_some() || self.paused;
        match self.menu {
            Menu::Tower(spot) => self.tower_menu(app, area, spot, busy),
            Menu::Foe(foe) => self.foe_card(app, area, foe),
            Menu::None => {
                self.next_strip(app, area, busy);
                self.tray(app, area, busy);
                if let Some(kind) = self.placing {
                    self.placing_card(app, area, kind);
                } else {
                    self.cards(app, area, busy);
                }
            }
        }
        inside
    }

    /// Who is coming next.
    fn next_strip(&mut self, app: &mut App, area: Rect, busy: bool) {
        let u = app.u;
        let y = area.y + 5.0 * u;
        let h = (NEXT_H - 6.0) * u;
        let sim = &self.sim;
        let g = &app.g;
        let mut tapped_foe = None;
        if let Some(wave) = sim.upcoming() {
            g.head("NEXT", area.x + 10.0 * u, y + h / 2.0, 10.0 * u, 0.0, DIM);
            let mut x = area.x + 52.0 * u;
            for (foe, count) in wave.tally().into_iter().take(6) {
                let r = Rect::new(x, y, 50.0 * u, h);
                chip(g, r, 1.0);
                g.spr_fit(foe_spr(foe), r.x + 14.0 * u, r.y + h / 2.0, h * 0.86, if foe_pose(foe) == Pose::Above { -PI / 2.0 } else { 0.0 }, WHITE);
                g.label(&count.to_string(), r.x + r.w - 6.0 * u, r.y + h / 2.0, 12.0 * u, 1.0, INK, None, 0.0);
                if app.ui.tapped(r) && !busy {
                    tapped_foe = Some(foe);
                }
                x += 53.0 * u;
            }
        } else if sim.playing() {
            g.label("That is the last of them. Hold.", area.x + 10.0 * u, y + h / 2.0, 12.5 * u, 0.0, DIM, None, 0.0);
        }
        if let Some(foe) = tapped_foe {
            self.menu = Menu::Foe(foe);
            self.placing = None;
            app.au.play(Sfx::Click, 0.6);
        }
    }

    /// What can be built here, and the button that brings the next wave.
    fn tray(&mut self, app: &mut App, area: Rect, busy: bool) {
        let u = app.u;
        let towers = BIOMES[self.sim.def.biome].towers;
        let n = towers.len() as f32;
        let gap = 4.0 * u;
        let y = area.y + NEXT_H * u;
        let h = (TRAY_H - 4.0) * u;
        let call_w = 92.0 * u;
        let w = (area.w - 12.0 * u - call_w - 6.0 * u - gap * (n - 1.0)) / n;
        for (i, &k) in towers.iter().enumerate() {
            let r = Rect::new(area.x + 6.0 * u + i as f32 * (w + gap), y, w, h);
            let cost = self.sim.price(k, 1);
            let ok = self.sim.seeds >= cost;
            let chosen = self.placing == Some(k);
            let ptr = &app.ui.ptr;
            if ptr.pressed && r.contains(ptr.pos) && !busy {
                if ok {
                    self.tray_press = Some((k, chosen));
                    self.placing = Some(k);
                    self.ghost = None;
                    app.au.play(Sfx::Click, 0.7);
                } else {
                    app.au.play(Sfx::Error, 0.6);
                    self.seed_pop = 1.0;
                }
            }
            let ptr = &app.ui.ptr;
            if ptr.released && r.contains(ptr.pos) && ptr.travel < 14.0 * u && self.tray_press == Some((k, true)) {
                // a second tap puts it back
                self.placing = None;
            }
            let chosen = self.placing == Some(k);
            let g = &app.g;
            g.nine(spr::BTN, r, 32.0, 8.0 * u, if chosen { rgb(0x6a6a52) } else if ok { rgb(0x3a3e34) } else { rgb(0x2a2c25) });
            if chosen {
                g.rect(Rect::new(r.x + 3.0 * u, r.y + r.h - 4.0 * u, r.w - 6.0 * u, 2.5 * u), GOLD);
            }
            let s = tower_spr(k);
            let fit = (h * 0.6).min(w * 0.8);
            let kk = fit / s.w.max(s.h);
            g.spr(s, r.x + w / 2.0, r.y + h * 0.37, s.w * kk, s.h * kk, 0.0, if ok { WHITE } else { rgb(0x8c8c8c) });
            price(g, r.x + w / 2.0, r.y + h * 0.83, 12.0 * u, cost, ok);
            app.mark(&format!("build{i}"), r);
        }

        let call = Rect::new(area.x + area.w - call_w - 6.0 * u, y, call_w, h);
        let sim = &self.sim;
        let (label, color, on) = match sim.phase {
            Phase::Ready => ("START".to_string(), GO, true),
            Phase::Battle if sim.can_call() => (format!("CALL +{}", sim.call_bonus()), EMBER, true),
            Phase::Battle if sim.wave < sim.waves.len() => ("COMING".to_string(), GREY, false),
            _ => (String::new(), GREY, false),
        };
        if !label.is_empty() {
            app.mark("call", call);
            if app.ui.button(&app.g, &mut app.au, 20, call, color, &label, None, on && !busy) {
                self.sim.call();
            }
            if on && self.sim.phase == Phase::Battle {
                // how long before they come by themselves
                bar(&app.g, Rect::new(call.x + 5.0 * u, call.y + call.h - 7.0 * u, call.w - 10.0 * u, 2.5 * u), self.sim.next_in / biome_core::sim::WAVE_GAP, GOLD, alpha(SHADE, 0.6));
            }
        }
    }

    /// While a structure is waiting for a place: what it is and does.
    fn placing_card(&mut self, app: &mut App, area: Rect, kind: Tw) {
        let u = app.u;
        let g = &app.g;
        let d = kind.def();
        let y = area.y + (NEXT_H + TRAY_H) * u + 2.0 * u;
        let x = area.x + 10.0 * u;
        g.head(&d.name.to_uppercase(), x, y + 9.0 * u, 12.5 * u, 0.0, INK);
        g.label(d.tag, x + g.text_w_in(Face::Head, &d.name.to_uppercase(), 12.5 * u) + 9.0 * u, y + 9.0 * u, 11.5 * u, 0.0, GOLD, None, 0.0);
        let text = match self.why {
            Some((why, _)) => format!("{why}."),
            None => d.blurb.to_string(),
        };
        let lines = g.wrap(&text, 12.0 * u, area.w - 20.0 * u);
        for (i, line) in lines.iter().take(2).enumerate() {
            g.write(Face::Text, line, x, y + 25.0 * u + i as f32 * 14.0 * u, 12.0 * u, 0.0, if self.why.is_some() { BLOOD } else { INK }, None, 0.0);
        }
        g.write(Face::Text, "Press on open ground and slide to place it. Tap it again to put it back.", x, y + 54.0 * u, 10.5 * u, 0.0, DIM, None, 0.0);
    }

    /// One card per pack member: tap the picture to choose who the next
    /// order goes to, tap the square button to use the ability.
    fn cards(&mut self, app: &mut App, area: Rect, busy: bool) {
        let u = app.u;
        let pack: Vec<usize> = (0..self.sim.units.len()).filter(|&i| !self.sim.units[i].guest).collect();
        let n = pack.len().max(1) as f32;
        let gap = 5.0 * u;
        let w = ((area.w - 12.0 * u - gap * (n - 1.0)) / n).min(200.0 * u);
        let total = w * n + gap * (n - 1.0);
        let y = area.y + (NEXT_H + TRAY_H) * u + 2.0 * u;
        let h = (PACK_H - 8.0) * u;
        for (slot, &i) in pack.iter().enumerate() {
            let x = area.x + (area.w - total) / 2.0 + slot as f32 * (w + gap);
            let r = Rect::new(x, y, w, h);
            let (who, alive, hp, cooling, ko, revive, perched, ready) = {
                let un = &self.sim.units[i];
                (un.who, un.alive(), un.hp / un.st.hp, un.cooling(), un.ko, un.st.revive, un.who.def().perched, un.cd <= 0.0 && un.alive())
            };
            let chosen = self.sel == i;
            let g = &app.g;
            let base = cr_color(who);
            g.nine(spr::BTN, r, 32.0, 8.0 * u, if chosen { rgb(0x3c4034) } else { rgb(0x262921) });
            if chosen {
                g.rect(Rect::new(r.x + 3.0 * u, r.y + r.h - 4.0 * u, r.w - 6.0 * u, 2.5 * u), base);
            }
            // the picture, with health as a ring round it
            let pic = h * 0.8;
            let px = r.x + 6.0 * u + pic / 2.0;
            let py = r.y + h * 0.48;
            let face = Rect::new(r.x, r.y, w - h * 0.9, h);
            portrait(g, cr_face(who), px, py, pic, alpha(SHADE, 0.9), if alive { WHITE } else { rgb(0x555555) });
            if alive && !perched {
                macroquad::prelude::draw_arc(px, py, 40, pic * 0.5, -90.0, 2.6 * u, 360.0 * hp.clamp(0.0, 1.0), if hp > 0.35 { MOSS } else { BLOOD });
            } else if alive {
                macroquad::prelude::draw_arc(px, py, 40, pic * 0.5, -90.0, 2.0 * u, 360.0, alpha(base, 0.8));
            } else {
                g.label(&format!("{:.0}", ko.ceil()), px, py, 20.0 * u, 0.5, INK, Some((SHADE, 1.6 * u)), 0.0);
                cooldown_ring(px, py, pic * 0.5, 2.6 * u, ko / revive.max(0.1), alpha(INK, 0.8));
            }
            // the name, where there is room for it
            let tx = px + pic / 2.0 + 7.0 * u;
            let room = w - (tx - r.x) - h * 0.86;
            if room > 34.0 * u {
                g.head(&who.def().name.to_uppercase(), tx, r.y + h * 0.36, (12.0 * u).min(room * 1.6 / who.def().name.len().max(4) as f32), 0.0, INK);
                g.label(if !alive { "down" } else if perched { "out of reach" } else { "on the ground" }, tx, r.y + h * 0.64, 10.0 * u, 0.0, DIM, None, 0.0);
            }
            if app.ui.tapped(face) && !busy {
                self.sel = i;
                app.au.play(Sfx::Click, 0.6);
            }
            app.mark(&format!("unit{slot}"), face);

            // the ability
            let bs = h * 0.78;
            let br = Rect::new(r.x + w - bs - 6.0 * u, r.y + (h - bs) / 2.0 - 1.0 * u, bs, bs);
            let shake = if self.nope > 0.0 && chosen { (self.t * 60.0).sin() * 3.0 * u * self.nope } else { 0.0 };
            let br = Rect::new(br.x + shake, br.y, br.w, br.h);
            app.mark(&format!("cast{slot}"), br);
            if app.ui.button(&app.g, &mut app.au, 30 + slot as u32, br, if ready { EMBER } else { rgb(0x34362f) }, "", Some(ability_icon(who)), alive && !busy) {
                self.sel = i;
                if !self.sim.cast(i) {
                    self.nope = 1.0;
                    app.au.play(Sfx::Error, 0.6);
                }
            }
            if alive && cooling > 0.0 {
                // filling from the bottom as it comes back
                let f = cooling.clamp(0.0, 1.0);
                app.g.rect(Rect::new(br.x + 2.0 * u, br.y + 2.0 * u, br.w - 4.0 * u, (br.h - 4.0 * u) * f), alpha(SHADE, 0.6));
            }
        }
    }

    fn close_button(&mut self, app: &mut App, area: Rect, busy: bool) -> bool {
        let u = app.u;
        let close = Rect::new(area.x + area.w - 38.0 * u, area.y + 6.0 * u, 32.0 * u, 30.0 * u);
        app.mark("close", close);
        app.ui.button(&app.g, &mut app.au, 49, close, GREY, "", Some(spr::IC_X), !busy)
    }

    fn tower_menu(&mut self, app: &mut App, area: Rect, spot: usize, busy: bool) {
        let u = app.u;
        let Some(tw) = self.sim.towers.get(spot).cloned().flatten() else {
            self.menu = Menu::None;
            return;
        };
        let d = tw.kind.def();
        let lv = tw.level as usize;
        let g = &app.g;
        let x = area.x + 10.0 * u;
        let y = area.y + 10.0 * u;
        let s = tower_spr(tw.kind);
        let kk = 62.0 * u / s.w.max(s.h);
        g.spr(s, x + 34.0 * u, y + 36.0 * u, s.w * kk, s.h * kk, 0.0, WHITE);
        stars(g, x + 34.0 * u, y + 84.0 * u, 13.0 * u, tw.level);
        let tx = x + 78.0 * u;
        g.head(&d.name.to_uppercase(), tx, y + 8.0 * u, 14.0 * u, 0.0, INK);
        // what it does, in numbers
        let stats = match d.mode {
            Mode::Field { slow } => format!("Slows by {:.0}%   Reach {:.1}", (slow + LV_SLOW * (lv as f32 - 1.0)) * 100.0, self.sim.reach(spot)),
            _ => format!("Hits {:.0}   Reach {:.1}   {:.1} a second", d.dmg * LV_DMG[lv - 1], d.range * LV_RANGE[lv - 1], d.rate * LV_RATE[lv - 1]),
        };
        g.label(&stats, tx, y + 27.0 * u, 12.0 * u, 0.0, GOLD, None, 0.0);
        let perk = if lv == 3 { d.perk_text.to_string() } else { format!("At level III: {}", d.perk_text) };
        let lines = g.wrap(&perk, 11.5 * u, area.w - 78.0 * u - 54.0 * u);
        for (i, line) in lines.iter().take(2).enumerate() {
            g.write(Face::Text, line, tx, y + 44.0 * u + i as f32 * 13.5 * u, 11.5 * u, 0.0, if lv == 3 { INK } else { DIM }, None, 0.0);
        }
        let by = y + 78.0 * u;
        let bh = 42.0 * u;
        let up = Rect::new(tx, by, 150.0 * u, bh);
        let sell = Rect::new(tx + 156.0 * u, by, 110.0 * u, bh);
        app.mark("upgrade", up);
        app.mark("sell", sell);
        match self.sim.upgrade_price(spot) {
            Some(cost) => {
                let ok = self.sim.seeds >= cost;
                if app.ui.button(&app.g, &mut app.au, 50, up, if ok { GO } else { GREY }, "", None, !busy) {
                    if ok {
                        self.sim.upgrade(spot);
                    } else {
                        app.au.play(Sfx::Error, 0.6);
                        self.seed_pop = 1.0;
                    }
                }
                let g = &app.g;
                g.head("IMPROVE", up.x + 12.0 * u, up.y + bh * 0.5, 12.0 * u, 0.0, INK);
                price(g, up.x + up.w - 34.0 * u, up.y + bh * 0.5, 14.0 * u, cost, ok);
            }
            None => {
                app.g.nine(spr::BTN, up, 32.0, 8.0 * u, rgb(0x2c2e27));
                app.g.head("FULLY GROWN", up.x + up.w / 2.0, up.y + bh * 0.5, 12.0 * u, 0.5, GOLD);
            }
        }
        let back = self.sim.sell_value(spot);
        if app.ui.button(&app.g, &mut app.au, 51, sell, RUST, "", None, !busy) {
            self.sim.sell(spot);
            self.menu = Menu::None;
        }
        let g = &app.g;
        g.head("SELL", sell.x + 12.0 * u, sell.y + bh * 0.5, 12.0 * u, 0.0, INK);
        price(g, sell.x + sell.w - 30.0 * u, sell.y + bh * 0.5, 13.0 * u, back, true);
        if self.close_button(app, area, busy) {
            self.menu = Menu::None;
        }
    }

    fn foe_card(&mut self, app: &mut App, area: Rect, foe: Foe) {
        let u = app.u;
        let d = foe.def();
        let g = &app.g;
        let y = area.y + 16.0 * u;
        let s = foe_spr(foe);
        let kk = 84.0 * u / s.w.max(s.h);
        g.spr(s, area.x + 52.0 * u, y + 52.0 * u, s.w * kk, s.h * kk, if foe_pose(foe) == Pose::Above { -PI / 2.0 } else { 0.0 }, WHITE);
        let tx = area.x + 108.0 * u;
        g.head(&d.name.to_uppercase(), tx, y + 8.0 * u, 16.0 * u, 0.0, INK);
        let mut tags = vec![d.class.name().to_string()];
        if d.flying {
            tags.push("flies".into());
        }
        if d.armor > 0.5 {
            tags.push(format!("armor {:.0}", d.armor));
        }
        if foe.rolls() {
            tags.push("cannot be held".into());
        }
        g.label(&tags.join("   "), tx, y + 28.0 * u, 12.0 * u, 0.0, GOLD, None, 0.0);
        for (i, line) in g.wrap(d.blurb, 13.0 * u, area.w - 118.0 * u).iter().take(3).enumerate() {
            g.write(Face::Text, line, tx, y + 50.0 * u + i as f32 * 16.0 * u, 13.0 * u, 0.0, INK, None, 0.0);
        }
        g.label("tap to close", area.x + area.w - 10.0 * u, area.y + area.h - app.bot - 12.0 * u, 10.0 * u, 1.0, DIM, None, 0.0);
        if app.ui.ptr.released {
            self.menu = Menu::None;
            app.ui.eat();
        }
    }

    // ---- placing a structure ---------------------------------------------------------------

    /// While a structure is chosen, a press on the field is about where to
    /// put it. A quick tap builds right there. A longer press lifts the
    /// structure clear of the finger so the spot can be seen, slides with
    /// it, and builds where it is when the finger comes up. The finger may
    /// run down over the tray as it slides (or start there, dragging a
    /// structure out): what counts is where the structure is.
    fn place_input(&mut self, app: &mut App, v: &View) {
        let Some(kind) = self.placing else { return };
        let (top, bottom) = self.field_span(app);
        let ptr = &app.ui.ptr;
        if !(ptr.down || ptr.released) {
            self.ghost = None;
            return;
        }
        let began_on_field = ptr.press_pos.y > top && ptr.press_pos.y < bottom;
        // a press that began on the tray only counts once it has moved up off it
        let sliding = began_on_field || ptr.pos.y < ptr.press_pos.y - 12.0 * app.u;
        let lift = if ptr.is_touch() { GHOST_LIFT * ((self.hold - 0.12) / 0.16).clamp(0.0, 1.0) } else { 0.0 };
        let at = v.tile(ptr.pos) - v2(0.0, lift);
        let ghost_y = v.at(at).y;
        if !sliding || ghost_y < top || ghost_y > bottom {
            self.ghost = None;
            return;
        }
        if ptr.down {
            self.ghost = Some(at);
            return;
        }
        // the finger came up: build here, if here will do
        self.ghost = None;
        match self.sim.can_build(at) {
            Ok(()) => {
                if self.sim.build(kind, at).is_some() {
                    self.placing = None;
                    self.why = None;
                } else {
                    self.why = Some(("Not enough forage", 1.6));
                    self.seed_pop = 1.0;
                    app.au.play(Sfx::Error, 0.6);
                }
            }
            Err(b) => {
                self.why = Some((refusal(b), 1.8));
                app.au.play(Sfx::Error, 0.6);
                app.log(&format!("REFUSED {b:?}"));
            }
        }
    }

    // ---- taps on the field --------------------------------------------------------------

    fn field_taps(&mut self, app: &mut App, v: &View) {
        let ptr = &app.ui.ptr;
        if !ptr.released || ptr.travel > 14.0 * app.u {
            return;
        }
        let (top, bottom) = self.field_span(app);
        if ptr.pos.y < top || ptr.pos.y > bottom || ptr.press_pos.y < top || ptr.press_pos.y > bottom {
            return;
        }
        let at = v.tile(ptr.pos);
        // a structure? (they stand a little above the middle of their ground)
        if let Some(spot) = self.sim.tower_at(at + v2(0.0, 0.12), 0.6) {
            self.menu = Menu::Tower(spot);
            app.au.play(Sfx::Click, 0.7);
            return;
        }
        if self.menu != Menu::None {
            self.menu = Menu::None;
            return;
        }
        // one of the pack?
        for (i, un) in self.sim.units.iter().enumerate() {
            if !un.guest && i != self.sel && (un.pos.x - at.x).hypot(un.pos.y - 0.2 - at.y) < 0.5 {
                self.sel = i;
                app.au.play(Sfx::Click, 0.6);
                return;
            }
        }
        // otherwise: go there
        if self.sim.units.get(self.sel).is_some_and(|un| !un.guest) && self.sim.playing() {
            self.sim.order(self.sel, at);
            let post = self.sim.units[self.sel].post;
            self.marks.push((post, 0.9));
            self.moved = true;
            app.au.play(Sfx::Step, 0.7);
        }
    }

    // ---- words over the field -----------------------------------------------------------

    fn draw_banners(&self, app: &App, v: &View) {
        let u = app.u;
        let g = &app.g;
        let r = v.rect();
        let cx = app.x0 + app.pw / 2.0;
        if self.sim.phase == Phase::Ready && self.over.is_none() {
            // the stage's name, and a word of advice
            let a = (self.intro / 0.5).min(1.0);
            let def = self.sim.def;
            let y = r.y.max(app.top + HUD_H * u) + 8.0 * u;
            let plate = Rect::new(app.x0, y, app.pw, 46.0 * u);
            g.nine(spr::PLATE, plate, 8.0, 8.0 * u, alpha(WHITE, a));
            g.head(&BIOMES[def.biome].name.to_uppercase(), cx, y + 13.0 * u, 9.5 * u, 0.5, alpha(self.pal.accent, a));
            g.head(&def.name.to_uppercase(), cx, y + 30.0 * u, (19.0 * u).min(app.pw * 1.25 / def.name.len().max(8) as f32), 0.5, alpha(INK, a));
            if !self.teach && self.placing.is_none() {
                let w = app.pw - 40.0 * u;
                let hh = g.para_height(def.tip, 13.0 * u, w - 24.0 * u);
                let tip = Rect::new(cx - w / 2.0, y + 56.0 * u, w, hh + 16.0 * u);
                chip(g, tip, a);
                g.para(def.tip, cx, tip.y + 8.0 * u, 13.0 * u, w - 24.0 * u, alpha(INK, a));
            }
        }
        if let Some((text, life, whole)) = &self.banner {
            let a = (life / 0.4).min(1.0) * ((whole - life) / 0.25).clamp(0.0, 1.0);
            let size = (26.0 * u).min(app.pw * 1.3 / text.len().max(6) as f32);
            let y = r.y + r.h * 0.3;
            g.nine(spr::PLATE, Rect::new(app.x0, y - size * 1.1, app.pw, size * 2.2), 8.0, 8.0 * u, alpha(WHITE, a));
            g.head(text, cx, y, size, 0.5, alpha(INK, a));
        }
    }

    /// The first stage of a new journey explains itself, one thing at a time.
    fn draw_hints(&self, app: &App, v: &View) {
        let u = app.u;
        let g = &app.g;
        let sim = &self.sim;
        let built = sim.towers.iter().flatten().count();
        let hero = sim.units[0].who.def();
        let tray_y = app.h - app.bot - (TRAY_H + PACK_H) * u;
        let first = vec2(app.x0 + 40.0 * u, tray_y + 2.0 * u);
        let call = vec2(app.x0 + app.pw - 52.0 * u, tray_y + 2.0 * u);
        let cast = vec2(app.x0 + app.pw - 40.0 * u, app.h - app.bot - PACK_H * u + 6.0 * u);
        let (text, target): (String, Option<Vec2>) = if self.placing.is_some() && built == 0 {
            ("Now press on open ground beside the trail. Slide to choose the spot; let go to build.".to_string(), self.hints.first().map(|p| v.at(*p)))
        } else if built == 0 {
            ("Choose something to build. A shrike is a good start.".to_string(), Some(first))
        } else if self.placing.is_some() {
            return;
        } else if !self.moved && sim.phase == Phase::Ready {
            (format!("Tap the trail to send {} there. {}", hero.name, if hero.perched { "She strikes from above." } else { "Invaders stop to fight him." }), Some(v.at(sim.map.routes[0].at(9.0).0)))
        } else if sim.phase == Phase::Ready {
            ("Ready? Tap START.".to_string(), Some(call))
        } else if !self.cast_once && sim.units[0].cd <= 0.0 && sim.worth_casting(0) {
            (format!("Now. Tap the lit button: {}.", hero.ability_name), Some(cast))
        } else if sim.wave <= 2 && built < 3 && sim.seeds >= 50 {
            ("You have forage to spare. Build again, or tap a structure to improve it.".to_string(), None)
        } else {
            return;
        };
        let r = v.rect();
        let w = app.pw - 36.0 * u;
        let hh = g.para_height(&text, 14.0 * u, w - 26.0 * u);
        let y = r.y.max(app.top + HUD_H * u) + if sim.phase == Phase::Ready { 62.0 * u } else { 10.0 * u };
        let boxr = Rect::new(app.x0 + (app.pw - w) / 2.0, y, w, hh + 18.0 * u);
        panel(g, boxr, u, WHITE);
        g.para(&text, boxr.x + w / 2.0, boxr.y + 9.0 * u, 14.0 * u, w - 26.0 * u, INK);
        if let Some(at) = target {
            let bounce = (self.t * 4.0).sin().abs() * 8.0 * u;
            g.spr_fit(spr::IC_NEXT, at.x, at.y - 22.0 * u - bounce, 28.0 * u, PI / 2.0, GOLD);
        }
    }

    // ---- cards over everything ------------------------------------------------------------

    fn pause_card(&mut self, app: &mut App) -> Option<Nav> {
        let u = app.u;
        let cx = app.x0 + app.pw / 2.0;
        let card = Rect::new(cx - 150.0 * u, app.h * 0.5 - 170.0 * u, 300.0 * u, 336.0 * u);
        let outside = dim(app, 0.65, card);
        panel(&app.g, card, u, WHITE);
        app.g.head("PAUSED", cx, card.y + 34.0 * u, 24.0 * u, 0.5, INK);
        let p = app.save.p();
        app.g.text_c(&format!("{}  -  {}", self.sim.def.name, p.difficulty.name()), cx, card.y + 62.0 * u, 13.0 * u, DIM);
        let mut nav = None;
        let bw = 220.0 * u;
        let bh = 50.0 * u;
        let bx = cx - bw / 2.0;
        app.mark("resume", Rect::new(bx, card.y + 84.0 * u, bw, bh));
        if app.ui.button(&app.g, &mut app.au, 60, Rect::new(bx, card.y + 84.0 * u, bw, bh), GO, "RESUME", Some(spr::IC_PLAY), true) || outside {
            self.paused = false;
            app.ui.eat();
        }
        if app.ui.button(&app.g, &mut app.au, 61, Rect::new(bx, card.y + 142.0 * u, bw, bh), STEEL, "START AGAIN", Some(spr::IC_RETRY), true) {
            nav = Some(Nav::Play(self.level));
        }
        app.mark("tomap", Rect::new(bx, card.y + 200.0 * u, bw, bh));
        if app.ui.button(&app.g, &mut app.au, 62, Rect::new(bx, card.y + 200.0 * u, bw, bh), STEEL, "BACK TO THE MAP", Some(spr::IC_HOME), true) {
            nav = Some(Nav::Map);
        }
        sound_toggles(app, cx, card.y + 268.0 * u, 63);
        if nav.is_some() {
            app.au.set_music_gain(1.0);
        }
        nav
    }

    fn over_card(&mut self, app: &mut App) -> Option<Nav> {
        let u = app.u;
        let cx = app.x0 + app.pw / 2.0;
        let level = self.level;
        let o = self.over.as_mut().unwrap();
        o.t += app.dt;
        let t = o.t - 0.9;
        if t < 0.0 {
            return None;
        }
        let rise = ease_out((t / 0.5).min(1.0));
        let card = Rect::new(cx - 158.0 * u, app.h * 0.5 - 156.0 * u + (1.0 - rise) * 24.0 * u, 316.0 * u, if o.won { 318.0 } else { 304.0 } * u);
        app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(SHADE, 0.7 * (t / 0.4).min(1.0)));
        panel(&app.g, card, u, alpha(WHITE, rise));
        let g = &app.g;
        g.head(o.word, cx, card.y + 38.0 * u, (26.0 * u).min(290.0 * u * 1.3 / o.word.len().max(6) as f32), 0.5, alpha(if o.won { INK } else { BLOOD }, rise));
        g.rect(Rect::new(cx - 60.0 * u, card.y + 62.0 * u, 120.0 * u, 1.5 * u), alpha(INK, 0.3 * rise));
        let mut nav = None;
        if o.won {
            // the marks arrive one at a time
            for i in 0..3u8 {
                let at = 0.5 + i as f32 * 0.35;
                let x = cx + (i as f32 - 1.0) * 62.0 * u;
                let y = card.y + 104.0 * u;
                g.spr_fit(spr::IC_STAR, x, y, 46.0 * u, 0.0, alpha(INK, 0.14));
                if i < o.stars && t > at {
                    let k = ease_out(((t - at) / 0.35).min(1.0));
                    g.spr_fit(spr::IC_STAR, x, y, 46.0 * u * (1.25 - 0.25 * k), 0.0, alpha(GOLD, k));
                    if o.shown <= i {
                        o.shown = i + 1;
                        app.au.play(Sfx::Star(i), 0.8);
                    }
                }
            }
            let g = &app.g;
            g.text_c(&format!("The home held at {} of {}. {} invaders stopped.", self.sim.hearts, self.sim.max_hearts, self.sim.stopped), cx, card.y + 148.0 * u, 12.5 * u, DIM);
            if o.first {
                let r = Rect::new(cx - 130.0 * u, card.y + 164.0 * u, 260.0 * u, 52.0 * u);
                chip(g, r, 1.0);
                g.label("+1 skill point", cx, r.y + 17.0 * u, 14.0 * u, 0.5, GOLD, None, 0.0);
                g.label("+1 bond point for the pack", cx, r.y + 36.0 * u, 12.5 * u, 0.5, FROST, None, 0.0);
            } else {
                g.text_c("Held before: no new points.", cx, card.y + 190.0 * u, 12.5 * u, DIM);
            }
            let ready = t > 1.3;
            let bw = 200.0 * u;
            app.mark("continue", Rect::new(cx - bw / 2.0 - 26.0 * u, card.y + 232.0 * u, bw, 54.0 * u));
            if app.ui.button(&app.g, &mut app.au, 70, Rect::new(cx - bw / 2.0 - 26.0 * u, card.y + 232.0 * u, bw, 54.0 * u), GO, "MOVE ON", Some(spr::IC_NEXT), ready) || (self.auto > 1 && t > 2.5) {
                nav = Some(Nav::Map);
            }
            let retry = Rect::new(cx + bw / 2.0 - 18.0 * u, card.y + 232.0 * u, 54.0 * u, 54.0 * u);
            if app.ui.button(&app.g, &mut app.au, 71, retry, STEEL, "", Some(spr::IC_RETRY), ready) {
                nav = Some(Nav::Play(level));
            }
        } else {
            let hero = self.sim.units[0].who;
            portrait(g, cr_face(hero), cx, card.y + 108.0 * u, 72.0 * u, alpha(INK, 0.35), rgb(0x777777));
            g.text_c(&format!("They broke through on wave {} of {}.", self.sim.wave, self.sim.waves.len()), cx, card.y + 162.0 * u, 13.0 * u, DIM);
            let bw = 230.0 * u;
            let bx = cx - bw / 2.0;
            app.mark("again", Rect::new(bx, card.y + 180.0 * u, bw, 50.0 * u));
            if app.ui.button(&app.g, &mut app.au, 72, Rect::new(bx, card.y + 180.0 * u, bw, 50.0 * u), GO, "TRY AGAIN", Some(spr::IC_RETRY), t > 0.8) || (self.auto > 1 && t > 2.5) {
                nav = Some(Nav::Play(level));
            }
            // one tap to an easier setting
            let easier = app.save.p().difficulty.easier();
            let half = (bw - 8.0 * u) / 2.0;
            if let Some(e) = easier {
                app.mark("easier", Rect::new(bx, card.y + 238.0 * u, half, 46.0 * u));
                if app.ui.button(&app.g, &mut app.au, 73, Rect::new(bx, card.y + 238.0 * u, half, 46.0 * u), STEEL, &format!("ON {}", e.name().to_uppercase()), None, t > 0.8) {
                    app.save.p_mut().difficulty = e;
                    app.save.store();
                    nav = Some(Nav::Play(level));
                }
            }
            let mr = if easier.is_some() { Rect::new(bx + half + 8.0 * u, card.y + 238.0 * u, half, 46.0 * u) } else { Rect::new(bx, card.y + 238.0 * u, bw, 46.0 * u) };
            if app.ui.button(&app.g, &mut app.au, 74, mr, STEEL, "MAP", Some(spr::IC_HOME), t > 0.8) {
                nav = Some(Nav::Map);
            }
        }
        if nav.is_some() {
            app.au.set_music_gain(1.0);
        }
        nav
    }
}

/// Where an invader is drawn. Each keeps a little to its own side of the
/// trail, and those being held bunch up behind one another, so a crowd
/// stopped at one spot reads as a crowd and not as a single dark heap.
fn foe_at(e: &biome_core::sim::Enemy, k: f32) -> V2 {
    let d = e.foe.def();
    let aside = ((e.id as f32 * 0.618).fract() - 0.5) * if d.boss || e.foe.rolls() { 0.0 } else { (0.46 - d.size * 0.3).max(0.0) };
    let back = if e.held.is_some() { ((e.id as f32 * 0.381).fract() - 0.2) * 0.5 } else { 0.0 };
    e.prev.lerp(e.pos, k) + e.dir.perp() * aside - e.dir * back
}

/// A circle of reach on the ground.
fn ring(c: Vec2, r: f32, cell: f32, color: Color) {
    draw_circle(c.x, c.y, r, alpha(color, 0.07 * color.a));
    draw_circle_lines(c.x, c.y, r, cell * 0.03, alpha(color, 0.7 * color.a));
}
