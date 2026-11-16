//! The story, told a line at a time, and what comes after a resident's
//! home has been held: their offer to come along, and (once the pack is
//! full) the choice of who stays behind.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::content::{join_line, lines, Beat, Line, Who};
use crate::gfx::*;
use crate::ui::*;
use crate::title::letterbox;
use crate::widgets::*;
use crate::{App, Nav};
use biome_core::campaign::FOLLOWERS;
use biome_core::creature::{spent, Cr};
use macroquad::prelude::Rect;

enum Stage {
    Talk,
    /// A resident is joining and there is room.
    Welcome(Cr),
    /// A resident is joining and somebody has to stay behind. The index is
    /// who: 0 and 1 are the two walking now, 2 is the newcomer.
    Crowded(Cr, usize),
}

pub struct Story {
    beat: Beat,
    lines: Vec<Line>,
    i: usize,
    /// Characters of the current line shown so far.
    shown: f32,
    t: f32,
    hero: Cr,
    stage: Stage,
    /// The landscape behind the telling.
    backdrop: usize,
    since: f32,
}

impl Story {
    pub fn new(app: &mut App, beat: Beat) -> Story {
        let hero = app.save.p().hero;
        app.au.track(Track::Trail);
        app.log(&format!("SCREEN story {beat:?}"));
        let lines = lines(beat, hero);
        let backdrop = match beat {
            Beat::Intro => scene::STORY_INTRO,
            Beat::Ending => scene::STORY_ENDING,
            _ => palette(beat.biome()).backdrop,
        };
        Story { beat, lines, i: 0, shown: 0.0, t: 0.0, hero, stage: Stage::Talk, backdrop, since: 0.0 }
    }

    /// The talking is over: remember that, and see whether anyone is joining.
    fn finish_talk(&mut self, app: &mut App) -> Option<Nav> {
        let p = app.save.p_mut();
        p.seen |= self.beat.bit();
        let joiner = self.beat.joiner().filter(|&c| p.is_friend(c) && !p.in_party(c));
        app.save.store();
        self.since = 0.0;
        match joiner {
            Some(who) => {
                self.stage = if app.save.p().free_slot().is_some() { Stage::Welcome(who) } else { Stage::Crowded(who, 2) };
                app.au.play(Sfx::Join, 0.8);
                app.log("JOIN");
                None
            }
            None => Some(Nav::Map),
        }
    }

    fn who(&self, w: Who) -> Option<Cr> {
        match w {
            Who::Narrator => None,
            Who::Hero => Some(self.hero),
            Who::Cr(c) => Some(c),
        }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        self.since += dt;
        let g = &app.g;

        // the place, drifting very slowly
        g.rect(Rect::new(0.0, 0.0, app.w, app.h), SHADE);
        let drift = (self.t * 0.04).sin() * 0.5 + 0.5;
        let grow = 16.0 * u;
        g.scene(self.backdrop, Rect::new(app.x0 - grow, -grow * drift, app.pw + 2.0 * grow, app.h + grow), 0.5, 0.4, WHITE);
        g.fade(Rect::new(app.x0, 0.0, app.pw, app.h * 0.3), SHADE, 0.6, 0.0);
        g.fade(Rect::new(app.x0, app.h * 0.45, app.pw, app.h * 0.55), SHADE, 0.0, 0.94);

        let nav = match self.stage {
            Stage::Talk => self.talk(app),
            Stage::Welcome(who) => self.welcome(app, who),
            Stage::Crowded(who, pick) => self.crowded(app, who, pick),
        };
        letterbox(app);
        nav
    }

    fn talk(&mut self, app: &mut App) -> Option<Nav> {
        let u = app.u;
        let cx = app.x0 + app.pw / 2.0;
        let line = &self.lines[self.i];
        let speaker = self.who(line.who);
        let total = line.text.chars().count() as f32;
        let before = self.shown as usize;
        self.shown = (self.shown + app.dt * 52.0).min(total);
        if (self.shown as usize) / 4 != before / 4 && self.shown < total {
            app.au.play(Sfx::Blip, 0.22);
        }
        let g = &app.g;

        // the words, in a plate along the bottom
        let bw = app.pw - 24.0 * u;
        let size = 17.0 * u;
        let text_w = bw - 44.0 * u;
        let wrapped = g.wrap(line.text, size, text_w);
        let bh = (wrapped.len() as f32 * size * 1.3 + 74.0 * u).max(150.0 * u);
        let by = app.h - app.bot - bh - 18.0 * u;
        let boxr = Rect::new(cx - bw / 2.0, by, bw, bh);
        panel(g, boxr, u, WHITE);
        let fade = (self.since / 0.25).min(1.0);
        let top = match speaker {
            Some(c) => {
                // whoever is speaking, and their name
                let ps = 92.0 * u;
                portrait(g, cr_face(c), boxr.x + 22.0 * u + ps / 2.0, boxr.y - ps * 0.22, ps, cr_color(c), alpha(WHITE, fade));
                g.head(&c.def().name.to_uppercase(), boxr.x + 34.0 * u + ps, boxr.y + 20.0 * u, 15.0 * u, 0.0, cr_color(c));
                boxr.y + 52.0 * u
            }
            None => boxr.y + 30.0 * u,
        };
        // wrap the whole line, then reveal: words do not jump between rows as they appear
        let mut left = self.shown as usize;
        for (row, full) in wrapped.iter().enumerate() {
            let n = full.chars().count().min(left);
            let part: String = full.chars().take(n).collect();
            let y = top + row as f32 * size * 1.3;
            if speaker.is_some() {
                g.write(Face::Text, &part, boxr.x + 22.0 * u, y, size, 0.0, INK, None, 0.0);
            } else {
                // the narrator: centred, a little quieter
                let whole = g.text_w_in(Face::Text, full, size);
                g.write(Face::Text, &part, cx - whole / 2.0, y, size, 0.0, rgb(0xcfc8b6), None, 0.0);
            }
            left = left.saturating_sub(n + 1);
            if left == 0 {
                break;
            }
        }
        if self.shown >= total {
            let pulse = 0.5 + 0.5 * (self.t * 3.0).sin();
            g.spr_fit(spr::IC_NEXT, boxr.x + bw - 26.0 * u, boxr.y + bh - 22.0 * u, 18.0 * u, std::f32::consts::FRAC_PI_2, alpha(GOLD, 0.5 + 0.5 * pulse));
        }
        g.label(&format!("{} / {}", self.i + 1, self.lines.len()), boxr.x + 22.0 * u, boxr.y + bh - 16.0 * u, 10.0 * u, 0.0, alpha(DIM, 0.7), None, 0.0);

        let skip = Rect::new(app.x0 + app.pw - 88.0 * u, app.top + 14.0 * u, 76.0 * u, 36.0 * u);
        app.mark("skip", skip);
        app.mark("next", boxr);
        if app.ui.button(&app.g, &mut app.au, 1, skip, GREY, "SKIP", None, true) {
            return self.finish_talk(app);
        }
        if app.ui.ptr.released && !skip.contains(app.ui.ptr.press_pos) {
            if self.shown < total {
                self.shown = total;
            } else if self.i + 1 < self.lines.len() {
                self.i += 1;
                self.shown = 0.0;
                self.since = 0.0;
                app.au.play(Sfx::Step, 0.4);
            } else {
                return self.finish_talk(app);
            }
        }
        None
    }

    fn welcome(&mut self, app: &mut App, who: Cr) -> Option<Nav> {
        let u = app.u;
        let cx = app.x0 + app.pw / 2.0;
        let d = who.def();
        app.g.rect(Rect::new(app.x0, 0.0, app.pw, app.h), alpha(SHADE, 0.55));
        let a = ease_out((self.since / 0.6).min(1.0));
        let cy = app.h * 0.3;
        let s = 190.0 * u * (0.92 + 0.08 * a);
        let g = &app.g;
        portrait(g, cr_face(who), cx, cy, s, cr_color(who), alpha(WHITE, a));
        g.head(&d.name.to_uppercase(), cx, cy + 128.0 * u, 34.0 * u, 0.5, alpha(INK, a));
        g.head("JOINS THE PACK", cx, cy + 158.0 * u, 13.0 * u, 0.5, alpha(cr_color(who), a));
        let card = Rect::new(app.x0 + 20.0 * u, cy + 184.0 * u, app.pw - 40.0 * u, 138.0 * u);
        panel(g, card, u, WHITE);
        g.head(&format!("THE {}  -  {}", d.species.to_uppercase(), d.role.to_uppercase()), cx, card.y + 22.0 * u, 11.0 * u, 0.5, DIM);
        let used = g.para(d.blurb, cx, card.y + 36.0 * u, 13.5 * u, card.w - 36.0 * u, INK);
        g.para("Followers learn skills with bond points. Find them under PACK on the map.", cx, card.y + 44.0 * u + used, 12.0 * u, card.w - 36.0 * u, DIM);
        let go = Rect::new(cx - 110.0 * u, (card.y + card.h + 14.0 * u).min(app.h - app.bot - 70.0 * u), 220.0 * u, 56.0 * u);
        app.mark("welcome", go);
        if app.ui.button(&app.g, &mut app.au, 2, go, GO, "WALK ON", None, self.since > 0.6) {
            let p = app.save.p_mut();
            if let Some(slot) = p.free_slot() {
                p.join(who, slot);
            }
            app.save.store();
            return Some(Nav::Map);
        }
        None
    }

    fn crowded(&mut self, app: &mut App, who: Cr, pick: usize) -> Option<Nav> {
        let u = app.u;
        let cx = app.x0 + app.pw / 2.0;
        app.g.rect(Rect::new(app.x0, 0.0, app.pw, app.h), alpha(SHADE, 0.7));
        let p = app.save.p().clone();
        let cast: Vec<Cr> = p.party.iter().flatten().copied().chain([who]).collect();
        let g = &app.g;
        g.head(&join_line(who).to_uppercase(), cx, app.top + 50.0 * u, (17.0 * u).min(app.pw * 1.2 / join_line(who).len() as f32), 0.5, INK);
        g.para(&format!("Only {FOLLOWERS} can walk with you. Tap whoever waits at home."), cx, app.top + 70.0 * u, 14.0 * u, app.pw - 50.0 * u, DIM);
        let top = app.top + 118.0 * u;
        let cw = (app.pw - 32.0 * u) / 3.0;
        let ch = 222.0 * u;
        let mut choose = None;
        for (i, &c) in cast.iter().enumerate() {
            let r = Rect::new(app.x0 + 12.0 * u + i as f32 * (cw + 4.0 * u), top, cw, ch);
            let out = i == pick;
            let (held, clicked) = app.ui.press(10 + i as u32, r, true);
            let g = &app.g;
            let body = Rect::new(r.x, r.y + if held { 2.0 * u } else { 0.0 }, r.w, r.h);
            panel(g, body, u, if out { rgb(0x8a8a8a) } else { WHITE });
            let s = cw * 0.74;
            portrait(g, cr_face(c), body.x + cw / 2.0, body.y + s * 0.66, s, if out { alpha(INK, 0.3) } else { cr_color(c) }, if out { rgb(0x666666) } else { WHITE });
            g.head(&c.def().name.to_uppercase(), body.x + cw / 2.0, body.y + s + 34.0 * u, 14.0 * u, 0.5, if out { DIM } else { INK });
            g.label(c.def().role, body.x + cw / 2.0, body.y + s + 52.0 * u, 12.0 * u, 0.5, DIM, None, 0.0);
            let pts = spent(c, p.skills_of(c));
            let note = if i == 2 { "new".to_string() } else { format!("{pts} bond point{}", if pts == 1 { "" } else { "s" }) };
            g.label(&note, body.x + cw / 2.0, body.y + s + 68.0 * u, 11.5 * u, 0.5, FROST, None, 0.0);
            g.head(if out { "WAITS AT HOME" } else { "WALKS WITH YOU" }, body.x + cw / 2.0, body.y + ch - 20.0 * u, 9.5 * u, 0.5, if out { BLOOD } else { MOSS });
            app.mark(&format!("pick{i}"), r);
            if clicked {
                choose = Some(i);
            }
        }
        if let Some(i) = choose {
            self.stage = Stage::Crowded(who, i);
            app.au.play(Sfx::Click, 0.8);
        }
        let g = &app.g;
        let stay = cast[pick];
        let text = if pick == 2 {
            format!("{} stays at home for now. You can change this at any time under PACK.", stay.def().name)
        } else {
            let pts = spent(stay, p.skills_of(stay));
            format!("{} goes home, and what was spent on {} ({} bond point{}) comes back to you. You can change this at any time under PACK.", stay.def().name, stay.def().name, pts, if pts == 1 { "" } else { "s" })
        };
        g.para(&text, cx, top + ch + 14.0 * u, 13.0 * u, app.pw - 44.0 * u, INK);
        let go = Rect::new(cx - 120.0 * u, (top + ch + 84.0 * u).min(app.h - app.bot - 70.0 * u), 240.0 * u, 56.0 * u);
        app.mark("decide", go);
        if app.ui.button(&app.g, &mut app.au, 3, go, GO, "SO BE IT", None, self.since > 0.5) {
            if pick < 2 {
                app.save.p_mut().join(who, pick);
                app.log(&format!("SWAP {:?} for {:?}", stay, who));
            }
            app.save.store();
            return Some(Nav::Map);
        }
        None
    }
}
