//! The title screen, and choosing a creature for a new journey.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::gfx::*;
use crate::ui::*;
use crate::widgets::*;
use crate::{App, Nav};
use biome_core::creature::HEROES;
use biome_core::level::Difficulty;
use biome_core::{Profile, LEVEL_COUNT};
use macroquad::prelude::Rect;

/// Black bars either side of the play column on anything wider than a phone.
pub fn letterbox(app: &App) {
    if app.x0 > 1.0 {
        app.g.rect(Rect::new(0.0, 0.0, app.x0, app.h), SHADE);
        app.g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), SHADE);
    }
}

pub struct Title {
    t: f32,
    confirm_new: bool,
}

impl Title {
    pub fn new(app: &mut App) -> Title {
        app.au.track(Track::Trail);
        app.log("SCREEN title");
        Title { t: 0.0, confirm_new: false }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        let cx = app.x0 + app.pw / 2.0;
        let g = &app.g;
        let col = Rect::new(app.x0, 0.0, app.pw, app.h);

        // the picture, drifting very slowly, darker at the top and bottom
        g.rect(Rect::new(0.0, 0.0, app.w, app.h), SHADE);
        let drift = (self.t * 0.05).sin() * 0.5 + 0.5;
        g.scene(scene::TITLE, Rect::new(col.x - 10.0 * u, -10.0 * u * drift, col.w + 20.0 * u, col.h + 20.0 * u), 0.5, 0.75, WHITE);
        g.fade(Rect::new(col.x, 0.0, col.w, col.h * 0.42), SHADE, 0.72, 0.0);
        g.fade(Rect::new(col.x, col.h * 0.56, col.w, col.h * 0.44), SHADE, 0.0, 0.92);

        // the name
        let a = ((self.t - 0.2) / 0.9).clamp(0.0, 1.0);
        let logo_y = app.top + (app.h * 0.115).max(74.0 * u);
        g.rect(Rect::new(cx - 92.0 * u, logo_y - 36.0 * u, 184.0 * u, 1.5 * u), alpha(INK, 0.5 * a));
        g.spaced("BIOME", cx, logo_y, 50.0 * u, 0.14, alpha(INK, a), Some((SHADE, 2.0 * u)));
        g.spaced("DEFENSE", cx, logo_y + 46.0 * u, 29.0 * u, 0.3, alpha(GOLD, a), Some((SHADE, 1.5 * u)));
        g.rect(Rect::new(cx - 92.0 * u, logo_y + 72.0 * u, 184.0 * u, 1.5 * u), alpha(INK, 0.5 * a));

        // the way in
        let journey = app.save.profile.clone();
        let mut nav = None;
        let by = app.h - app.bot - 196.0 * u;
        let label = if journey.is_some() { "CONTINUE" } else { "SET OUT" };
        let play = Rect::new(cx - 118.0 * u, by, 236.0 * u, 60.0 * u);
        app.mark("play", play);
        if app.ui.button(&app.g, &mut app.au, 1, play, GO, label, Some(spr::IC_PLAY), self.t > 0.4 && !self.confirm_new) {
            nav = Some(if journey.is_some() { Nav::Map } else { Nav::Choose });
        }
        let g = &app.g;
        match &journey {
            Some(p) => {
                let done = p.cleared();
                let who = p.hero.def();
                let text = if p.finished() {
                    format!("{} the {}. The road is held: {} of {} marks.", who.name, who.species, p.total_stars(), LEVEL_COUNT * 3)
                } else {
                    format!("{} the {}. Stage {} of {}.", who.name, who.species, done + 1, LEVEL_COUNT)
                };
                g.write(Face::Text, &text, cx, by + 78.0 * u, 13.5 * u, 0.5, INK, Some((SHADE, 1.2 * u)), 0.0);
            }
            None => g.write(Face::Text, "They took your den. They are not having the next one.", cx, by + 78.0 * u, 13.5 * u, 0.5, INK, Some((SHADE, 1.2 * u)), 0.0),
        }
        let row = by + 104.0 * u;
        sound_toggles(app, if journey.is_some() { cx - 78.0 * u } else { cx }, row, 2);
        if journey.is_some() {
            let fresh = Rect::new(cx + 6.0 * u, row, 150.0 * u, 44.0 * u);
            app.mark("newgame", fresh);
            if app.ui.button(&app.g, &mut app.au, 4, fresh, GREY, "NEW JOURNEY", None, !self.confirm_new) {
                self.confirm_new = true;
            }
        }
        letterbox(app);

        if self.confirm_new {
            let card = Rect::new(cx - 150.0 * u, app.h * 0.5 - 100.0 * u, 300.0 * u, 200.0 * u);
            let out = dim(app, 0.7, card);
            panel(&app.g, card, u, WHITE);
            app.g.head("START OVER?", cx, card.y + 34.0 * u, 21.0 * u, 0.5, INK);
            app.g.para("This forgets the journey so far: marks, skills and everyone who walked with you.", cx, card.y + 56.0 * u, 14.0 * u, 250.0 * u, DIM);
            if app.ui.button(&app.g, &mut app.au, 5, Rect::new(cx - 136.0 * u, card.y + 130.0 * u, 130.0 * u, 50.0 * u), RUST, "FORGET IT", None, true) {
                app.save.profile = None;
                app.save.store();
                nav = Some(Nav::Choose);
            }
            if app.ui.button(&app.g, &mut app.au, 6, Rect::new(cx + 6.0 * u, card.y + 130.0 * u, 130.0 * u, 50.0 * u), STEEL, "KEEP IT", None, true) || out {
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
    /// Who was showing before, while their picture fades out.
    was: usize,
    since: f32,
}

impl Choose {
    pub fn new(app: &mut App) -> Choose {
        app.au.track(Track::Trail);
        app.log("SCREEN choose");
        Choose { t: 0.0, pick: 0, was: 0, since: 9.0 }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        self.since += dt;
        let cx = app.x0 + app.pw / 2.0;
        let g = &app.g;
        let col = Rect::new(app.x0, 0.0, app.pw, app.h);
        let who = HEROES[self.pick];
        let d = who.def();

        // the creature itself fills the top of the screen
        g.rect(Rect::new(0.0, 0.0, app.w, app.h), SHADE);
        let pic = Rect::new(col.x, 0.0, col.w, col.h * 0.62);
        let fade = (self.since / 0.45).min(1.0);
        if fade < 1.0 {
            g.scene(hero_scene(HEROES[self.was]), pic, 0.5, 0.3, WHITE);
        }
        g.scene(hero_scene(who), pic, 0.5, 0.3, alpha(WHITE, fade));
        g.fade(Rect::new(col.x, 0.0, col.w, 110.0 * u + app.top), SHADE, 0.8, 0.0);
        g.fade(Rect::new(col.x, pic.h * 0.5, col.w, pic.h * 0.5 + 1.0), SHADE, 0.0, 1.0);
        g.head("WHO ARE YOU", cx, app.top + 40.0 * u, 15.0 * u, 0.5, alpha(INK, 0.85));

        // three to choose from
        let row = pic.h - 30.0 * u;
        let mut clicked = None;
        for (i, &c) in HEROES.iter().enumerate() {
            let chosen = i == self.pick;
            let x = cx + (i as f32 - 1.0) * 100.0 * u;
            let size = if chosen { 76.0 } else { 58.0 } * u;
            portrait(g, cr_face(c), x, row, size, if chosen { cr_color(c) } else { alpha(INK, 0.35) }, if chosen { WHITE } else { rgb(0x9a9a9a) });
            let hit = Rect::new(x - 48.0 * u, row - 46.0 * u, 96.0 * u, 92.0 * u);
            app.mark(&format!("hero{i}"), hit);
            if app.ui.tapped(hit) && !chosen {
                clicked = Some(i);
            }
        }
        if let Some(i) = clicked {
            self.was = self.pick;
            self.pick = i;
            self.since = 0.0;
            app.au.play(Sfx::Click, 0.8);
        }

        // who they are
        let mut y = row + 62.0 * u;
        g.head(&d.name.to_uppercase(), cx, y, 30.0 * u, 0.5, INK);
        y += 26.0 * u;
        g.head(&format!("THE {}  -  {}", d.species.to_uppercase(), d.role.to_uppercase()), cx, y, 11.0 * u, 0.5, cr_color(who));
        y += 14.0 * u;
        y += g.para(d.blurb, cx, y, 14.5 * u, col.w - 56.0 * u, INK) + 12.0 * u;
        let ah = 74.0 * u;
        let chipr = Rect::new(col.x + 18.0 * u, y, col.w - 36.0 * u, ah);
        chip(g, chipr, 1.0);
        g.spr_fit(ability_icon(who), chipr.x + 32.0 * u, chipr.y + ah / 2.0, 36.0 * u, 0.0, GOLD);
        g.head(&d.ability_name.to_uppercase(), chipr.x + 62.0 * u, chipr.y + 17.0 * u, 12.5 * u, 0.0, GOLD);
        for (i, line) in g.wrap(d.ability_text, 12.5 * u, chipr.w - 76.0 * u).iter().take(3).enumerate() {
            g.write(Face::Text, line, chipr.x + 62.0 * u, chipr.y + 35.0 * u + i as f32 * 15.0 * u, 12.5 * u, 0.0, INK, None, 0.0);
        }

        let mut nav = None;
        let by = (y + ah + 16.0 * u).min(app.h - app.bot - 70.0 * u);
        let go = Rect::new(cx - 130.0 * u, by, 260.0 * u, 58.0 * u);
        app.mark("begin", go);
        if app.ui.button(&app.g, &mut app.au, 9, go, GO, &format!("SET OUT AS {}", d.name.to_uppercase()), None, self.t > 0.3) {
            let mut p = Profile::new(who);
            p.difficulty = Difficulty::Normal;
            app.save.profile = Some(p);
            app.save.store();
            app.au.play(Sfx::Join, 0.8);
            nav = Some(Nav::Map);
        }
        let back = Rect::new(app.x0 + 10.0 * u, app.top + 20.0 * u, 42.0 * u, 40.0 * u);
        if app.ui.button(&app.g, &mut app.au, 8, back, GREY, "", Some(spr::IC_BACK), true) {
            nav = Some(Nav::Title);
        }
        letterbox(app);
        nav
    }
}
