//! The story, told a line at a time, and what comes after a resident's
//! home has been held: their offer to come along, and (once the pack is
//! full) the choice of who stays behind.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::content::{join_line, lines, Beat, Line, Who};
use crate::fx::Fx;
use crate::gfx::*;
use crate::ui::*;
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
    pal: Palette,
    fx: Fx,
    since: f32,
}

impl Story {
    pub fn new(app: &mut App, beat: Beat) -> Story {
        let hero = app.save.p().hero;
        app.au.track(Track::Trail);
        app.log(&format!("SCREEN story {beat:?}"));
        let lines = lines(beat, hero);
        Story { beat, lines, i: 0, shown: 0.0, t: 0.0, hero, stage: Stage::Talk, pal: palette(beat.biome()), fx: Fx::new(app.rng.next_u64()), since: 0.0 }
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
        self.fx.update(dt);
        let cx = app.x0 + app.pw / 2.0;
        let g = &app.g;

        // somewhere to stand: the biome's ground under an evening sky
        let dark = matches!(self.beat, Beat::Intro);
        let horizon = app.h * 0.52;
        let (sky_a, sky_b) = if dark { (rgb(0x1a1f3a), rgb(0x5a3a4a)) } else if matches!(self.beat, Beat::Ending) { (rgb(0x6ab8f0), rgb(0xffe6b0)) } else { (rgb(0x7ac0ee), rgb(0xe8f4d8)) };
        g.vgrad(Rect::new(0.0, 0.0, app.w, horizon), sky_a, sky_b);
        let (ga, gb) = self.pal.ground;
        let shade = if dark { 0.45 } else { 0.0 };
        g.vgrad(Rect::new(0.0, horizon, app.w, app.h - horizon), mix(ga, rgb(0x101020), shade), mix(gb, rgb(0x101020), shade));
        // a far line of the biome's tall things, and its home
        for k in 0..7 {
            let x = app.x0 + (k as f32 + 0.5) / 7.0 * app.pw + ((k * 37) % 11) as f32 * 2.0 * u;
            let s = (70.0 + ((k * 53) % 40) as f32) * u;
            g.spr_ex(self.pal.tall, x, horizon - s * 0.3, s, s, 0.0, mix(WHITE, sky_b, 0.45), k % 2 == 0);
        }
        if !matches!(self.beat, Beat::Intro) {
            let hs = 190.0 * u;
            g.spr(self.pal.home, cx + 96.0 * u, horizon + 30.0 * u, hs, hs, 0.0, WHITE);
        } else {
            // the machine, looming
            let s = 260.0 * u;
            g.spr(spr::BIG_YELLOW, cx + 70.0 * u + (self.t * 0.7).sin() * 4.0 * u, horizon - 6.0 * u, s, s, 0.0, rgb(0x6a6470));
        }
        for k in 0..6 {
            let x = app.x0 + ((k * 71 + 20) % 100) as f32 / 100.0 * app.pw;
            let y = horizon + (40.0 + ((k * 47) % 200) as f32) * u;
            g.spr_fit(self.pal.tufts[k % 2], x, y, 34.0 * u, 0.0, mix(WHITE, rgb(0x101020), shade));
        }
        if app.x0 > 1.0 {
            g.rect(Rect::new(0.0, 0.0, app.x0, app.h), rgb(0x16240f));
            g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), rgb(0x16240f));
        }

        match self.stage {
            Stage::Talk => self.talk(app),
            Stage::Welcome(who) => self.welcome(app, who),
            Stage::Crowded(who, pick) => self.crowded(app, who, pick),
        }
    }

    fn talk(&mut self, app: &mut App) -> Option<Nav> {
        let u = app.u;
        let cx = app.x0 + app.pw / 2.0;
        let horizon = app.h * 0.52;
        let line = &self.lines[self.i];
        let speaker = self.who(line.who);
        let total = line.text.chars().count() as f32;
        let before = self.shown as usize;
        self.shown = (self.shown + app.dt * 46.0).min(total);
        if (self.shown as usize) / 3 != before / 3 && self.shown < total {
            app.au.play(Sfx::Blip, 0.35);
        }
        let g = &app.g;

        // whoever is speaking stands forward; the last other speaker listens
        let ground = horizon + 150.0 * u;
        let listener = self.lines[..self.i].iter().rev().filter_map(|l| self.who(l.who)).find(|&c| Some(c) != speaker);
        if let Some(c) = listener {
            let right = c != self.hero;
            let x = if right { cx + 104.0 * u } else { cx - 104.0 * u };
            let s = 118.0 * u;
            g.spr(spr::SHADOW, x, ground, s * 0.8, s * 0.22, 0.0, alpha(WHITE, 0.5));
            g.spr_ex(cr_spr(c), x, ground - s * 0.46, s, s, 0.0, rgb(0xa8a8b0), right);
        }
        if let Some(c) = speaker {
            let right = c != self.hero;
            let x = if right { cx + 84.0 * u } else { cx - 84.0 * u };
            let enter = ease_back((self.since / 0.3).min(1.0));
            let s = 176.0 * u * (0.85 + 0.15 * enter);
            let talk = if self.shown < total { (self.t * 14.0).sin().abs() * 5.0 * u } else { 0.0 };
            g.spr(spr::SHADOW, x, ground + 6.0 * u, s * 0.8, s * 0.22, 0.0, alpha(WHITE, 0.6));
            g.spr_ex(cr_spr(c), x, ground - s * 0.44 - talk, s, s * (1.0 + 0.02 * (self.t * 2.4).sin()), 0.0, WHITE, right);
        }

        // the words
        let bw = app.pw - 24.0 * u;
        let bh = 150.0 * u;
        let by = app.h - app.bot - bh - 16.0 * u;
        let boxr = Rect::new(cx - bw / 2.0, by, bw, bh);
        match speaker {
            Some(c) => {
                panel(g, boxr, u, WHITE);
                let plate = Rect::new(boxr.x + 16.0 * u, boxr.y - 16.0 * u, g.text_w(c.def().name, 17.0 * u) + 34.0 * u, 34.0 * u);
                g.nine(spr::BTN, plate, 32.0, 14.0 * u, cr_color(c));
                g.label(c.def().name, plate.x + plate.w / 2.0, plate.y + plate.h * 0.45, 17.0 * u, 0.5, WHITE, Some((alpha(SOIL, 0.7), 1.5 * u)), 0.0);
            }
            None => g.nine(spr::CHIP, boxr, 24.0, 18.0 * u, WHITE),
        }
        let text: String = line.text.chars().take(self.shown as usize).collect();
        let size = 17.0 * u;
        let color = if speaker.is_some() { BARK } else { CREAM };
        let wrapped = g.wrap(line.text, size, bw - 44.0 * u);
        // wrap the whole line, then reveal: words do not jump between rows as they appear
        let mut left = text.chars().count();
        for (row, full) in wrapped.iter().enumerate() {
            let n = full.chars().count().min(left);
            let part: String = full.chars().take(n).collect();
            g.label(&part, boxr.x + 22.0 * u, boxr.y + 34.0 * u + row as f32 * size * 1.25, size, 0.0, color, None, 0.0);
            left = left.saturating_sub(n + 1);
            if left == 0 {
                break;
            }
        }
        if self.shown >= total {
            let bob = (self.t * 5.0).sin().abs() * 4.0 * u;
            g.spr_fit(spr::IC_NEXT, boxr.x + bw - 28.0 * u, boxr.y + bh - 24.0 * u + bob, 22.0 * u, std::f32::consts::FRAC_PI_2, if speaker.is_some() { rgb(0xc08a3a) } else { CREAM });
        }
        g.label(&format!("{} / {}", self.i + 1, self.lines.len()), boxr.x + 22.0 * u, boxr.y + bh - 16.0 * u, 10.0 * u, 0.0, alpha(color, 0.5), None, 0.0);

        let skip = Rect::new(app.x0 + app.pw - 92.0 * u, app.top + 14.0 * u, 80.0 * u, 38.0 * u);
        app.mark("skip", skip);
        app.mark("next", boxr);
        if app.ui.button(&app.g, &mut app.au, 1, skip, rgb(0x6a7a8a), "SKIP", None, true) {
            return self.finish_talk(app);
        }
        if app.ui.ptr.released && !skip.contains(app.ui.ptr.press_pos) {
            if self.shown < total {
                self.shown = total;
            } else if self.i + 1 < self.lines.len() {
                self.i += 1;
                self.shown = 0.0;
                self.since = 0.0;
                app.au.play(Sfx::Step, 0.5);
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
        app.g.rect(Rect::new(app.x0, 0.0, app.pw, app.h), alpha(rgb(0x10200c), 0.5));
        let pop = ease_back((self.since / 0.5).min(1.0));
        let cy = app.h * 0.36;
        let s = 220.0 * u * pop;
        let g = &app.g;
        g.spr(spr::GLOW, cx, cy, s * 2.0, s * 2.0, 0.0, alpha(cr_color(who), 0.75));
        g.lights(|| g.spr(spr::RAYS, cx, cy, s * 2.4, s * 2.4, self.t * 0.3, alpha(WHITE, if g.has_light() { 0.3 } else { 0.0 })));
        g.spr(cr_spr(who), cx, cy, s, s, (self.t * 3.0).sin() * 0.04, WHITE);
        if self.since < dt_once(app.dt) {
            self.fx.confetti(app.x0, app.x0 + app.pw, app.top, 60, u);
        }
        self.fx.draw(&app.g);
        let g = &app.g;
        g.title(&format!("{} joins the pack!", d.name), cx, cy + 150.0 * u, (28.0 * u).min(app.pw * 1.4 / (d.name.len() as f32 + 16.0)), rgb(0xfff07a), rgb(0x2a4a12));
        let card = Rect::new(app.x0 + 20.0 * u, cy + 176.0 * u, app.pw - 40.0 * u, 128.0 * u);
        panel(g, card, u, WHITE);
        g.label(&format!("{} the {}  -  {}", d.name, d.species, d.role), cx, card.y + 24.0 * u, 15.0 * u, 0.5, cr_color(who), Some((BARK, 1.4 * u)), 0.0);
        let used = g.para(d.blurb, cx, card.y + 38.0 * u, 13.0 * u, card.w - 36.0 * u, BARK);
        g.para("Followers learn skills with bond points. Find them under PACK on the map.", cx, card.y + 44.0 * u + used, 11.5 * u, card.w - 36.0 * u, alpha(BARK, 0.7));
        let go = Rect::new(cx - 110.0 * u, (card.y + card.h + 14.0 * u).min(app.h - app.bot - 70.0 * u), 220.0 * u, 58.0 * u);
        app.mark("welcome", go);
        if app.ui.button(&app.g, &mut app.au, 2, go, GREEN, "WELCOME!", None, self.since > 0.6) {
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
        app.g.rect(Rect::new(app.x0, 0.0, app.pw, app.h), alpha(rgb(0x10200c), 0.6));
        let p = app.save.p().clone();
        let cast: Vec<Cr> = p.party.iter().flatten().copied().chain([who]).collect();
        let g = &app.g;
        g.title(join_line(who), cx, app.top + 50.0 * u, (22.0 * u).min(app.pw * 1.45 / join_line(who).len() as f32), rgb(0xfff07a), rgb(0x2a4a12));
        g.para(&format!("But only {FOLLOWERS} can walk with you. Tap whoever waits at home."), cx, app.top + 70.0 * u, 14.0 * u, app.pw - 50.0 * u, WHITE);
        let top = app.top + 120.0 * u;
        let cw = (app.pw - 32.0 * u) / 3.0;
        let ch = 230.0 * u;
        let mut choose = None;
        for (i, &c) in cast.iter().enumerate() {
            let r = Rect::new(app.x0 + 12.0 * u + i as f32 * (cw + 4.0 * u), top, cw, ch);
            let out = i == pick;
            let (held, clicked) = app.ui.press(10 + i as u32, r, true);
            let g = &app.g;
            let body = Rect::new(r.x, r.y + if held { 2.0 * u } else { 0.0 }, r.w, r.h);
            g.nine(spr::PANEL, body, 32.0, 18.0 * u, if out { rgb(0xb8b0a8) } else { WHITE });
            let s = cw * 0.86;
            g.spr(cr_spr(c), body.x + cw / 2.0, body.y + s * 0.58, s, s, 0.0, if out { rgb(0x8a8a90) } else { WHITE });
            g.label(c.def().name, body.x + cw / 2.0, body.y + s + 16.0 * u, 17.0 * u, 0.5, if out { alpha(BARK, 0.7) } else { cr_color(c) }, Some((BARK, 1.3 * u)), 0.0);
            g.label(c.def().role, body.x + cw / 2.0, body.y + s + 34.0 * u, 12.0 * u, 0.5, alpha(BARK, 0.8), None, 0.0);
            let pts = spent(c, p.skills_of(c));
            let note = if i == 2 { "new".to_string() } else { format!("{pts} bond pts") };
            g.label(&note, body.x + cw / 2.0, body.y + s + 52.0 * u, 12.0 * u, 0.5, rgb(0x3a7ab0), None, 0.0);
            let tag = Rect::new(body.x + 8.0 * u, body.y + ch - 40.0 * u, cw - 16.0 * u, 28.0 * u);
            g.nine(spr::PILL, tag, 31.0, 14.0 * u, if out { rgb(0x8a6a5a) } else { rgb(0x3fa84a) });
            g.label(if out { "waits at home" } else { "walks with you" }, tag.x + tag.w / 2.0, tag.y + tag.h * 0.5, 11.0 * u, 0.5, WHITE, None, 0.0);
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
            format!("{} stays at home for now. You can swap at any time under PACK.", stay.def().name)
        } else {
            let pts = spent(stay, p.skills_of(stay));
            format!("{} goes home, and the {} bond points spent on {} come back to you. You can swap at any time under PACK.", stay.def().name, pts, stay.def().name)
        };
        g.para(&text, cx, top + ch + 14.0 * u, 13.0 * u, app.pw - 44.0 * u, WHITE);
        let go = Rect::new(cx - 120.0 * u, (top + ch + 80.0 * u).min(app.h - app.bot - 70.0 * u), 240.0 * u, 58.0 * u);
        app.mark("decide", go);
        if app.ui.button(&app.g, &mut app.au, 3, go, GREEN, "THAT IS THE PLAN", None, self.since > 0.5) {
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

/// True for the first frame after `since` was reset.
fn dt_once(dt: f32) -> f32 {
    dt * 1.5
}
