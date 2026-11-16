//! A stage being played. The rules live in `biome_core::sim`; this screen
//! steps them, turns what they report into pictures and sound, and passes
//! on what the player taps.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::content::{LOSE_WORDS, WIN_WORDS};
use crate::field::{draw_ground, Scenery, View};
use crate::fx::Fx;
use crate::gfx::*;
use crate::ui::*;
use crate::widgets::*;
use crate::{App, Nav};
use biome_core::bot::{Bot, Style};
use biome_core::creature::Cr;
use biome_core::defs::{Look, Mode, LV_DMG, LV_RANGE, LV_RATE, LV_SLOW};
use biome_core::sim::{BlockKind, Ev, Phase, Setup, Sim, ZoneKind, LEASH, TICK};
use biome_core::{Foe, Tw, BIOMES, V2};
use macroquad::prelude::{draw_circle, draw_circle_lines, draw_line, vec2, Color, Rect, Vec2};
use std::f32::consts::{PI, TAU};

const HUD_H: f32 = 48.0;
const STRIP_H: f32 = 38.0;
const CARD_H: f32 = 86.0;

#[derive(Clone, Copy, PartialEq)]
enum Menu {
    None,
    Build(usize),
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
    Mound(usize),
    Home,
    Block(usize),
    Unit(usize),
    Foe(usize),
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
    beams: Vec<Beam>,
    leaps: Vec<Leap>,
    /// Where creatures have been sent: (place, seconds left to show it).
    marks: Vec<(V2, f32)>,
    /// How many invaders have been shooed in quick succession (the pops climb a scale).
    run: u8,
    run_t: f32,
    banner: Option<(String, f32)>,
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

impl Play {
    pub fn new(app: &mut App, level: usize) -> Play {
        let p = app.save.p();
        let setup = Setup { level, difficulty: p.difficulty, pack: p.pack(), guests: p.guests(level), seed: app.rng.next_u64() };
        let teach = level == 0 && p.stars[0] == 0;
        let sim = Sim::new(&setup);
        let pal = palette(sim.def.biome);
        let sc = Scenery::new(&sim.map, &pal, 77 + level as u64 * 13);
        let auto = app.q.num("auto").unwrap_or(0.0) as u8;
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
            beams: Vec::new(),
            leaps: Vec::new(),
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

    fn view(&self, app: &App) -> View {
        let u = app.u;
        let top = app.top + HUD_H * u;
        let bottom = app.h - app.bot - (STRIP_H + CARD_H) * u;
        View::fit(Rect::new(app.x0, top, app.pw, (bottom - top).max(50.0)))
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
            b.age < 0.2
        });
        self.leaps.retain_mut(|l| {
            l.age += dt;
            l.age < 0.22
        });
        self.marks.retain_mut(|m| {
            m.1 -= dt;
            m.1 > 0.0
        });
        if let Some((_, life)) = &mut self.banner {
            *life -= dt;
            if *life <= 0.0 {
                self.banner = None;
            }
        }
        if self.sim.phase == Phase::Ready {
            self.intro += dt;
        }
        self.ambient(app, &v, dt);
        let k = (self.acc / TICK).clamp(0.0, 1.0);

        // ---- the field ----------------------------------------------------
        let shake = self.fx.offset();
        let mut sv = v;
        sv.origin += shake;
        let full = Rect::new(app.x0 - 20.0, -20.0, app.pw + 40.0, app.h + 40.0);
        draw_ground(&app.g, &sv, &self.sim.map, &self.pal, &self.sc, full, self.t);
        self.draw_world(app, &sv, k);
        self.fx.draw(&app.g);
        if let Some((top, bottom)) = mood_wash(self.sim.def.mood) {
            app.g.vgrad(Rect::new(0.0, 0.0, app.w, app.h), top, bottom);
            self.draw_lights(app, &sv, k);
        }
        self.draw_bars(&sv, k);
        self.fx.draw_text(&app.g);
        if self.home_hit > 0.0 {
            app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(rgb(0xff3020), 0.22 * self.home_hit));
        }
        // letterbox on wide screens
        if app.x0 > 1.0 {
            app.g.rect(Rect::new(0.0, 0.0, app.x0, app.h), rgb(0x16240f));
            app.g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), rgb(0x16240f));
        }

        // ---- the interface ------------------------------------------------
        let used = self.hud(app, &v) | self.bottom(app, &v);
        if !busy && !used {
            self.field_taps(app, &v);
        }
        self.draw_banners(app, &v);
        if self.teach && !busy {
            self.draw_hints(app, &v);
        }
        if app.debug && app.tick(20) {
            for (i, m) in self.sim.map.mounds.iter().enumerate() {
                let c = v.at(*m);
                app.log(&format!("MOUND {} {:.0} {:.0} {}", i, c.x, c.y, self.sim.towers[i].is_some() as u8));
            }
            app.log(&format!("STATE phase={:?} wave={}/{} hearts={} seeds={} foes={}", self.sim.phase, self.sim.wave, self.sim.waves.len(), self.sim.hearts, self.sim.seeds, self.sim.foes.len()));
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

    fn on_event(&mut self, app: &mut App, v: &View, ev: Ev) {
        let u = app.u;
        let cell = v.cell;
        match ev {
            Ev::Spawn { .. } => {}
            Ev::Fire { spot, .. } => {
                let look = self.sim.towers[spot].as_ref().map(|t| t.kind.def().look).unwrap_or(Look::None);
                let sfx = match look {
                    Look::Spore | Look::Acorn | Look::Rock | Look::Snowball => Sfx::Lob,
                    Look::Zap => Sfx::Zap,
                    Look::Peck | Look::Dive => Sfx::Peck,
                    Look::Tongue => Sfx::Tongue,
                    _ => Sfx::Dart,
                };
                app.au.play(sfx, 0.35);
            }
            Ev::Beam { look, a, b } => self.beams.push(Beam { look, a: v.at(a), b: v.at(b), age: 0.0 }),
            Ev::Hit { at, look, crit } => {
                let c = v.at(at);
                self.fx.burst(c, look_color(look), if crit { 7 } else { 3 }, 70.0 * u, 5.0 * u, 0.0);
                if crit {
                    self.fx.text("CRIT!", c - vec2(0.0, cell * 0.5), 13.0 * u, rgb(0xfff07a), rgb(0x6a3a00), 0.6);
                }
            }
            Ev::Burst { at, radius, look } => {
                let c = v.at(at);
                let col = look_color(look);
                self.fx.ring(c, cell * 0.2, cell * radius.max(0.4), 0.3, col, 0.0);
                self.fx.burst(c, col, 9, 120.0 * u, 6.0 * u, 0.0);
                for _ in 0..3 {
                    self.fx.puff(c, 10.0 * u, vec2(0.0, 0.0), alpha(mix(col, WHITE, 0.4), 0.7));
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
                    pos: c - vec2(0.0, cell * 0.2),
                    vel: vec2(0.0, 0.0),
                    gravity: 0.0,
                    drag: 0.0,
                    rot: self.rng.range(-0.8, 0.8),
                    spin: 0.0,
                    size: cell * 0.7,
                    grow: 1.3,
                    age: 0.0,
                    life: 0.16,
                    color: WHITE,
                });
            }
            Ev::Shooed { at, foe, seeds } => {
                let c = v.at(at);
                let d = foe.def();
                self.run = (self.run + 1).min(8);
                self.run_t = 0.5;
                app.au.play(Sfx::Shoo(self.run - 1), if d.boss { 0.9 } else { 0.45 });
                // a puff of dust, and it is gone
                for _ in 0..(if d.boss { 14 } else { 3 }) {
                    let off = vec2(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 0.4)) * cell * d.size * 0.4;
                    self.fx.puff(c + off, cell * 0.3 * d.size.max(0.6), vec2(0.0, -10.0 * u), alpha(WHITE, 0.8));
                }
                self.fx.burst(c, rgb(0xfff2c0), if d.boss { 30 } else { 5 }, 110.0 * u, 5.0 * u, 0.0);
                if seeds > 0 {
                    self.fx.text(&format!("+{seeds}"), c - vec2(0.0, cell * 0.3), (if d.boss { 20.0 } else { 12.0 }) * u, rgb(0xffe08a), rgb(0x5a3200), 0.7);
                    self.seed_pop = 1.0;
                }
                if d.boss {
                    self.fx.shake(9.0 * u);
                    self.fx.ring(c, cell * 0.3, cell * 3.0, 0.6, WHITE, 0.0);
                    self.fx.confetti(app.x0, app.x0 + app.pw, app.top, 50, u);
                    self.banner = Some((format!("{} TURNS BACK!", d.name.to_uppercase()), 2.6));
                }
            }
            Ev::Leak { at, hearts, .. } => {
                let c = v.at(at);
                app.au.play(Sfx::Leak, 0.8);
                crate::platform::vibrate(40);
                self.heart_pop = 1.0;
                self.home_hit = 1.0;
                self.fx.shake(6.0 * u);
                self.fx.text(&format!("-{}", hearts.max(1)), c - vec2(0.0, cell * 0.8), 20.0 * u, rgb(0xff6a5a), rgb(0x5a0a0a), 0.9);
            }
            Ev::Cast { who, at, radius, .. } => {
                let c = v.at(at);
                let r = radius * cell;
                match who {
                    Cr::Fox => {
                        app.au.play(Sfx::Pounce, 0.8);
                        self.fx.ring(c, cell * 0.2, cell * 0.9, 0.3, rgb(0xffb060), 0.0);
                        self.fx.burst(c, rgb(0xffb060), 12, 140.0 * u, 6.0 * u, 0.0);
                    }
                    Cr::Badger => {
                        app.au.play(Sfx::Slam, 0.9);
                        self.fx.shake(10.0 * u);
                        self.fx.ring(c, cell * 0.3, r, 0.4, rgb(0xfff2c0), 0.0);
                        self.fx.ring(c, cell * 0.2, r * 0.7, 0.4, rgb(0xd0a060), 0.08);
                        for i in 0..14 {
                            let a = i as f32 / 14.0 * TAU;
                            self.fx.puff(c + vec2(a.cos(), a.sin() * 0.7) * r * 0.6, 12.0 * u, vec2(a.cos(), a.sin()) * 40.0 * u, alpha(rgb(0xd8c0a0), 0.85));
                        }
                    }
                    Cr::Owl => {
                        app.au.play(Sfx::Gust, 0.8);
                        self.fx.ring(c, cell * 0.3, r, 0.45, WHITE, 0.0);
                        for i in 0..16 {
                            let a = i as f32 / 16.0 * TAU;
                            self.fx.drift(spr::STREAK, c + vec2(a.cos(), a.sin()) * cell * 0.3, vec2(a.cos(), a.sin()) * 190.0 * u, 14.0 * u, 0.4, alpha(WHITE, 0.8), 0.0);
                        }
                        for _ in 0..5 {
                            let off = vec2(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)) * r * 0.5;
                            self.fx.drift(spr::P_FEATHER, c + off, vec2(self.rng.range(-30.0, 30.0), -20.0) * u, 12.0 * u, 0.9, rgb(0xd9c8f0), 60.0 * u);
                        }
                    }
                    Cr::Rabbit => {
                        app.au.play(Sfx::Thump, 0.9);
                        self.fx.shake(6.0 * u);
                        self.fx.ring(c, cell * 0.2, r, 0.35, rgb(0xfff2c0), 0.0);
                        self.fx.burst(c, rgb(0xfff2c0), 10, 120.0 * u, 6.0 * u, 0.0);
                    }
                    Cr::Squirrel => {
                        app.au.play(Sfx::Storm, 0.8);
                        self.fx.ring(c, cell * 0.3, r, 0.5, rgb(0xd9a060), 0.0);
                    }
                    Cr::Beaver => {
                        app.au.play(Sfx::Dam, 0.9);
                        self.fx.shake(4.0 * u);
                        self.fx.chunks(spr::P_BIT, c, rgb(0xa8744a), 10, 130.0 * u, 7.0 * u, 300.0 * u);
                    }
                    Cr::Tortoise => {
                        app.au.play(Sfx::Shell, 0.9);
                        self.fx.ring(c, cell * 0.2, cell * 0.9, 0.35, rgb(0xd8f08a), 0.0);
                    }
                    Cr::Marmot => {
                        app.au.play(Sfx::Whistle, 0.8);
                        for i in 0..3 {
                            self.fx.ring(c, cell * 0.2, r, 0.6, rgb(0xfff07a), i as f32 * 0.12);
                        }
                        for _ in 0..5 {
                            let off = vec2(self.rng.range(-0.6, 0.6), self.rng.range(-0.8, -0.2)) * cell;
                            self.fx.drift(spr::P_NOTE, c + off, vec2(self.rng.range(-20.0, 20.0), -50.0) * u, 12.0 * u, 0.9, rgb(0xfff07a), 0.0);
                        }
                    }
                }
                self.cast_once = true;
            }
            Ev::Leap { unit, from, to } => self.leaps.push(Leap { unit, from, to, age: 0.0 }),
            Ev::Ko { unit } => {
                let c = v.at(self.sim.units[unit].pos);
                app.au.play(Sfx::Ko, 0.7);
                self.fx.text("OUCH", c - vec2(0.0, cell * 0.7), 14.0 * u, rgb(0xffd0c0), rgb(0x5a1a0a), 0.9);
            }
            Ev::Up { unit } => {
                let c = v.at(self.sim.units[unit].pos);
                app.au.play(Sfx::Up, 0.6);
                self.fx.sparkle(c - vec2(0.0, cell * 0.4), cell * 0.5, 8, 9.0 * u, rgb(0xfff07a));
            }
            Ev::Stand { unit } => {
                let c = v.at(self.sim.units[unit].pos);
                app.au.play(Sfx::Slam, 0.6);
                self.fx.text("NOT YET!", c - vec2(0.0, cell * 0.8), 15.0 * u, rgb(0xfff07a), rgb(0x5a3200), 1.0);
                self.fx.ring(c, cell * 0.3, cell * 1.4, 0.5, rgb(0xfff07a), 0.0);
            }
            Ev::Built { spot } => {
                let c = v.at(self.sim.map.mounds[spot]);
                app.au.play(Sfx::Build, 0.8);
                crate::platform::vibrate(12);
                for i in 0..8 {
                    let a = i as f32 / 8.0 * TAU;
                    self.fx.puff(c + vec2(a.cos(), a.sin() * 0.5) * cell * 0.4, 9.0 * u, vec2(a.cos() * 30.0, -10.0) * u, alpha(mix(self.pal.mound, WHITE, 0.4), 0.85));
                }
                self.fx.sparkle(c - vec2(0.0, cell * 0.3), cell * 0.4, 6, 8.0 * u, rgb(0xfff2c0));
                app.log(&format!("BUILT {spot}"));
            }
            Ev::Upgraded { spot } => {
                let c = v.at(self.sim.map.mounds[spot]);
                app.au.play(Sfx::Upgrade, 0.8);
                self.fx.ring(c, cell * 0.2, cell * 0.9, 0.4, rgb(0xfff07a), 0.0);
                self.fx.sparkle(c - vec2(0.0, cell * 0.3), cell * 0.5, 10, 9.0 * u, rgb(0xfff07a));
                self.fx.text("LEVEL UP", c - vec2(0.0, cell * 0.9), 13.0 * u, rgb(0xfff07a), rgb(0x5a3200), 0.8);
            }
            Ev::Sold { spot, seeds } => {
                let c = v.at(self.sim.map.mounds[spot]);
                app.au.play(Sfx::Sell, 0.8);
                self.fx.text(&format!("+{seeds}"), c - vec2(0.0, cell * 0.4), 14.0 * u, rgb(0xffe08a), rgb(0x5a3200), 0.8);
                self.seed_pop = 1.0;
            }
            Ev::Wave { n, seeds } => {
                app.au.play(Sfx::Wave, 0.8);
                let last = n == self.sim.waves.len();
                self.banner = Some((if last { "LAST WAVE".to_string() } else { format!("WAVE {n}") }, 1.6));
                if seeds > 0 {
                    self.seed_pop = 1.0;
                    let at = vec2(app.x0 + 150.0 * u, app.top + 62.0 * u);
                    self.fx.text(&format!("+{seeds}"), at, 14.0 * u, rgb(0xffe08a), rgb(0x5a3200), 1.0);
                }
                app.log(&format!("WAVE {n}"));
            }
            Ev::Jam { at, radius } => {
                let c = v.at(at);
                app.au.play(Sfx::Jam, 0.7);
                self.fx.ring(c, cell * 0.3, radius * cell, 0.5, rgb(0xb8f08a), 0.0);
                for _ in 0..10 {
                    let off = vec2(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)) * radius * cell * 0.6;
                    self.fx.puff(c + off, 14.0 * u, vec2(0.0, -12.0 * u), alpha(rgb(0xb8f08a), 0.6));
                }
            }
            Ev::Scare { at, radius } => {
                let c = v.at(at);
                app.au.play(Sfx::Rattle, 0.7);
                self.fx.ring(c, cell * 0.3, radius * cell, 0.45, rgb(0xffd23a), 0.0);
                self.fx.text("EEK!", c - vec2(0.0, cell * 0.9), 13.0 * u, rgb(0xfff07a), rgb(0x5a3200), 0.7);
            }
            Ev::Root { at, radius } => {
                let c = v.at(at);
                app.au.play(Sfx::Root, 0.6);
                self.fx.ring(c, cell * 0.3, radius * cell, 0.4, rgb(0xd8f0ff), 0.0);
            }
            Ev::BlockUp { at, kind } => {
                let c = v.at(at);
                if kind != BlockKind::Dam {
                    self.fx.burst(c, rgb(0xfff2c0), 8, 100.0 * u, 6.0 * u, 0.0);
                }
            }
            Ev::BlockDown { at, kind, flood } => {
                let c = v.at(at);
                match kind {
                    BlockKind::Dam => {
                        self.fx.chunks(spr::P_BIT, c, rgb(0xa8744a), 12, 150.0 * u, 7.0 * u, 320.0 * u);
                        app.au.play(Sfx::Dam, 0.6);
                    }
                    _ => {
                        for _ in 0..4 {
                            self.fx.puff(c, 12.0 * u, vec2(0.0, -10.0 * u), alpha(WHITE, 0.8));
                        }
                    }
                }
                if flood {
                    self.fx.spray(c, 26, 220.0 * u, 7.0 * u);
                    self.fx.ring(c, cell * 0.3, cell * 1.7, 0.5, rgb(0x9fdcff), 0.0);
                    app.au.play(Sfx::Gust, 0.6);
                }
            }
            Ev::Swallow { at } => {
                let c = v.at(at);
                self.fx.text("GULP", c - vec2(0.0, cell * 0.5), 13.0 * u, rgb(0xb8f08a), rgb(0x1f4a12), 0.7);
            }
            Ev::Haste { spot } => {
                let c = v.at(self.sim.map.mounds[spot]);
                self.fx.sparkle(c - vec2(0.0, cell * 0.4), cell * 0.4, 5, 8.0 * u, rgb(0xfff07a));
            }
            Ev::Mend => {
                self.heart_pop = 1.0;
                let c = v.at(self.sim.map.home);
                self.fx.hearts(c - vec2(0.0, cell * 0.8), 2, u);
            }
            Ev::Boss { foe } => {
                app.au.play(Sfx::Boss, 1.0);
                crate::platform::vibrate(80);
                self.fx.shake(8.0 * u);
                self.banner = Some((foe.def().name.to_uppercase(), 3.0));
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
            self.fx.confetti(app.x0, app.x0 + app.pw, app.top, 90, app.u);
            app.log(&format!("WON {stars}"));
        } else {
            app.au.play(Sfx::Lose, 0.9);
            app.log("LOST");
        }
        app.au.set_music_gain(0.35);
        let words: &[&'static str] = if won { &WIN_WORDS } else { &LOSE_WORDS };
        self.over = Some(Over { won, stars, t: 0.0, first, word: words[self.rng.below(words.len() as u32) as usize], shown: 0 });
        self.menu = Menu::None;
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
            Drift::Petals => {
                self.drift_cd = 0.5;
                self.fx.drift(spr::P_PETAL, vec2(x, r.y - 10.0), vec2(self.rng.range(10.0, 40.0), self.rng.range(30.0, 55.0)) * u, 8.0 * u, 7.0, alpha(rgb(0xffd6ea), 0.85), 4.0 * u);
            }
            Drift::Leaves => {
                self.drift_cd = 0.6;
                let col = [rgb(0xe8a030), rgb(0xd9603a), rgb(0x9bc04a)][self.rng.below(3) as usize];
                self.fx.drift(spr::P_LEAF, vec2(x, r.y - 10.0), vec2(self.rng.range(-25.0, 25.0), self.rng.range(35.0, 60.0)) * u, 10.0 * u, 7.0, alpha(col, 0.9), 5.0 * u);
            }
            Drift::Fireflies => {
                self.drift_cd = 0.35;
                self.fx.drift(spr::GLOW, vec2(x, y), vec2(self.rng.range(-12.0, 12.0), self.rng.range(-14.0, 4.0)) * u, 9.0 * u, 2.6, alpha(rgb(0xf4ff8a), 0.8), 0.0);
            }
            Drift::Dust => {
                self.drift_cd = 0.3;
                self.fx.drift(spr::P_DOT, vec2(r.x - 6.0, y), vec2(self.rng.range(70.0, 130.0), self.rng.range(-8.0, 8.0)) * u, 4.0 * u, 4.5, alpha(rgb(0xfff0d0), 0.5), 0.0);
            }
            Drift::Snow => {
                self.drift_cd = 0.16;
                self.fx.drift(spr::P_SNOW, vec2(x, r.y - 10.0), vec2(self.rng.range(-20.0, 10.0), self.rng.range(35.0, 70.0)) * u, self.rng.range(4.0, 8.0) * u, 9.0, alpha(WHITE, 0.9), 0.0);
            }
            Drift::Smoke => {
                self.drift_cd = 0.7;
                self.fx.drift(spr::P_SMOKE, vec2(x, y), vec2(self.rng.range(10.0, 30.0), self.rng.range(-30.0, -12.0)) * u, 22.0 * u, 3.5, alpha(rgb(0x6a6a6a), 0.22), 0.0);
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

    fn draw_world(&self, app: &App, v: &View, k: f32) {
        let g = &app.g;
        let sim = &self.sim;
        let cell = v.cell;
        let t = self.t;

        // where invaders come in
        for (i, &s) in sim.map.spawns.iter().enumerate() {
            let c = v.at(s);
            let coming = sim.upcoming().is_some_and(|w| w.groups.iter().any(|gr| gr.spawn as usize == i)) && sim.can_call();
            if coming {
                let pulse = 0.5 + 0.5 * (t * 4.0).sin();
                g.spr(spr::GLOW, c.x, c.y, cell * (1.6 + 0.3 * pulse), cell * (1.6 + 0.3 * pulse), 0.0, alpha(rgb(0xff5a3a), 0.35 + 0.2 * pulse));
            }
        }

        // the ground stained by slowing fields, and lingering effects
        for (spot, tw) in sim.towers.iter().enumerate() {
            let Some(tw) = tw else { continue };
            if let Some(col) = field_color(tw.kind) {
                let c = v.at(tw.pos);
                let r = sim.reach(spot) * cell;
                let a = if tw.jam > 0.0 { 0.06 } else { 0.2 };
                draw_circle(c.x, c.y, r, alpha(col, a));
                draw_circle_lines(c.x, c.y, r * (0.9 + 0.1 * (t * 1.5 + spot as f32).sin()), cell * 0.04, alpha(col, a * 1.6));
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

        // reach of whatever the open menu is about, or of the chosen creature
        match self.menu {
            Menu::Tower(spot) if sim.towers[spot].is_some() => {
                let c = v.at(sim.map.mounds[spot]);
                ring(c, sim.reach(spot) * cell, cell, WHITE);
            }
            Menu::Build(spot) => {
                let c = v.at(sim.map.mounds[spot]);
                ring(c, 2.2 * cell, cell, alpha(WHITE, 0.6));
            }
            _ => {}
        }
        if let Some(un) = sim.units.get(self.sel) {
            if !un.guest && self.menu == Menu::None {
                let c = v.at(un.post);
                let d = un.who.def();
                let r = if d.perched { un.st.range } else { LEASH };
                ring(c, r * cell, cell, alpha(cr_color(un.who), 0.55));
            }
        }
        for &(p, life) in &self.marks {
            let c = v.at(p);
            let a = (life / 0.3).min(1.0);
            g.spr_fit(spr::FX_PAW, c.x, c.y, cell * (0.55 + 0.1 * (t * 8.0).sin()), 0.0, alpha(WHITE, 0.9 * a));
        }

        // everything standing on the ground, back to front
        let mut items: Vec<(f32, Item)> = Vec::with_capacity(64 + sim.foes.len());
        for (i, p) in self.sc.props.iter().enumerate() {
            items.push((p.pos.y, Item::Prop(i)));
        }
        for (i, m) in sim.map.mounds.iter().enumerate() {
            items.push((m.y + 0.2, Item::Mound(i)));
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
                items.push((e.pos.y, Item::Foe(i)));
            }
        }
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (_, item) in &items {
            match *item {
                Item::Prop(i) => {
                    let p = &self.sc.props[i];
                    let c = v.at(p.pos);
                    let h = p.size * cell;
                    let w = h * p.spr.w / p.spr.h;
                    g.spr(spr::SHADOW, c.x, c.y, w * 0.8, w * 0.28, 0.0, alpha(WHITE, 0.6));
                    let sway = if p.size > 1.0 { (t * 0.9 + p.pos.x * 1.7).sin() * 0.02 } else { 0.0 };
                    g.spr_ex(p.spr, c.x, c.y - h * 0.45, w, h, sway, WHITE, p.flip);
                }
                Item::Mound(i) => self.draw_mound(app, v, i),
                Item::Home => {
                    let c = v.at(sim.map.home);
                    let s = cell * 1.9;
                    let jolt = self.home_hit * (t * 50.0).sin() * cell * 0.05;
                    g.spr(self.pal.home, c.x + jolt, c.y - s * 0.16, s, s, 0.0, mix(WHITE, rgb(0xff8070), self.home_hit * 0.6));
                }
                Item::Block(i) => {
                    let b = &sim.blocks[i];
                    let c = v.at(b.pos);
                    let fade = (b.t / 0.6).min(1.0);
                    let (s, size) = match b.kind {
                        BlockKind::Dam => (spr::DAM, 1.15),
                        BlockKind::Decoy => (spr::DECOY, 0.95),
                        BlockKind::Cousin => (spr::COUSIN, 0.8),
                    };
                    let h = size * cell;
                    g.spr(spr::SHADOW, c.x, c.y + h * 0.1, h * 0.8, h * 0.28, 0.0, alpha(WHITE, 0.6 * fade));
                    let wob = if b.kind == BlockKind::Decoy { (t * 3.0).sin() * 0.08 } else { 0.0 };
                    g.spr(s, c.x, c.y - h * 0.3, h, h, wob, alpha(WHITE, fade));
                }
                Item::Unit(i) => self.draw_unit(app, v, i, k),
                Item::Foe(i) => self.draw_foe(app, v, i, k),
            }
        }
        // whirlwinds, then fliers above everything
        for z in &sim.zones {
            if let ZoneKind::Twister(_) = z.kind {
                let c = v.at(z.pos);
                let s = cell * 1.6 * (z.t / 0.4).min(1.0);
                g.spr(spr::TWISTER, c.x + (t * 9.0).sin() * cell * 0.08, c.y - s * 0.35, s * (0.9 + 0.1 * (t * 13.0).sin()), s, (t * 6.0).sin() * 0.1, alpha(WHITE, 0.8));
            }
        }
        for i in 0..sim.foes.len() {
            if sim.foes[i].foe.def().flying {
                self.draw_foe(app, v, i, k);
            }
        }
        self.draw_shots(app, v, k);
    }

    fn draw_mound(&self, app: &App, v: &View, i: usize) {
        let g = &app.g;
        let sim = &self.sim;
        let cell = v.cell;
        let t = self.t;
        let c = v.at(sim.map.mounds[i]);
        let chosen = matches!(self.menu, Menu::Build(s) | Menu::Tower(s) if s == i);
        let Some(tw) = &sim.towers[i] else {
            // an empty mound beckons when there are seeds to spend on it
            let cheapest = BIOMES[sim.def.biome].towers.iter().map(|k| sim.price(*k, 1)).min().unwrap_or(0);
            let can = sim.seeds >= cheapest && sim.playing();
            let bob = if can { (t * 3.0 + i as f32).sin() * 0.03 } else { 0.0 };
            let s = cell * (1.08 + bob + if chosen { 0.1 } else { 0.0 });
            g.spr(spr::MOUND, c.x, c.y + cell * 0.04, s, s, 0.0, self.pal.mound);
            if can {
                let a = 0.55 + 0.35 * (t * 3.0 + i as f32).sin();
                g.spr_fit(spr::IC_UP, c.x, c.y - cell * (0.2 + 0.06 * (t * 3.0 + i as f32).sin()), cell * 0.42, 0.0, alpha(rgb(0xfff07a), a));
            }
            return;
        };
        let s = cell * 1.1;
        g.spr(spr::MOUND, c.x, c.y + cell * 0.06, s, s, 0.0, self.pal.mound);
        // a kick when it fires, a pulse when it levels
        let kick = (1.0 - tw.fired / 0.16).clamp(0.0, 1.0);
        let size = cell * (1.18 + 0.07 * (tw.level as f32 - 1.0)) * if chosen { 1.08 } else { 1.0 };
        let (w, h) = (size * (1.0 + 0.1 * kick), size * (1.0 - 0.12 * kick));
        let jammed = tw.jam > 0.0;
        let col = if jammed { rgb(0x8a9a8a) } else if tw.haste > 0.0 { mix(WHITE, rgb(0xfff07a), 0.3 + 0.3 * (t * 12.0).sin().abs()) } else { WHITE };
        let lean = if jammed { (t * 2.0).sin() * 0.05 } else { ((tw.aim.x - tw.pos.x) * 0.03).clamp(-0.08, 0.08) * kick };
        g.spr(tower_spr(tw.kind), c.x, c.y - size * 0.28 + (size - h) * 0.5, w, h, lean, col);
        if jammed {
            g.spr_fit(spr::P_SMOKE, c.x + (t * 3.0).sin() * cell * 0.1, c.y - cell * 0.75, cell * 0.5, 0.0, alpha(rgb(0xb8f08a), 0.75));
        }
        for n in 0..tw.level {
            let x = c.x + (n as f32 - (tw.level as f32 - 1.0) / 2.0) * cell * 0.22;
            g.spr_fit(spr::IC_STAR, x, c.y + cell * 0.36, cell * 0.24, 0.0, WHITE);
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
        let size = cell * if un.guest { 1.05 } else { 1.22 };
        let chosen = i == self.sel && !un.guest;
        if chosen {
            let pulse = 1.0 + 0.06 * (t * 5.0).sin();
            g.spr(spr::RING, c.x, c.y + cell * 0.08, cell * 1.15 * pulse, cell * 0.6 * pulse, 0.0, alpha(WHITE, 0.9));
        }
        if !un.alive() {
            // flat out, with a clock running down
            g.spr(spr::SHADOW, c.x, c.y + cell * 0.1, size * 0.8, size * 0.26, 0.0, alpha(WHITE, 0.5));
            g.spr(cr_spr(un.who), c.x, c.y - size * 0.1, size, size, 1.45, alpha(rgb(0x9aa0a8), 0.85));
            g.spr_fit(spr::P_ZZZ, c.x + cell * 0.3, c.y - cell * (0.5 + 0.1 * (t * 2.0).sin()), cell * 0.3, 0.0, WHITE);
            cooldown_ring(c.x, c.y - cell * 0.1, cell * 0.48, cell * 0.07, un.ko / un.st.revive.max(0.1), alpha(WHITE, 0.8));
            return;
        }
        let moving = un.pos.dist(un.prev) > 1e-4;
        let lift = if d.perched { cell * 0.42 } else { 0.0 } + hop * cell;
        let bob = if un.who == Cr::Owl {
            (t * 3.0 + i as f32).sin() * cell * 0.06
        } else if moving {
            (t * 15.0).sin().abs() * cell * 0.1
        } else {
            0.0
        };
        // a lunge on every attack
        let lunge = (1.0 - un.swung / 0.16).clamp(0.0, 1.0) * cell * 0.16 * un.face;
        let breathe = 1.0 + 0.025 * (t * 2.6 + i as f32 * 1.3).sin();
        g.spr(spr::SHADOW, c.x, c.y + cell * 0.1, size * (0.75 - lift / cell * 0.25), size * 0.26, 0.0, alpha(WHITE, 0.7));
        if d.perched && un.who != Cr::Owl {
            // something to perch on
            g.spr(self.pal.rock, c.x, c.y - cell * 0.02, cell * 0.62, cell * 0.62, 0.0, WHITE);
        }
        let (sprite, tint) = if un.shell > 0.0 { (spr::TORTOISE_SHELL, WHITE) } else { (cr_spr(un.who), WHITE) };
        let y = c.y - size * 0.34 - lift - bob;
        if un.stand > 0.0 || un.shell > 0.0 {
            let col = if un.stand > 0.0 { rgb(0xfff07a) } else { rgb(0xd8f08a) };
            g.spr(spr::GLOW, c.x, y, size * 1.7, size * 1.7, 0.0, alpha(col, 0.6 + 0.2 * (t * 10.0).sin()));
        }
        g.spr_ex(sprite, c.x + lunge, y, size, size * breathe, if moving { (t * 15.0).sin() * 0.06 } else { 0.0 }, tint, un.face < 0.0);
        if un.frenzy > 0 {
            for n in 0..un.frenzy {
                g.spr_fit(spr::SK_FLAME, c.x + (n as f32 - (un.frenzy as f32 - 1.0) / 2.0) * cell * 0.17, y - size * 0.55, cell * 0.2, 0.0, rgb(0xff8a3a));
            }
        }
        if un.guest {
            g.spr_fit(spr::P_HEART, c.x + size * 0.36, y - size * 0.34, cell * 0.26, 0.0, WHITE);
        }
    }

    fn draw_foe(&self, app: &App, v: &View, i: usize, k: f32) {
        let g = &app.g;
        let cell = v.cell;
        let t = self.t;
        let e = &self.sim.foes[i];
        let d = e.foe.def();
        let c = v.at(e.prev.lerp(e.pos, k));
        let size = d.size * cell * 1.5;
        let phase = e.id as f32 * 1.7;
        let (lift, bob) = if d.flying {
            (cell * 0.55 + size * 0.1, (t * 5.0 + phase).sin() * cell * 0.05)
        } else if e.held.is_some() && e.stun <= 0.0 {
            (0.0, (t * 16.0 + phase).sin().abs() * cell * 0.04)
        } else if e.stun > 0.0 {
            (0.0, 0.0)
        } else {
            (0.0, (t * d.speed * 9.0 + phase).sin().abs() * cell * 0.055)
        };
        let fleeing = e.fear > 0.0;
        let flip = (e.dir.x < -0.25) != fleeing;
        let rot = if e.foe == Foe::Tumbleweed {
            e.dist * 2.4
        } else if e.stun > 0.0 {
            (t * 10.0 + phase).sin() * 0.12
        } else if e.foe.rolls() {
            (t * 30.0 + phase).sin() * 0.02
        } else {
            (t * d.speed * 9.0 + phase).sin() * 0.05
        };
        let mut tint = WHITE;
        let slowed = e.slowed();
        if slowed > 0.05 {
            tint = mix(tint, rgb(0x8fc8ff), (slowed * 1.2).min(0.6));
        }
        if e.poison_t > 0.0 {
            tint = mix(tint, rgb(0x9be04a), 0.35 + 0.15 * (t * 8.0).sin());
        }
        let shadow_w = size * if d.flying { 0.5 } else { 0.75 };
        g.spr(spr::SHADOW, c.x, c.y + size * 0.16, shadow_w, shadow_w * 0.36, 0.0, alpha(WHITE, if d.flying { 0.45 } else { 0.7 }));
        let y = c.y - size * 0.2 - lift - bob;
        let sprite = foe_spr(e.foe);
        g.spr_ex(sprite, c.x, y, size, size, rot, tint, flip);
        if e.hurt_t < 0.08 {
            g.lights(|| g.spr_ex(sprite, c.x, y, size, size, rot, alpha(WHITE, 0.6), flip));
        }
        if e.stun > 0.0 {
            g.spr_fit(spr::FX_STUN, c.x, y - size * 0.5, size * 0.6, (t * 6.0).sin() * 0.3, rgb(0xfff07a));
        }
        if fleeing {
            g.spr_fit(spr::P_DROP, c.x - size * 0.3, y - size * 0.35, size * 0.22, 0.4, rgb(0x9fdcff));
            g.spr_fit(spr::P_DROP, c.x + size * 0.3, y - size * 0.4, size * 0.2, -0.4, rgb(0x9fdcff));
        }
        if e.mark_t > 0.0 {
            g.spr_fit(spr::SK_EYE, c.x, y - size * 0.55, size * 0.3, 0.0, rgb(0xff6a5a));
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
            let Some(sprite) = shot_spr(s.look) else { continue };
            let p = s.prev.lerp(s.pos, k);
            let c = v.at(p);
            if s.lob {
                let kk = (s.t / s.dur).clamp(0.0, 1.0);
                let height = (kk * PI).sin() * (0.9 + s.from.dist(s.to) * 0.2) * cell;
                g.spr(spr::SHADOW, c.x, c.y, cell * 0.3, cell * 0.11, 0.0, alpha(WHITE, 0.5));
                let size = cell * (0.4 + 0.14 * (kk * PI).sin());
                g.spr_fit(sprite, c.x, c.y - height - cell * 0.2, size, t * 9.0, WHITE);
            } else {
                let dir = s.to - s.from;
                let size = cell * if s.look == Look::Bee { 0.34 } else { 0.42 };
                let wobble = if s.look == Look::Bee { (t * 40.0).sin() * cell * 0.04 } else { 0.0 };
                g.spr_fit(sprite, c.x, c.y - cell * 0.3 + wobble, size, dir.y.atan2(dir.x), WHITE);
            }
        }
        for b in &self.beams {
            let a = 1.0 - b.age / 0.2;
            let lift = vec2(0.0, cell * 0.3);
            let (from, to) = (b.a - lift, b.b - lift * 0.6);
            match b.look {
                Look::Tongue => {
                    // out and back
                    let reach = (b.age / 0.2 * PI).sin();
                    let tip = from + (to - from) * reach;
                    draw_line(from.x, from.y, tip.x, tip.y, cell * 0.09, rgb(0xff7a9a));
                    draw_circle(tip.x, tip.y, cell * 0.09, rgb(0xff9ab4));
                }
                Look::Zap => {
                    // a crooked line that jumps about
                    let n = 5;
                    let mut prev = from;
                    let side = vec2(-(to - from).y, (to - from).x).normalize_or_zero();
                    for j in 1..=n {
                        let f = j as f32 / n as f32;
                        let jit = if j == n { 0.0 } else { ((b.age * 90.0).floor() + j as f32 * 7.3 + from.x).sin() * cell * 0.16 };
                        let pt = from + (to - from) * f + side * jit;
                        draw_line(prev.x, prev.y, pt.x, pt.y, cell * 0.1, alpha(rgb(0x8ae8ff), 0.5 * a));
                        draw_line(prev.x, prev.y, pt.x, pt.y, cell * 0.045, alpha(rgb(0xffffe0), a));
                        prev = pt;
                    }
                    g.spr(spr::GLOW, to.x, to.y, cell * 0.9, cell * 0.9, 0.0, alpha(rgb(0xfff07a), 0.6 * a));
                }
                _ => {
                    // a peck or a stoop: a pale streak that is gone at once
                    let mid = (from + to) / 2.0;
                    let dir = to - from;
                    g.spr(spr::STREAK, mid.x, mid.y, dir.length(), cell * 0.16, dir.y.atan2(dir.x), alpha(look_color(b.look), 0.9 * a));
                    if b.look == Look::Dive {
                        let at = from + dir * (b.age / 0.2).min(1.0);
                        g.spr_fit(spr::P_FEATHER, at.x, at.y, cell * 0.5, dir.y.atan2(dir.x), WHITE);
                    }
                }
            }
        }
    }

    /// Glows that show once the light goes: the home's window, things firing.
    fn draw_lights(&self, app: &App, v: &View, _k: f32) {
        let g = &app.g;
        let cell = v.cell;
        let glow = mood_glow(self.sim.def.mood);
        let t = self.t;
        g.lights(|| {
            let c = v.at(self.sim.map.home);
            g.spr(spr::GLOW, c.x, c.y - cell * 0.1, cell * 3.2, cell * 3.2, 0.0, alpha(rgb(0xffd08a), 0.34 * glow * (0.9 + 0.1 * (t * 3.0).sin())));
            for tw in self.sim.towers.iter().flatten() {
                let c = v.at(tw.pos);
                let flare = (1.0 - tw.fired / 0.25).clamp(0.0, 1.0);
                g.spr(spr::GLOW, c.x, c.y - cell * 0.3, cell * 1.7, cell * 1.7, 0.0, alpha(rgb(0xfff0c0), (0.1 + 0.3 * flare) * glow));
            }
            for (i, un) in self.sim.units.iter().enumerate() {
                if un.alive() {
                    let c = v.at(un.pos);
                    g.spr(spr::GLOW, c.x, c.y - cell * 0.3, cell * 1.6, cell * 1.6, 0.0, alpha(cr_color(un.who), 0.16 * glow * (1.0 + 0.2 * (t * 2.0 + i as f32).sin())));
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
            let c = v.at(e.prev.lerp(e.pos, k));
            let size = d.size * cell * 1.5;
            let lift = if d.flying { cell * 0.55 + size * 0.1 } else { 0.0 };
            let w = if d.boss { cell * 1.5 } else { size * 0.6 };
            let h = if d.boss { cell * 0.12 } else if d.size < 0.56 { cell * 0.05 } else { cell * 0.07 };
            tiny_bar(c.x, c.y - size * 0.72 - lift, w, h, e.hp / e.max_hp, if d.boss { rgb(0xff5a4a) } else { rgb(0xff8a4a) });
        }
        for (i, un) in self.sim.units.iter().enumerate() {
            if !un.alive() || un.who.def().perched {
                continue;
            }
            if un.hp < un.st.hp - 0.5 || un.holding > 0 {
                let c = v.at(self.unit_pos(i, k).0);
                tiny_bar(c.x, c.y - cell * 1.0, cell * 0.7, cell * 0.08, un.hp / un.st.hp, rgb(0x7ae04a));
            }
        }
        for b in &self.sim.blocks {
            if b.kind == BlockKind::Dam {
                let c = v.at(b.pos);
                tiny_bar(c.x, c.y - cell * 0.75, cell * 0.8, cell * 0.07, (b.hp / b.max_hp).min(b.t / 8.0 + 0.2), rgb(0xd9a060));
            }
        }
    }

    // ---- the top of the screen ----------------------------------------------------

    fn hud(&mut self, app: &mut App, _v: &View) -> bool {
        let u = app.u;
        let y = app.top + 5.0 * u;
        let h = 38.0 * u;
        let sim = &self.sim;
        let g = &app.g;
        let mut used = false;
        g.vgrad(Rect::new(app.x0, 0.0, app.pw, app.top + HUD_H * u + 10.0 * u), alpha(rgb(0x0c1408), 0.55), alpha(rgb(0x0c1408), 0.0));

        // hearts and seeds
        let left = Rect::new(app.x0 + 6.0 * u, y, 164.0 * u, h);
        chip(g, left, 0.9);
        let hp = 1.0 + 0.35 * self.heart_pop;
        g.spr_fit(spr::IC_HEART, left.x + 20.0 * u, left.y + h / 2.0, 24.0 * u * hp, 0.0, WHITE);
        let low = sim.hearts * 3 <= sim.max_hearts;
        g.label(&sim.hearts.to_string(), left.x + 35.0 * u, left.y + h / 2.0, 19.0 * u, 0.0, if low { rgb(0xff8a7a) } else { WHITE }, Some((alpha(SOIL, 0.8), 1.6 * u)), 0.0);
        let sp = 1.0 + 0.3 * self.seed_pop;
        g.spr_fit(spr::IC_SEED, left.x + 88.0 * u, left.y + h / 2.0, 24.0 * u * sp, 0.0, WHITE);
        g.label(&sim.seeds.to_string(), left.x + 103.0 * u, left.y + h / 2.0, 19.0 * u, 0.0, rgb(0xffe08a), Some((alpha(SOIL, 0.8), 1.6 * u)), 0.0);

        // which wave
        let mid = Rect::new(app.x0 + 176.0 * u, y, 112.0 * u, h);
        chip(g, mid, 0.9);
        let label = if sim.phase == Phase::Ready { format!("{} waves", sim.waves.len()) } else { format!("Wave {}/{}", sim.wave, sim.waves.len()) };
        g.label(&label, mid.x + mid.w / 2.0, mid.y + h / 2.0, 15.0 * u, 0.5, WHITE, Some((alpha(SOIL, 0.8), 1.4 * u)), 0.0);

        // speed and pause
        let bs = 40.0 * u;
        let speed = Rect::new(app.x0 + app.pw - 2.0 * bs - 10.0 * u, y - 1.0 * u, bs, bs);
        let pause = Rect::new(app.x0 + app.pw - bs - 6.0 * u, y - 1.0 * u, bs, bs);
        app.mark("speed", speed);
        app.mark("pause", pause);
        if app.ui.button(&app.g, &mut app.au, 10, speed, if self.speed > 1 { ORANGE } else { BLUE }, "", Some(spr::IC_SPEED), self.over.is_none()) {
            self.speed = if self.speed > 1 { 1 } else { 2 };
            used = true;
        }
        if app.ui.button(&app.g, &mut app.au, 11, pause, BLUE, "", Some(spr::IC_PAUSE), self.over.is_none() && !self.paused) {
            self.paused = true;
            used = true;
        }
        used |= app.ui.ptr.down && (speed.contains(app.ui.ptr.pos) || pause.contains(app.ui.ptr.pos));
        used
    }

    // ---- the bottom of the screen ---------------------------------------------------

    /// The strip of what is coming, then either the pack's cards or the
    /// open menu. Returns true if the pointer is busy down here.
    fn bottom(&mut self, app: &mut App, _v: &View) -> bool {
        let u = app.u;
        let top = app.h - app.bot - (STRIP_H + CARD_H) * u;
        let area = Rect::new(app.x0, top, app.pw, app.h - top);
        app.g.vgrad(Rect::new(area.x, area.y - 8.0 * u, area.w, 12.0 * u), alpha(rgb(0x0c1408), 0.0), alpha(rgb(0x0c1408), 0.5));
        app.g.vgrad(Rect::new(area.x, area.y + 4.0 * u, area.w, area.h), rgb(0x3a2a16), rgb(0x24180c));
        app.g.rect(Rect::new(area.x, area.y + 4.0 * u, area.w, 2.0 * u), alpha(rgb(0xd0a060), 0.45));
        let inside = area.contains(app.ui.ptr.pos) || (app.ui.ptr.down && area.contains(app.ui.ptr.press_pos));
        let busy = self.over.is_some() || self.paused;
        match self.menu {
            Menu::Build(spot) => self.build_menu(app, area, spot, busy),
            Menu::Tower(spot) => self.tower_menu(app, area, spot, busy),
            Menu::Foe(foe) => self.foe_card(app, area, foe),
            Menu::None => {
                self.strip(app, area, busy);
                self.cards(app, area, busy);
            }
        }
        inside
    }

    /// Who is coming next, and the button that calls them.
    fn strip(&mut self, app: &mut App, area: Rect, busy: bool) {
        let u = app.u;
        let y = area.y + 8.0 * u;
        let h = (STRIP_H - 6.0) * u;
        let sim = &self.sim;
        let g = &app.g;
        let mut tapped_foe = None;
        if let Some(wave) = sim.upcoming() {
            g.label("NEXT", area.x + 10.0 * u, y + h / 2.0, 11.0 * u, 0.0, alpha(CREAM, 0.7), None, 0.0);
            let mut x = area.x + 46.0 * u;
            for (foe, count) in wave.tally().into_iter().take(4) {
                let r = Rect::new(x, y, 46.0 * u, h);
                chip(g, r, 0.8);
                g.spr_fit(foe_spr(foe), r.x + 14.0 * u, r.y + h / 2.0, 25.0 * u, 0.0, WHITE);
                g.label(&count.to_string(), r.x + r.w - 5.0 * u, r.y + h / 2.0, 13.0 * u, 1.0, WHITE, Some((alpha(SOIL, 0.8), 1.2 * u)), 0.0);
                if app.ui.tapped(r) && !busy {
                    tapped_foe = Some(foe);
                }
                x += 49.0 * u;
            }
        } else if sim.playing() {
            g.label("That is all of them. Hold on!", area.x + 10.0 * u, y + h / 2.0, 13.0 * u, 0.0, CREAM, None, 0.0);
        }
        let call = Rect::new(area.x + area.w - 118.0 * u, y - 2.0 * u, 110.0 * u, h + 2.0 * u);
        let (label, color, on) = match sim.phase {
            Phase::Ready => ("START".to_string(), GREEN, true),
            Phase::Battle if sim.can_call() => (format!("CALL  +{}", sim.call_bonus()), ORANGE, true),
            Phase::Battle if sim.wave < sim.waves.len() => ("...".to_string(), GREY, false),
            _ => (String::new(), GREY, false),
        };
        if !label.is_empty() {
            let pulse = if sim.phase == Phase::Ready { 1.0 + 0.04 * (self.t * 5.0).sin() } else { 1.0 };
            let r = Rect::new(call.x - call.w * (pulse - 1.0) / 2.0, call.y, call.w * pulse, call.h);
            app.mark("call", r);
            if on && sim.phase == Phase::Battle {
                // the ring of time left before they come by themselves
                bar(&app.g, Rect::new(call.x, call.y + call.h + 1.0 * u, call.w, 4.0 * u), sim.next_in / biome_core::sim::WAVE_GAP, rgb(0xffe08a), alpha(SOIL, 0.6));
            }
            if app.ui.button(&app.g, &mut app.au, 20, r, color, &label, None, on && !busy) {
                self.sim.call();
            }
        }
        if let Some(foe) = tapped_foe {
            self.menu = Menu::Foe(foe);
            app.au.play(Sfx::Click, 0.6);
        }
    }

    /// One card per pack member: tap the picture to choose who the next
    /// order goes to, tap the round button to use the ability.
    fn cards(&mut self, app: &mut App, area: Rect, busy: bool) {
        let u = app.u;
        let pack: Vec<usize> = (0..self.sim.units.len()).filter(|&i| !self.sim.units[i].guest).collect();
        let n = pack.len().max(1) as f32;
        let gap = 5.0 * u;
        let w = ((area.w - 12.0 * u - gap * (n - 1.0)) / n).min(190.0 * u);
        let total = w * n + gap * (n - 1.0);
        let y = area.y + STRIP_H * u + 4.0 * u;
        let h = (CARD_H - 10.0) * u;
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
            g.nine(spr::BTN, r, 32.0, 16.0 * u, if chosen { mix(base, WHITE, 0.25) } else { mix(base, rgb(0x2a1a0c), 0.45) });
            if chosen {
                g.nine(spr::CHIP, Rect::new(r.x - 2.0 * u, r.y - 2.0 * u, r.w + 4.0 * u, r.h + 2.0 * u), 24.0, 16.0 * u, alpha(WHITE, 0.0));
            }
            // the picture
            let pic = h * 0.82;
            let px = r.x + pic * 0.52;
            let py = r.y + h * 0.47;
            let face = Rect::new(r.x, r.y, w - h * 0.78, h);
            g.spr_fit(cr_spr(who), px, py, pic, 0.0, if alive { WHITE } else { rgb(0x7a7a7a) });
            if !alive {
                g.label(&format!("{:.0}", ko.ceil()), px, py, 22.0 * u, 0.5, WHITE, Some((alpha(SOIL, 0.9), 2.0 * u)), 0.0);
                cooldown_ring(px, py, pic * 0.46, 3.0 * u, ko / revive.max(0.1), alpha(WHITE, 0.8));
            }
            // name and health
            let tx = r.x + pic * 1.02;
            let room = w - pic * 1.02 - h * 0.74;
            if room > 30.0 * u {
                g.label(who.def().name, tx, r.y + h * 0.3, 13.0 * u, 0.0, WHITE, Some((alpha(SOIL, 0.75), 1.3 * u)), 0.0);
                if perched {
                    g.label("up high", tx, r.y + h * 0.58, 10.0 * u, 0.0, alpha(WHITE, 0.75), None, 0.0);
                } else {
                    bar(g, Rect::new(tx, r.y + h * 0.5, room - 4.0 * u, 8.0 * u), hp, if hp > 0.35 { rgb(0x7ae04a) } else { rgb(0xff6a4a) }, alpha(SOIL, 0.7));
                }
            } else if !perched {
                bar(g, Rect::new(r.x + 6.0 * u, r.y + h - 13.0 * u, pic * 0.9, 7.0 * u), hp, if hp > 0.35 { rgb(0x7ae04a) } else { rgb(0xff6a4a) }, alpha(SOIL, 0.7));
            }
            if app.ui.tapped(face) && !busy {
                self.sel = i;
                app.au.play(Sfx::Click, 0.6);
            }
            app.mark(&format!("unit{slot}"), face);

            // the ability
            let bs = h * 0.74;
            let br = Rect::new(r.x + w - bs - 6.0 * u, r.y + (h - bs) / 2.0 - 1.0 * u, bs, bs);
            let shake = if self.nope > 0.0 && chosen { (self.t * 60.0).sin() * 3.0 * u * self.nope } else { 0.0 };
            let br = Rect::new(br.x + shake, br.y, br.w, br.h);
            if ready {
                let p = 0.5 + 0.5 * (self.t * 4.0 + slot as f32).sin();
                app.g.spr(spr::GLOW, br.x + bs / 2.0, br.y + bs / 2.0, bs * (1.5 + 0.2 * p), bs * (1.5 + 0.2 * p), 0.0, alpha(rgb(0xfff07a), 0.45 + 0.25 * p));
            }
            app.mark(&format!("cast{slot}"), br);
            if app.ui.button(&app.g, &mut app.au, 30 + slot as u32, br, if ready { rgb(0xf2b020) } else { rgb(0x6a6258) }, "", Some(ability_icon(who)), alive && !busy) {
                self.sel = i;
                if !self.sim.cast(i) {
                    self.nope = 1.0;
                    app.au.play(Sfx::Error, 0.6);
                }
            }
            if alive && cooling > 0.0 {
                cooldown_ring(br.x + bs / 2.0, br.y + bs / 2.0, bs * 0.5, 3.5 * u, cooling, rgb(0xfff07a));
            }
        }
    }

    fn build_menu(&mut self, app: &mut App, area: Rect, spot: usize, busy: bool) {
        let u = app.u;
        let towers = BIOMES[self.sim.def.biome].towers;
        let n = towers.len() as f32;
        let gap = 5.0 * u;
        let w = (area.w - 12.0 * u - gap * (n - 1.0)) / n;
        let y = area.y + 30.0 * u;
        let h = area.h - app.bot - 36.0 * u;
        // the card under the finger explains itself in the title row
        let mut title = "Build what?".to_string();
        let mut picked = None;
        for (i, &k) in towers.iter().enumerate() {
            let r = Rect::new(area.x + 6.0 * u + i as f32 * (w + gap), y, w, h);
            let cost = self.sim.price(k, 1);
            let ok = self.sim.seeds >= cost;
            let (held, clicked) = app.ui.press(40 + i as u32, r, !busy);
            let over = r.contains(app.ui.ptr.pos) && app.ui.ptr.down;
            if held || over {
                title = format!("{}: {}", k.def().name, k.def().tag);
            }
            let g = &app.g;
            let dy = if held { 2.0 * u } else { 0.0 };
            let body = Rect::new(r.x, r.y + dy, r.w, r.h);
            g.nine(spr::BTN, body, 32.0, 14.0 * u, if !ok { rgb(0x7a7468) } else if held { rgb(0xd8c890) } else { rgb(0xf4e6b8) });
            g.spr_fit(tower_spr(k), body.x + w / 2.0, body.y + h * 0.36, (h * 0.56).min(w * 0.86), 0.0, if ok { WHITE } else { rgb(0x9a9a9a) });
            let name = short_name(k);
            g.label(name, body.x + w / 2.0, body.y + h * 0.68, (10.5 * u).min(w * 1.55 / name.len().max(6) as f32), 0.5, BARK, None, 0.0);
            price(g, body.x + w / 2.0, body.y + h * 0.85, 12.0 * u, cost, ok);
            app.mark(&format!("build{i}"), r);
            if clicked {
                if ok {
                    picked = Some(k);
                } else {
                    app.au.play(Sfx::Error, 0.6);
                    self.seed_pop = 1.0;
                }
            }
        }
        let g = &app.g;
        g.label(&title, area.x + 10.0 * u, area.y + 18.0 * u, (12.5 * u).min(area.w * 1.5 / title.len().max(20) as f32), 0.0, CREAM, None, 0.0);
        let close = Rect::new(area.x + area.w - 34.0 * u, area.y + 5.0 * u, 28.0 * u, 26.0 * u);
        if app.ui.button(&app.g, &mut app.au, 49, close, rgb(0xc0503a), "", Some(spr::IC_X), !busy) {
            self.menu = Menu::None;
        }
        if let Some(k) = picked {
            if self.sim.build(spot, k) {
                self.menu = Menu::None;
            }
        }
    }

    fn tower_menu(&mut self, app: &mut App, area: Rect, spot: usize, busy: bool) {
        let u = app.u;
        let Some(tw) = self.sim.towers[spot].clone() else {
            self.menu = Menu::None;
            return;
        };
        let d = tw.kind.def();
        let lv = tw.level as usize;
        let g = &app.g;
        let x = area.x + 8.0 * u;
        let y = area.y + 10.0 * u;
        g.spr_fit(tower_spr(tw.kind), x + 30.0 * u, y + 30.0 * u, 58.0 * u, 0.0, WHITE);
        for n in 0..3 {
            g.spr_fit(if n < tw.level { spr::IC_STAR } else { spr::IC_STAR_GRAY }, x + 12.0 * u + n as f32 * 18.0 * u, y + 68.0 * u, 17.0 * u, 0.0, WHITE);
        }
        let tx = x + 66.0 * u;
        g.label(d.name, tx, y + 8.0 * u, 15.0 * u, 0.0, CREAM, None, 0.0);
        // what it does, in numbers
        let stats = match d.mode {
            Mode::Field { slow } => format!("Slows by {:.0}%   Reach {:.1}", (slow + LV_SLOW * (lv as f32 - 1.0)) * 100.0, self.sim.reach(spot)),
            _ => format!("Hits {:.0}   Reach {:.1}   {:.1} a second", d.dmg * LV_DMG[lv - 1], d.range * LV_RANGE[lv - 1], d.rate * LV_RATE[lv - 1]),
        };
        g.label(&stats, tx, y + 26.0 * u, 11.0 * u, 0.0, rgb(0xffe08a), None, 0.0);
        let perk = if lv == 3 { d.perk_text.to_string() } else { format!("At level 3: {}", d.perk_text) };
        let lines = g.wrap(&perk, 10.5 * u, area.w - 66.0 * u - 50.0 * u);
        for (i, line) in lines.iter().take(2).enumerate() {
            g.label(line, tx, y + 42.0 * u + i as f32 * 12.5 * u, 10.5 * u, 0.0, alpha(CREAM, if lv == 3 { 1.0 } else { 0.7 }), None, 0.0);
        }
        let by = y + 70.0 * u;
        let bh = 36.0 * u;
        let up = Rect::new(tx, by, 132.0 * u, bh);
        let sell = Rect::new(tx + 138.0 * u, by, 100.0 * u, bh);
        app.mark("upgrade", up);
        app.mark("sell", sell);
        match self.sim.upgrade_price(spot) {
            Some(cost) => {
                let ok = self.sim.seeds >= cost;
                if app.ui.button(&app.g, &mut app.au, 50, up, if ok { GREEN } else { GREY }, "", None, !busy) {
                    if ok {
                        self.sim.upgrade(spot);
                    } else {
                        app.au.play(Sfx::Error, 0.6);
                        self.seed_pop = 1.0;
                    }
                }
                let g = &app.g;
                g.spr_fit(spr::IC_UP, up.x + 20.0 * u, up.y + bh * 0.45, 20.0 * u, 0.0, WHITE);
                price(g, up.x + up.w * 0.62, up.y + bh * 0.45, 15.0 * u, cost, ok);
            }
            None => {
                app.g.nine(spr::BTN, up, 32.0, 14.0 * u, rgb(0x6a6258));
                app.g.label("TOP LEVEL", up.x + up.w / 2.0, up.y + bh * 0.45, 13.0 * u, 0.5, rgb(0xffe08a), None, 0.0);
            }
        }
        let back = self.sim.sell_value(spot);
        if app.ui.button(&app.g, &mut app.au, 51, sell, ORANGE, "", None, !busy) {
            self.sim.sell(spot);
            self.menu = Menu::None;
        }
        let g = &app.g;
        g.label("Sell", sell.x + 12.0 * u, sell.y + bh * 0.45, 13.0 * u, 0.0, WHITE, Some((alpha(SOIL, 0.6), 1.2 * u)), 0.0);
        price(g, sell.x + sell.w * 0.72, sell.y + bh * 0.45, 13.0 * u, back, true);
        let close = Rect::new(area.x + area.w - 34.0 * u, area.y + 6.0 * u, 28.0 * u, 26.0 * u);
        if app.ui.button(&app.g, &mut app.au, 49, close, rgb(0xc0503a), "", Some(spr::IC_X), !busy) {
            self.menu = Menu::None;
        }
    }

    fn foe_card(&mut self, app: &mut App, area: Rect, foe: Foe) {
        let u = app.u;
        let d = foe.def();
        let g = &app.g;
        let y = area.y + 14.0 * u;
        g.spr_fit(foe_spr(foe), area.x + 46.0 * u, y + 44.0 * u, 76.0 * u, 0.0, WHITE);
        let tx = area.x + 94.0 * u;
        g.label(d.name, tx, y + 8.0 * u, 17.0 * u, 0.0, CREAM, None, 0.0);
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
        g.label(&tags.join("  -  "), tx, y + 27.0 * u, 11.5 * u, 0.0, rgb(0xffe08a), None, 0.0);
        for (i, line) in g.wrap(d.blurb, 12.0 * u, area.w - 104.0 * u).iter().take(3).enumerate() {
            g.label(line, tx, y + 46.0 * u + i as f32 * 14.5 * u, 12.0 * u, 0.0, CREAM, None, 0.0);
        }
        g.label("tap to close", area.x + area.w - 10.0 * u, area.y + area.h - app.bot - 10.0 * u, 10.0 * u, 1.0, alpha(CREAM, 0.6), None, 0.0);
        if app.ui.ptr.released {
            self.menu = Menu::None;
            app.ui.eat();
        }
    }

    // ---- taps on the field --------------------------------------------------------------

    fn field_taps(&mut self, app: &mut App, v: &View) {
        let ptr = &app.ui.ptr;
        if !ptr.released || ptr.travel > 14.0 * app.u {
            return;
        }
        let top = app.top + HUD_H * app.u;
        let bottom = app.h - app.bot - (STRIP_H + CARD_H) * app.u;
        if ptr.pos.y < top || ptr.pos.y > bottom || ptr.press_pos.y < top || ptr.press_pos.y > bottom {
            return;
        }
        let at = v.tile(ptr.pos);
        // a mound?
        let mut best: Option<(usize, f32)> = None;
        for (i, m) in self.sim.map.mounds.iter().enumerate() {
            let d = (m.x - at.x).hypot(m.y - 0.1 - at.y);
            if d < 0.62 && best.is_none_or(|b| d < b.1) {
                best = Some((i, d));
            }
        }
        if let Some((spot, _)) = best {
            self.menu = if self.sim.towers[spot].is_some() { Menu::Tower(spot) } else { Menu::Build(spot) };
            app.au.play(Sfx::Click, 0.7);
            return;
        }
        if self.menu != Menu::None {
            self.menu = Menu::None;
            return;
        }
        // one of the pack?
        for (i, un) in self.sim.units.iter().enumerate() {
            if !un.guest && i != self.sel && (un.pos.x - at.x).hypot(un.pos.y - 0.35 - at.y) < 0.5 {
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
            let drop = ease_back((self.intro / 0.5).min(1.0));
            let y = r.y + 34.0 * u * drop - 6.0 * u;
            let def = self.sim.def;
            g.spr(spr::RIBBON, cx, y, 300.0 * u, 84.0 * u, 0.0, self.pal.accent);
            g.label(def.name, cx, y - 9.0 * u, (22.0 * u).min(300.0 * u * 1.3 / def.name.len().max(8) as f32), 0.5, WHITE, Some((alpha(SOIL, 0.8), 2.0 * u)), 0.0);
            if !self.teach {
                let w = app.pw - 40.0 * u;
                let hh = g.para_height(def.tip, 13.0 * u, w - 24.0 * u);
                let tip = Rect::new(cx - w / 2.0, y + 38.0 * u, w, hh + 16.0 * u);
                chip(g, tip, 0.88 * drop);
                g.para(def.tip, cx, tip.y + 8.0 * u, 13.0 * u, w - 24.0 * u, alpha(WHITE, drop));
            }
        }
        if let Some((text, life)) = &self.banner {
            let a = (life / 0.3).min(1.0);
            let pop = ease_back(((if text.len() > 9 { 3.0 } else { 1.6 } - life) / 0.3).clamp(0.0, 1.0));
            let size = (34.0 * u).min(app.pw * 1.5 / text.len().max(6) as f32);
            g.label(text, cx, r.y + r.h * 0.3, size * pop, 0.5, alpha(rgb(0xfff07a), a), Some((alpha(rgb(0x5a2a00), a), size * 0.08)), -0.03);
        }
    }

    /// The first stage of a new journey explains itself, one thing at a time.
    fn draw_hints(&self, app: &App, v: &View) {
        let u = app.u;
        let g = &app.g;
        let sim = &self.sim;
        let built = sim.towers.iter().flatten().count();
        let hero = sim.units[0].who.def();
        let call = vec2(app.x0 + app.pw - 63.0 * u, app.h - app.bot - (STRIP_H + CARD_H) * u + 4.0 * u);
        let cast = vec2(app.x0 + app.pw / 2.0 + 60.0 * u, app.h - app.bot - CARD_H * u + 8.0 * u);
        let (text, target): (String, Option<Vec2>) = if let Menu::Build(_) = self.menu {
            ("Pick something to build. Thistles are a good start.".to_string(), None)
        } else if built == 0 {
            ("Tap a mound to build on it.".to_string(), Some(v.at(sim.map.mounds[2]) - vec2(0.0, v.cell * 0.5)))
        } else if !self.moved && sim.phase == Phase::Ready {
            (format!("Tap the trail to send {} there. {}", hero.name, if hero.perched { "She strikes from above." } else { "Invaders stop to fight him." }), Some(v.at(sim.map.routes[0].at(9.0).0)))
        } else if sim.phase == Phase::Ready {
            ("Ready? Tap START.".to_string(), Some(call))
        } else if !self.cast_once && sim.units[0].cd <= 0.0 && sim.worth_casting(0) {
            (format!("Now! Tap the gold button: {}.", hero.ability_name), Some(cast))
        } else if sim.wave <= 2 && built < 3 && sim.seeds >= 50 {
            ("You have seeds to spare. Build on another mound.".to_string(), None)
        } else {
            return;
        };
        let r = v.rect();
        let w = app.pw - 36.0 * u;
        let hh = g.para_height(&text, 14.0 * u, w - 26.0 * u);
        let y = r.y + if sim.phase == Phase::Ready { 78.0 * u } else { 14.0 * u };
        let boxr = Rect::new(app.x0 + (app.pw - w) / 2.0, y, w, hh + 18.0 * u);
        g.nine(spr::BUBBLE, boxr, 30.0, 16.0 * u, WHITE);
        g.para(&text, boxr.x + w / 2.0, boxr.y + 9.0 * u, 14.0 * u, w - 26.0 * u, BARK);
        if let Some(at) = target {
            let bounce = (self.t * 5.0).sin().abs() * 10.0 * u;
            g.spr_fit(spr::IC_NEXT, at.x, at.y - 26.0 * u - bounce, 34.0 * u, PI / 2.0, rgb(0xfff07a));
        }
    }

    // ---- cards over everything ------------------------------------------------------------

    fn pause_card(&mut self, app: &mut App) -> Option<Nav> {
        let u = app.u;
        let cx = app.x0 + app.pw / 2.0;
        let card = Rect::new(cx - 150.0 * u, app.h * 0.5 - 170.0 * u, 300.0 * u, 340.0 * u);
        let outside = dim(app, 0.6, card);
        panel(&app.g, card, u, WHITE);
        app.g.title("PAUSED", cx, card.y + 36.0 * u, 30.0 * u, rgb(0xfff07a), BARK);
        let p = app.save.p();
        app.g.text_c(&format!("{}  -  {}", self.sim.def.name, p.difficulty.name()), cx, card.y + 66.0 * u, 13.0 * u, BARK);
        let mut nav = None;
        let bw = 220.0 * u;
        let bh = 52.0 * u;
        let bx = cx - bw / 2.0;
        app.mark("resume", Rect::new(bx, card.y + 86.0 * u, bw, bh));
        if app.ui.button(&app.g, &mut app.au, 60, Rect::new(bx, card.y + 86.0 * u, bw, bh), GREEN, "RESUME", Some(spr::IC_PLAY), true) || outside {
            self.paused = false;
            app.ui.eat();
        }
        if app.ui.button(&app.g, &mut app.au, 61, Rect::new(bx, card.y + 146.0 * u, bw, bh), BLUE, "START AGAIN", Some(spr::IC_RETRY), true) {
            nav = Some(Nav::Play(self.level));
        }
        app.mark("tomap", Rect::new(bx, card.y + 206.0 * u, bw, bh));
        if app.ui.button(&app.g, &mut app.au, 62, Rect::new(bx, card.y + 206.0 * u, bw, bh), ORANGE, "BACK TO THE MAP", Some(spr::IC_HOME), true) {
            nav = Some(Nav::Map);
        }
        sound_toggles(app, cx, card.y + 272.0 * u, 63);
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
        let rise = ease_back((t / 0.45).min(1.0));
        let card = Rect::new(cx - 158.0 * u, app.h * 0.5 - 150.0 * u + (1.0 - rise) * 60.0 * u, 316.0 * u, if o.won { 320.0 } else { 300.0 } * u);
        app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(rgb(0x0c1408), 0.55 * (t / 0.3).min(1.0)));
        panel(&app.g, card, u, alpha(WHITE, rise.min(1.0)));
        let g = &app.g;
        g.spr(spr::RIBBON, cx, card.y + 8.0 * u, 320.0 * u, 90.0 * u, 0.0, if o.won { rgb(0x3fa84a) } else { rgb(0x8a6a5a) });
        g.label(o.word, cx, card.y - 1.0 * u, (25.0 * u).min(300.0 * u * 1.35 / o.word.len().max(6) as f32), 0.5, WHITE, Some((alpha(SOIL, 0.8), 2.2 * u)), 0.0);
        let mut nav = None;
        if o.won {
            // the stars arrive one at a time
            for i in 0..3u8 {
                let at = 0.5 + i as f32 * 0.35;
                let x = cx + (i as f32 - 1.0) * 76.0 * u;
                let y = card.y + 92.0 * u - if i == 1 { 10.0 * u } else { 0.0 };
                g.spr_fit(spr::IC_STAR_GRAY, x, y, 62.0 * u, (i as f32 - 1.0) * 0.18, WHITE);
                if i < o.stars && t > at {
                    let pop = ease_back(((t - at) / 0.3).min(1.0));
                    g.spr_fit(spr::IC_STAR, x, y, 66.0 * u * pop, (i as f32 - 1.0) * 0.18, WHITE);
                    if o.shown <= i {
                        o.shown = i + 1;
                        app.au.play(Sfx::Star(i), 0.8);
                        self.fx.sparkle(vec2(x, y), 30.0 * u, 10, 10.0 * u, rgb(0xfff07a));
                    }
                }
            }
            let g = &app.g;
            g.text_c(&format!("{} of {} hearts kept. {} invaders shooed.", self.sim.hearts, self.sim.max_hearts, self.sim.shooed), cx, card.y + 146.0 * u, 12.5 * u, BARK);
            if o.first {
                let r = Rect::new(cx - 130.0 * u, card.y + 162.0 * u, 260.0 * u, 54.0 * u);
                g.nine(spr::CHIP, r, 24.0, 16.0 * u, alpha(rgb(0x2a5a1a), 1.0));
                g.label("+1 skill point", cx, r.y + 17.0 * u, 15.0 * u, 0.5, rgb(0xfff07a), None, 0.0);
                g.label("+1 bond point for the pack", cx, r.y + 37.0 * u, 13.0 * u, 0.5, rgb(0xb8f0ff), None, 0.0);
            } else {
                g.text_c("Already held once: no new points.", cx, card.y + 188.0 * u, 12.5 * u, alpha(BARK, 0.7));
            }
            let ready = t > 1.3;
            let bw = 200.0 * u;
            app.mark("continue", Rect::new(cx - bw / 2.0, card.y + 228.0 * u, bw, 56.0 * u));
            if app.ui.button(&app.g, &mut app.au, 70, Rect::new(cx - bw / 2.0, card.y + 228.0 * u, bw, 56.0 * u), GREEN, "ON WE GO", Some(spr::IC_NEXT), ready) || (self.auto > 1 && t > 2.5) {
                nav = Some(Nav::Map);
            }
            let retry = Rect::new(cx + bw / 2.0 + 8.0 * u, card.y + 234.0 * u, 44.0 * u, 44.0 * u);
            if app.ui.button(&app.g, &mut app.au, 71, retry, BLUE, "", Some(spr::IC_RETRY), ready) {
                nav = Some(Nav::Play(level));
            }
        } else {
            let hero = self.sim.units[0].who;
            g.spr(cr_spr(hero), cx, card.y + 96.0 * u, 96.0 * u, 96.0 * u, 0.25, rgb(0xb8b8c0));
            g.text_c(&format!("They got in on wave {} of {}.", self.sim.wave, self.sim.waves.len()), cx, card.y + 152.0 * u, 13.0 * u, BARK);
            let bw = 230.0 * u;
            let bx = cx - bw / 2.0;
            app.mark("again", Rect::new(bx, card.y + 170.0 * u, bw, 50.0 * u));
            if app.ui.button(&app.g, &mut app.au, 72, Rect::new(bx, card.y + 170.0 * u, bw, 50.0 * u), GREEN, "TRY AGAIN", Some(spr::IC_RETRY), t > 0.8) || (self.auto > 1 && t > 2.5) {
                nav = Some(Nav::Play(level));
            }
            // one tap to an easier setting
            let easier = app.save.p().difficulty.easier();
            let half = (bw - 8.0 * u) / 2.0;
            if let Some(e) = easier {
                app.mark("easier", Rect::new(bx, card.y + 228.0 * u, half, 46.0 * u));
                if app.ui.button(&app.g, &mut app.au, 73, Rect::new(bx, card.y + 228.0 * u, half, 46.0 * u), BLUE, &format!("ON {}", e.name().to_uppercase()), None, t > 0.8) {
                    app.save.p_mut().difficulty = e;
                    app.save.store();
                    nav = Some(Nav::Play(level));
                }
            }
            let mr = if easier.is_some() { Rect::new(bx + half + 8.0 * u, card.y + 228.0 * u, half, 46.0 * u) } else { Rect::new(bx, card.y + 228.0 * u, bw, 46.0 * u) };
            if app.ui.button(&app.g, &mut app.au, 74, mr, ORANGE, "MAP", Some(spr::IC_HOME), t > 0.8) {
                nav = Some(Nav::Map);
            }
        }
        self.fx.draw(&app.g);
        if nav.is_some() {
            app.au.set_music_gain(1.0);
        }
        nav
    }
}

/// A dashed-looking circle of reach on the ground.
fn ring(c: Vec2, r: f32, cell: f32, color: Color) {
    draw_circle(c.x, c.y, r, alpha(color, 0.08 * color.a));
    draw_circle_lines(c.x, c.y, r, cell * 0.045, alpha(color, 0.75 * color.a));
}

/// A name short enough for a build card.
fn short_name(k: Tw) -> &'static str {
    match k {
        Tw::Thistle => "Thistle",
        Tw::Woodpecker => "Woodpecker",
        Tw::Cattail => "Cattails",
        Tw::Icicle => "Icicles",
        Tw::Snowball => "Snowballs",
        Tw::Eyrie => "Eyrie",
        Tw::Cactus => "Cactus",
        other => other.def().name,
    }
}
