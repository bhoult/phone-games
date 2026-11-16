//! The title screen, and choosing a creature for a new journey.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::fx::Fx;
use crate::gfx::*;
use crate::ui::*;
use crate::widgets::*;
use crate::{App, Nav};
use biome_core::creature::{Cr, HEROES};
use biome_core::level::Difficulty;
use biome_core::{Profile, LEVEL_COUNT};
use macroquad::prelude::{vec2, Rect};

/// The backdrop both screens share: dawn over the biomes, meadow below.
fn backdrop(app: &App, t: f32) -> f32 {
    let g = &app.g;
    let scene_h = app.h * 0.64;
    g.vgrad(Rect::new(0.0, 0.0, app.w, app.h), rgb(0x9bd95f), rgb(0x4f9a3a));
    g.scene(scene::TITLE, Rect::new(app.x0, 0.0, app.pw, scene_h), WHITE);
    // the meadow carries on below the picture
    g.vgrad(Rect::new(app.x0, scene_h - 2.0, app.pw, app.h - scene_h + 2.0), rgb(0x7fc64f), rgb(0x3f8a34));
    // a slow warm glow where the sun is
    let pulse = 0.5 + 0.5 * (t * 0.6).sin();
    g.lights(|| g.spr(spr::RAYS, app.x0 + app.pw * 0.42, scene_h * 0.42, app.pw * 1.5, app.pw * 1.5, t * 0.05, alpha(rgb(0xfff0a0), if g.has_light() { 0.1 + 0.05 * pulse } else { 0.0 })));
    if app.x0 > 1.0 {
        g.rect(Rect::new(0.0, 0.0, app.x0, app.h), rgb(0x16240f));
        g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), rgb(0x16240f));
    }
    scene_h
}

/// A creature standing on the grass, breathing.
fn stand(g: &Gfx, who: Cr, x: f32, ground: f32, size: f32, t: f32, flip: bool) {
    g.spr(spr::SHADOW, x, ground, size * 0.8, size * 0.22, 0.0, alpha(WHITE, 0.6));
    let breathe = 1.0 + 0.025 * (t * 2.4 + x * 0.01).sin();
    g.spr_ex(cr_spr(who), x, ground - size * 0.46 * breathe, size, size * breathe, 0.0, WHITE, flip);
}

pub struct Title {
    t: f32,
    fx: Fx,
    confirm_new: bool,
    hop: f32,
}

impl Title {
    pub fn new(app: &mut App) -> Title {
        app.au.track(Track::Trail);
        app.log("SCREEN title");
        Title { t: 0.0, fx: Fx::new(app.rng.next_u64()), confirm_new: false, hop: 0.0 }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        self.hop = (self.hop - dt * 2.5).max(0.0);
        self.fx.update(dt);
        let cx = app.x0 + app.pw / 2.0;
        backdrop(app, self.t);
        let g = &app.g;

        // the name, a letter at a time
        let logo_y = app.top + (app.h * 0.13).max(80.0 * u);
        for (row, (word, size, color, edge)) in [("BIOME", 76.0, rgb(0xfff2a8), rgb(0x3a5a12)), ("DEFENSE", 56.0, rgb(0xffffff), rgb(0x2a4a12))].into_iter().enumerate() {
            let size = size * u;
            let total = g.text_w(word, size);
            let mut x = cx - total / 2.0;
            for (i, ch) in word.chars().enumerate() {
                let s = ch.to_string();
                let w = g.text_w(&s, size);
                let enter = ease_back(((self.t - 0.15 - row as f32 * 0.25 - i as f32 * 0.05) / 0.4).clamp(0.0, 1.0));
                let bob = (self.t * 2.2 - i as f32 * 0.55 - row as f32).sin() * 3.0 * u;
                g.label(&s, x + w / 2.0, logo_y + row as f32 * 62.0 * u + bob, size * enter, 0.5, color, Some((edge, size * 0.075)), (self.t * 1.6 + i as f32).sin() * 0.03);
                x += w;
            }
        }
        // a leaf either side of the name
        for side in [-1.0f32, 1.0] {
            g.spr_fit(spr::P_LEAF, cx + side * 150.0 * u, logo_y - 8.0 * u + (self.t * 2.0 + side).sin() * 4.0 * u, 30.0 * u, side * 0.6 + (self.t * 1.3).sin() * 0.2, rgb(0x7ad04a));
        }

        // the pack, or the three who might set out
        let ground = (app.h * 0.7).min(app.h - app.bot - 200.0 * u);
        let journey = app.save.profile.clone();
        let cast: Vec<Cr> = match &journey {
            Some(p) => p.pack().iter().map(|c| c.0).collect(),
            None => vec![HEROES[1], HEROES[0], HEROES[2]],
        };
        let n = cast.len() as f32;
        let mut tapped = None;
        for (i, &who) in cast.iter().enumerate() {
            let lead = journey.is_some() && i == 0 || journey.is_none() && i == 1;
            let size = if lead { 150.0 } else { 112.0 } * u;
            let slot = if journey.is_some() { [0.0, -1.0, 1.0][i.min(2)] } else { i as f32 - 1.0 };
            let x = cx + slot * (if n > 2.0 { 116.0 } else { 100.0 }) * u;
            let y = ground + if lead { 10.0 * u } else { 0.0 } - if lead { self.hop.powi(2) * 0.0 + (self.hop * std::f32::consts::PI).sin() * 26.0 * u } else { 0.0 };
            stand(g, who, x, y, size, self.t + i as f32, slot > 0.0);
            let hit = Rect::new(x - size / 2.0, y - size, size, size);
            if app.ui.tapped(hit) {
                tapped = Some((who, vec2(x, y - size * 0.6)));
            }
        }
        if let Some((_, at)) = tapped {
            self.hop = 1.0;
            self.fx.hearts(at, 3, u);
            app.au.play(Sfx::Blip, 0.9);
        }
        self.fx.draw(&app.g);

        // the way in
        let mut nav = None;
        let by = ground + 34.0 * u;
        let pulse = 1.0 + 0.03 * (self.t * 4.0).sin();
        let bw = 236.0 * u * pulse;
        let label = if journey.is_some() { "CONTINUE" } else { "SET OUT" };
        let play = Rect::new(cx - bw / 2.0, by, bw, 64.0 * u);
        app.mark("play", play);
        if app.ui.button(&app.g, &mut app.au, 1, play, GREEN, label, Some(spr::IC_PLAY), self.t > 0.4 && !self.confirm_new) {
            nav = Some(if journey.is_some() { Nav::Map } else { Nav::Choose });
        }
        let g = &app.g;
        match &journey {
            Some(p) => {
                let done = p.cleared();
                let text = if p.finished() { format!("Journey complete: {} of {} stars", p.total_stars(), LEVEL_COUNT * 3) } else { format!("Stage {} of {}   {} stars", done + 1, LEVEL_COUNT, p.total_stars()) };
                g.label(&text, cx, by + 80.0 * u, 13.0 * u, 0.5, WHITE, Some((alpha(SOIL, 0.7), 1.3 * u)), 0.0);
            }
            None => g.label("Wild creatures. One road. Six homes worth keeping.", cx, by + 80.0 * u, 12.5 * u, 0.5, WHITE, Some((alpha(SOIL, 0.7), 1.3 * u)), 0.0),
        }
        let row = (by + 98.0 * u).min(app.h - app.bot - 58.0 * u);
        sound_toggles(app, cx - 78.0 * u, row, 2);
        if journey.is_some() {
            let fresh = Rect::new(cx + 6.0 * u, row, 150.0 * u, 46.0 * u);
            app.mark("newgame", fresh);
            if app.ui.button(&app.g, &mut app.au, 4, fresh, ORANGE, "NEW JOURNEY", None, !self.confirm_new) {
                self.confirm_new = true;
            }
        }

        if self.confirm_new {
            let card = Rect::new(cx - 150.0 * u, app.h * 0.5 - 100.0 * u, 300.0 * u, 200.0 * u);
            let out = dim(app, 0.6, card);
            panel(&app.g, card, u, WHITE);
            app.g.title("Start over?", cx, card.y + 34.0 * u, 26.0 * u, rgb(0xfff07a), BARK);
            app.g.para("This forgets the journey so far: stars, skills and friends.", cx, card.y + 58.0 * u, 14.0 * u, 250.0 * u, BARK);
            if app.ui.button(&app.g, &mut app.au, 5, Rect::new(cx - 136.0 * u, card.y + 128.0 * u, 130.0 * u, 50.0 * u), rgb(0xc0503a), "FORGET IT", None, true) {
                app.save.profile = None;
                app.save.store();
                nav = Some(Nav::Choose);
            }
            if app.ui.button(&app.g, &mut app.au, 6, Rect::new(cx + 6.0 * u, card.y + 128.0 * u, 130.0 * u, 50.0 * u), GREEN, "KEEP IT", None, true) || out {
                self.confirm_new = false;
                app.ui.eat();
            }
        }
        nav
    }
}

pub struct Choose {
    t: f32,
    pick: usize,
    since: f32,
}

impl Choose {
    pub fn new(app: &mut App) -> Choose {
        app.au.track(Track::Trail);
        app.log("SCREEN choose");
        Choose { t: 0.0, pick: 0, since: 0.0 }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        self.since += dt;
        let cx = app.x0 + app.pw / 2.0;
        backdrop(app, self.t);
        app.g.rect(Rect::new(app.x0, 0.0, app.pw, app.h), alpha(rgb(0x10200c), 0.45));
        let g = &app.g;
        g.title("Who are you?", cx, app.top + 44.0 * u, 34.0 * u, rgb(0xfff2a8), rgb(0x2a4a12));

        // three to choose from: the chosen one steps forward
        let row = app.top + (app.h * 0.31).max(190.0 * u);
        let mut clicked = None;
        for (i, &who) in HEROES.iter().enumerate() {
            let chosen = i == self.pick;
            let x = cx + (i as f32 - 1.0) * 122.0 * u;
            let grow = if chosen { ease_back((self.since / 0.3).min(1.0)) } else { 0.0 };
            let size = (96.0 + 44.0 * grow) * u;
            if chosen {
                g.spr(spr::GLOW, x, row - size * 0.45, size * 1.9, size * 1.9, 0.0, alpha(cr_color(who), 0.7));
                g.lights(|| g.spr(spr::RAYS, x, row - size * 0.45, size * 2.2, size * 2.2, self.t * 0.3, alpha(WHITE, if g.has_light() { 0.22 } else { 0.0 })));
            }
            stand(g, who, x, row + if chosen { 8.0 * u } else { 0.0 }, size, self.t + i as f32, false);
            g.label(who.def().name, x, row + 26.0 * u, if chosen { 20.0 } else { 15.0 } * u, 0.5, if chosen { WHITE } else { alpha(WHITE, 0.7) }, Some((alpha(SOIL, 0.8), 1.6 * u)), 0.0);
            let hit = Rect::new(x - 58.0 * u, row - 150.0 * u, 116.0 * u, 190.0 * u);
            app.mark(&format!("hero{i}"), hit);
            if app.ui.tapped(hit) && !chosen {
                clicked = Some(i);
            }
        }
        if let Some(i) = clicked {
            self.pick = i;
            self.since = 0.0;
            app.au.play(Sfx::Click, 0.8);
        }

        // who they are
        let who = HEROES[self.pick];
        let d = who.def();
        let card = Rect::new(app.x0 + 14.0 * u, row + 46.0 * u, app.pw - 28.0 * u, 232.0 * u);
        panel(g, card, u, WHITE);
        g.label(&format!("{} the {}", d.name, d.species), cx, card.y + 26.0 * u, 22.0 * u, 0.5, cr_color(who), Some((BARK, 1.8 * u)), 0.0);
        g.label(d.role, cx, card.y + 48.0 * u, 13.0 * u, 0.5, alpha(BARK, 0.75), None, 0.0);
        let used = g.para(d.blurb, cx, card.y + 60.0 * u, 14.0 * u, card.w - 40.0 * u, BARK);
        let ay = card.y + 68.0 * u + used;
        let chipr = Rect::new(card.x + 14.0 * u, ay, card.w - 28.0 * u, card.y + card.h - ay - 14.0 * u);
        g.nine(spr::CHIP, chipr, 24.0, 14.0 * u, WHITE);
        g.spr_fit(ability_icon(who), chipr.x + 30.0 * u, chipr.y + chipr.h / 2.0, 40.0 * u, 0.0, rgb(0xfff07a));
        g.label(d.ability_name, chipr.x + 58.0 * u, chipr.y + 16.0 * u, 15.0 * u, 0.0, rgb(0xfff07a), None, 0.0);
        let text = d.ability_text;
        for (i, line) in g.wrap(text, 12.0 * u, chipr.w - 70.0 * u).iter().take(3).enumerate() {
            g.label(line, chipr.x + 58.0 * u, chipr.y + 34.0 * u + i as f32 * 14.0 * u, 12.0 * u, 0.0, WHITE, None, 0.0);
        }

        let mut nav = None;
        let by = (card.y + card.h + 16.0 * u).min(app.h - app.bot - 74.0 * u);
        let go = Rect::new(cx - 120.0 * u, by, 240.0 * u, 62.0 * u);
        app.mark("begin", go);
        if app.ui.button(&app.g, &mut app.au, 9, go, GREEN, &format!("BE {}", d.name.to_uppercase()), Some(spr::IC_PLAY), self.t > 0.3) {
            let mut p = Profile::new(who);
            p.difficulty = Difficulty::Normal;
            app.save.profile = Some(p);
            app.save.store();
            app.au.play(Sfx::Join, 0.8);
            nav = Some(Nav::Map);
        }
        let back = Rect::new(app.x0 + 10.0 * u, app.top + 22.0 * u, 44.0 * u, 44.0 * u);
        if app.ui.button(&app.g, &mut app.au, 8, back, BLUE, "", Some(spr::IC_BACK), true) {
            nav = Some(Nav::Title);
        }
        nav
    }
}
