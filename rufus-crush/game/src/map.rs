//! The level map: Rufus's days, top to bottom, one chapter after another.

use crate::audio::Sfx;
use crate::content::*;
use crate::fx::Fx;
use crate::gfx::*;
use crate::rufus::{Mood, Rufus};
use crate::ui::*;
use crate::{App, Nav};
use macroquad::prelude::{vec2, Rect, Vec2};
use rufus_core::chapters::day_of;
use rufus_core::level::{chapter_count, chapter_levels, CHAPTERS, CHAPTER_LEN};
use rufus_core::LEVELS;

const BANNER_H: f32 = 132.0;
const STEP: f32 = 88.0;
const CHAPTER_H: f32 = BANNER_H + 34.0 + STEP * CHAPTER_LEN as f32;
const HEADER: f32 = 58.0;
const TAIL: f32 = 250.0;
/// Where each level sits across the path, as a fraction of the half-width.
const WEAVE: [f32; 6] = [-0.62, -0.1, 0.5, 0.66, 0.12, -0.46];

pub struct MapScreen {
    scroll: f32,
    vel: f32,
    dragging: bool,
    last_y: f32,
    t: f32,
    focus: usize,
    rufus: Rufus,
    fx: Fx,
    nudge: Option<(usize, f32)>,
    placed: bool,
}

impl MapScreen {
    pub fn new(app: &mut App, focus: Option<usize>) -> MapScreen {
        if app.debug {
            macroquad::logging::info!("RUFUS SCREEN map");
        }
        let frontier = app.save.frontier();
        let mut rufus = Rufus::new(app.rng.next_u64());
        rufus.rest(Mood::Asleep);
        MapScreen {
            scroll: 0.0,
            vel: 0.0,
            dragging: false,
            last_y: 0.0,
            t: 0.0,
            focus: focus.unwrap_or(frontier).min(LEVELS.len() - 1),
            rufus,
            fx: Fx::new(app.rng.next_u64()),
            nudge: None,
            placed: false,
        }
    }

    /// Centre of a level's button, in content space (before scrolling).
    fn node(app: &App, level: usize) -> Vec2 {
        let u = app.u;
        let (c, i) = (level / CHAPTER_LEN, level % CHAPTER_LEN);
        let y = c as f32 * CHAPTER_H + BANNER_H + 34.0 + STEP * (i as f32 + 0.5);
        vec2(app.x0 + app.pw / 2.0 + WEAVE[i] * app.pw * 0.36, y * u)
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        let header = app.top + HEADER * u;
        let chapters = chapter_count();
        let content_h = (chapters as f32 * CHAPTER_H + TAIL) * u;
        let view_h = app.h - header;
        let max_scroll = (content_h - view_h).max(0.0);
        if !self.placed {
            self.placed = true;
            self.scroll = (Self::node(app, self.focus).y - view_h * 0.5).clamp(0.0, max_scroll);
        }
        let frontier = app.save.frontier();
        let cx = app.x0 + app.pw / 2.0;

        // The map is a long way from top to bottom now. Whenever the level
        // you're up to is out of sight, a button takes you back to it.
        let here = Self::node(app, frontier).y + header - self.scroll;
        let jump = (here < header - 30.0 * u || here > app.h + 30.0 * u).then(|| Rect::new(cx - 84.0 * u, app.h - app.bot - 62.0 * u, 168.0 * u, 46.0 * u));
        let on_jump = |p: Vec2| jump.is_some_and(|r| r.contains(p));

        // drag to scroll, with a bit of glide after letting go
        let ptr = &app.ui.ptr;
        if ptr.pressed && ptr.pos.y > header && !on_jump(ptr.pos) {
            self.dragging = true;
            self.vel = 0.0;
            self.last_y = ptr.pos.y;
        }
        if self.dragging && ptr.down {
            let dy = ptr.pos.y - self.last_y;
            self.last_y = ptr.pos.y;
            self.scroll -= dy;
            if dt > 0.0 {
                self.vel = lerp(self.vel, -dy / dt, 0.5);
            }
        } else {
            self.dragging = false;
            self.scroll += self.vel * dt;
            self.vel *= (-dt * 4.5).exp();
        }
        if self.scroll < 0.0 || self.scroll > max_scroll {
            self.scroll = self.scroll.clamp(0.0, max_scroll);
            self.vel = 0.0;
        }

        let tap = ptr.released && ptr.travel < 12.0 * u && ptr.pos.y > header && !on_jump(ptr.pos);
        let tap_pos = ptr.pos;
        let g = &app.g;
        let oy = header - self.scroll;
        let mut nav = None;

        // each chapter gets its own colours and its backdrop as a banner
        g.rect(Rect::new(0.0, 0.0, app.w, app.h), theme(0).bottom);
        for c in 0..chapters {
            let th = theme(c);
            let top = oy + c as f32 * CHAPTER_H * u;
            let h = (if c + 1 == chapters { CHAPTER_H + TAIL } else { CHAPTER_H }) * u;
            if top > app.h || top + h < 0.0 {
                continue;
            }
            g.vgrad(Rect::new(0.0, top, app.w, h + 1.0), th.top, th.bottom);
            for k in 0..9 {
                let fx = (k as f32 * 0.618_034 + c as f32 * 0.31).fract();
                let fy = (k as f32 * 0.29 + 0.13).fract();
                g.spr(spr::P_PAW, app.x0 + fx * app.pw, top + BANNER_H * u + fy * (h - BANNER_H * u), 28.0 * u, 28.0 * u, k as f32 * 1.7, alpha(WHITE, 0.06));
            }
            let banner = Rect::new(app.x0, top, app.pw, BANNER_H * u);
            g.scene(th.scene, banner, th.light);
            g.vgrad(Rect::new(banner.x, banner.y + banner.h - 2.0, banner.w, 12.0 * u), alpha(rgb(0x000000), 0.2), alpha(rgb(0x000000), 0.0));
            let rw = 268.0 * u;
            g.spr(spr::RIBBON, cx, banner.y + banner.h - 6.0 * u, rw, rw * 0.28, 0.0, WHITE);
            // long names shrink to stay on the ribbon
            let name = CHAPTERS[c].name;
            let name_size = (19.0 * u).min(19.0 * u * (rw * 0.8) / g.text_w(name, 19.0 * u).max(1.0));
            g.label(name, cx, banner.y + banner.h - 15.0 * u, name_size, 0.5, WHITE, Some((rgb(0x7d0f1b), 1.6 * u)), 0.0);
            let levels = chapter_levels(c);
            let in_chapter = levels.len();
            let earned: u32 = levels.map(|i| app.save.paws[i] as u32).sum();
            g.nine(spr::CHIP, Rect::new(banner.x + 10.0 * u, banner.y + 10.0 * u, 74.0 * u, 26.0 * u), 22.0, 11.0 * u, WHITE);
            g.spr_fit(spr::PAW_GOLD, banner.x + 25.0 * u, banner.y + 23.0 * u, 17.0 * u, 0.0, WHITE);
            g.label(&format!("{earned}/{}", in_chapter * 3), banner.x + 37.0 * u, banner.y + 23.0 * u, 13.0 * u, 0.0, WHITE, None, 0.0);
            // which day of Rufus's life this is
            let day = Rect::new(banner.x + banner.w - 78.0 * u, banner.y + 10.0 * u, 68.0 * u, 26.0 * u);
            g.nine(spr::CHIP, day, 22.0, 11.0 * u, WHITE);
            g.label(&format!("DAY {}", day_of(c)), day.x + day.w / 2.0, day.y + day.h / 2.0, 13.0 * u, 0.5, WHITE, None, 0.0);
        }
        if app.x0 > 1.0 {
            g.rect(Rect::new(0.0, 0.0, app.x0, app.h), alpha(rgb(0x000000), 0.28));
            g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), alpha(rgb(0x000000), 0.28));
        }

        // the path: a trail of paw prints from one level to the next
        for level in 0..LEVELS.len() - 1 {
            let (a, b) = (Self::node(app, level) + vec2(0.0, oy), Self::node(app, level + 1) + vec2(0.0, oy));
            if a.y.max(b.y) < -40.0 || a.y.min(b.y) > app.h + 40.0 {
                continue;
            }
            let walked = app.save.paws[level] > 0;
            let d = b - a;
            let n = (d.length() / (21.0 * u)).floor().max(2.0) as usize;
            let dir = d.y.atan2(d.x) + std::f32::consts::FRAC_PI_2;
            let side = vec2(-d.y, d.x).normalize_or_zero() * 6.0 * u;
            for k in 1..n {
                let p = a + d * (k as f32 / n as f32) + side * if k % 2 == 0 { 1.0 } else { -1.0 };
                g.spr(spr::P_PAW, p.x, p.y, 13.0 * u, 13.0 * u, dir, if walked { alpha(rgb(0xfff4d6), 0.95) } else { alpha(rgb(0x0b1026), 0.22) });
            }
        }

        // level buttons
        for level in 0..LEVELS.len() {
            let mut p = Self::node(app, level) + vec2(0.0, oy);
            if p.y < -60.0 * u || p.y > app.h + 60.0 * u {
                continue;
            }
            let paws = app.save.paws[level];
            let open = app.save.unlocked(level);
            let current = level == frontier && paws == 0;
            if let Some((l, t)) = self.nudge {
                if l == level {
                    p.x += wobble(t, 50.0, 9.0) * 7.0 * u;
                }
            }
            let size = if current { 72.0 * u * (1.0 + 0.05 * (self.t * 5.0).sin()) } else { 62.0 * u };
            let color = if !open { rgb(0x8f97aa) } else if current { ORANGE } else if paws == 3 { rgb(0xffc21f) } else { GREEN };
            if current {
                g.spr(spr::GLOW, p.x, p.y, size * 2.0, size * 2.0, 0.0, alpha(rgb(0xfff2a0), 0.6 + 0.2 * (self.t * 5.0).sin()));
            }
            g.spr(spr::NODE, p.x, p.y, size, size, 0.0, color);
            if open {
                // four digits have to fit where one used to
                let number = (level + 1).to_string();
                let digits = [0.42, 0.42, 0.36, 0.29][number.len().clamp(1, 4) - 1];
                g.label(&number, p.x, p.y - size * 0.055, size * digits, 0.5, WHITE, Some((alpha(NAVY, 0.6), size * 0.03)), 0.0);
                if paws > 0 {
                    for i in 0..3 {
                        let x = p.x + (i as f32 - 1.0) * 17.0 * u;
                        let y = p.y + size * 0.58 - if i == 1 { -3.0 * u } else { 0.0 };
                        g.spr_fit(if i < paws { spr::PAW_GOLD } else { spr::PAW_GRAY }, x, y, 17.0 * u, (i as f32 - 1.0) * 0.2, WHITE);
                    }
                }
            } else {
                g.spr_fit(spr::IC_LOCK, p.x, p.y - size * 0.05, size * 0.44, 0.0, alpha(WHITE, 0.9));
            }
            if current {
                // Rufus marks the spot
                let hop = (self.t * 4.0).sin().abs() * 9.0 * u;
                g.spr_fit(spr::SUPER, p.x + size * 0.56, p.y - size * 0.52 - hop, 42.0 * u, 0.12, WHITE);
            }
            // the name of the level you're up to (and the one just tapped)
            if current || self.nudge.is_some_and(|(l, _)| l == level) {
                let title = LEVELS[level].title;
                let left = p.x < cx;
                let tx = if left { p.x + size * 0.62 + if current { 30.0 * u } else { 0.0 } } else { p.x - size * 0.62 };
                // shrink a long title rather than let it run off the edge
                let room = if left { app.x0 + app.pw - 8.0 * u - tx } else { tx - app.x0 - 8.0 * u };
                let title_size = (14.5 * u).min(14.5 * u * room / g.text_w(title, 14.5 * u).max(1.0));
                g.label(title, tx, p.y + 2.0 * u, title_size, if left { 0.0 } else { 1.0 }, WHITE, Some((alpha(NAVY, 0.75), 1.5 * u)), 0.0);
            }
            if level == frontier {
                app.mark("node", Rect::new(p.x - 1.0, p.y - 1.0, 2.0, 2.0));
            }
            if tap && nav.is_none() && tap_pos.distance(p) < size * 0.62 {
                if open {
                    app.au.play(Sfx::Click, 0.8);
                    nav = Some(Nav::Play(level));
                } else {
                    app.au.play(Sfx::Bonk, 0.7);
                    self.nudge = Some((level, 0.0));
                    let f = self.fx.text(&format!("Finish level {} first!", frontier + 1), p - vec2(0.0, size * 0.75), 15.0 * u, WHITE, NAVY, 1.4);
                    f.pos.x = f.pos.x.clamp(app.x0 + 90.0 * u, app.x0 + app.pw - 90.0 * u);
                }
            }
        }
        if let Some((_, t)) = &mut self.nudge {
            *t += dt;
            if *t > 1.6 {
                self.nudge = None;
            }
        }

        // the end of the day
        let end_y = oy + (chapters as f32 * CHAPTER_H + 150.0) * u;
        if end_y > -200.0 * u && end_y < app.h + 200.0 * u {
            self.rufus.update(dt, &mut self.fx);
            self.rufus.draw(g, cx, end_y + 40.0 * u, 120.0 * u);
            let all = app.save.paws.iter().all(|&p| p > 0);
            let line = if all { "Every single day of being a very good dog." } else { "The last bedtime is down here. Eventually." };
            g.label(line, cx, end_y + 66.0 * u, 15.0 * u, 0.5, WHITE, Some((alpha(NAVY, 0.7), 1.4 * u)), 0.0);
        }
        self.fx.update(dt);
        self.fx.draw(g);
        self.fx.draw_text(g);

        // header bar
        g.rect(Rect::new(0.0, 0.0, app.w, header), rgb(0x24407a));
        g.vgrad(Rect::new(0.0, header, app.w, 8.0 * u), alpha(rgb(0x000000), 0.25), alpha(rgb(0x000000), 0.0));

        // as wide as its numbers: the total runs to four digits now
        let tally = format!("{}/{}", app.save.total_paws(), LEVELS.len() * 3);
        let chip_w = (39.0 * u + g.text_w(&tally, 13.5 * u)).max(76.0 * u);
        let chip = Rect::new(app.x0 + app.pw - 8.0 * u - chip_w, app.top + 15.0 * u, chip_w, 28.0 * u);
        g.nine(spr::CHIP, chip, 22.0, 12.0 * u, WHITE);
        g.spr_fit(spr::PAW_GOLD, chip.x + 16.0 * u, chip.y + chip.h * 0.5, 18.0 * u, 0.0, WHITE);
        g.label(&tally, chip.x + 29.0 * u, chip.y + chip.h * 0.5, 13.5 * u, 0.0, WHITE, None, 0.0);
        let title_x = cx.min((app.x0 + 52.0 * u + chip.x) / 2.0);
        g.label("Rufus's Big Days", title_x, app.top + HEADER * u * 0.5, 21.0 * u, 0.5, WHITE, Some((alpha(NAVY, 0.7), 1.6 * u)), 0.0);
        let home = Rect::new(app.x0 + 8.0 * u, app.top + 9.0 * u, 40.0 * u, 40.0 * u);
        app.mark("home", home);
        if app.ui.button(&app.g, &mut app.au, 1, home, BLUE, "", Some(spr::IC_HOME), true) {
            nav = Some(Nav::Title);
        }
        if let Some(r) = jump {
            app.mark("jump", r);
            if app.ui.button(&app.g, &mut app.au, 2, r, ORANGE, &format!("LEVEL {}", frontier + 1), Some(spr::IC_PLAY), true) {
                self.scroll = (Self::node(app, frontier).y - view_h * 0.5).clamp(0.0, max_scroll);
                self.vel = 0.0;
                self.dragging = false;
            }
        }
        nav
    }
}
