//! The journey: one long trail up through the six biomes, three stages in
//! each. The pack stands on the trail and walks on when a stage is won.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::content::STAGE_LINES;
use crate::gfx::*;
use crate::ui::*;
use crate::widgets::*;
use crate::{App, Nav};
use biome_core::level::{is_finale, Difficulty, LEVELS, PER_BIOME};
use biome_core::{Rng, BIOMES, LEVEL_COUNT};
use macroquad::prelude::{draw_circle, vec2, Rect, Vec2};

/// Height of one biome on the map, in layout units.
const BAND: f32 = 500.0;
/// Where the first biome starts, above the bottom of the map.
const FOOT: f32 = 70.0;

/// Where a stage's marker sits: (sideways from the middle, height up the map), in units.
fn node(level: usize) -> Vec2 {
    let b = (level / PER_BIOME) as f32;
    let k = (level % PER_BIOME) as f32;
    // three to a biome, with room left above the last for the next biome's sign
    vec2(104.0 * (level as f32 * 1.25 + 0.6).sin(), FOOT + b * BAND + 150.0 + k * 135.0)
}

/// The trail as a smooth line through every marker: `s` runs from -1 (the
/// foot of the map) through 0..17 (the stages) to 18 (off the top).
fn trail(s: f32) -> Vec2 {
    let pt = |i: i32| -> Vec2 {
        if i < 0 {
            vec2(0.0, FOOT - 60.0 + i as f32 * 40.0)
        } else if i >= LEVEL_COUNT as i32 {
            node(LEVEL_COUNT - 1) + vec2(30.0, 90.0 * (i - LEVEL_COUNT as i32 + 1) as f32)
        } else {
            node(i as usize)
        }
    };
    let i = s.floor() as i32;
    let f = s - i as f32;
    let (p0, p1, p2, p3) = (pt(i - 1), pt(i), pt(i + 1), pt(i + 2));
    // Catmull-Rom
    0.5 * ((2.0 * p1) + (-p0 + p2) * f + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * f * f + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * f * f * f)
}

struct Prop {
    at: Vec2,
    spr: Spr,
    size: f32,
    flip: bool,
}

pub struct MapScreen {
    t: f32,
    scroll: f32,
    vel: f32,
    dragging: bool,
    /// Where on the trail the pack is (in stages; see [`trail`]).
    at: f32,
    target: f32,
    /// The stage whose card is open.
    open: Option<usize>,
    props: Vec<Prop>,
    settled: bool,
    prev_y: f32,
}

impl MapScreen {
    pub fn new(app: &mut App) -> MapScreen {
        app.au.track(Track::Trail);
        app.log("SCREEN map");
        let p = app.save.p();
        let front = p.frontier();
        // just won a stage: start there and walk on
        let (at, target) = match app.walk_from.take() {
            Some(from) if from < front || p.finished() => (from as f32, if p.finished() { from as f32 } else { front as f32 }),
            _ => (front as f32, front as f32),
        };
        // scenery, the same every time
        let mut rng = Rng::new(4242);
        let mut props = Vec::new();
        for b in 0..BIOMES.len() {
            let pal = palette(b);
            for _ in 0..20 {
                let rise = rng.range(20.0, BAND - 10.0);
                let y = FOOT + b as f32 * BAND + rise;
                let side = if rng.chance(0.5) { -1.0 } else { 1.0 };
                let x = side * rng.range(60.0, 185.0);
                // keep clear of the biome's sign
                if rise < 100.0 && x.abs() < 140.0 {
                    continue;
                }
                // keep off the trail
                let near = (0..LEVEL_COUNT * 4).any(|k| trail(k as f32 / 4.0).distance(vec2(x, y)) < 46.0);
                if near {
                    continue;
                }
                let roll = rng.unit();
                let (spr, size) = if roll < 0.4 { (pal.tall, rng.range(62.0, 92.0)) } else if roll < 0.6 { (pal.rock, rng.range(40.0, 56.0)) } else { (pal.tufts[rng.below(2) as usize], rng.range(24.0, 34.0)) };
                props.push(Prop { at: vec2(x, y), spr, size, flip: rng.chance(0.5) });
            }
        }
        props.sort_by(|a, b| b.at.y.partial_cmp(&a.at.y).unwrap());
        MapScreen { t: 0.0, scroll: -1.0, vel: 0.0, dragging: false, at, target, open: None, props, settled: false, prev_y: 0.0 }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        let cx = app.x0 + app.pw / 2.0;
        let view_h = app.h / u;
        let total = FOOT + BIOMES.len() as f32 * BAND + 130.0;
        let max_scroll = (total - view_h).max(0.0);
        let p = app.save.p().clone();
        let front = p.frontier();

        // the pack walks on
        if (self.target - self.at).abs() > 0.001 {
            self.at += (self.target - self.at).signum() * dt * 0.75;
            if (self.target - self.at).abs() < 0.02 {
                self.at = self.target;
            }
        } else if !self.settled {
            self.settled = true;
            if !p.finished() {
                self.open = Some(front);
            }
        }
        let token = trail(self.at);
        if self.scroll < 0.0 {
            self.scroll = (token.y - view_h * 0.42).clamp(0.0, max_scroll);
        }
        // follow the pack while it walks; otherwise the finger decides
        if !self.settled {
            let want = (token.y - view_h * 0.42).clamp(0.0, max_scroll);
            self.scroll += (want - self.scroll) * (dt * 4.0).min(1.0);
        }
        let sheet_h = if self.open.is_some() { 236.0 * u + app.bot } else { 0.0 };
        let ptr = &app.ui.ptr;
        let in_map = ptr.press_pos.y > app.top + 60.0 * u && ptr.press_pos.y < app.h - sheet_h;
        if ptr.down && in_map && ptr.travel > 8.0 * u {
            if !self.dragging {
                self.dragging = true;
            }
        }
        if !ptr.down {
            self.dragging = false;
        }
        // screen position of a map point
        let scroll = self.scroll;
        let at = |m: Vec2| vec2(cx + m.x * u, app.h - (m.y - scroll) * u);

        // ---- the land ----------------------------------------------------
        let g = &app.g;
        for b in 0..BIOMES.len() {
            let pal = palette(b);
            let y0 = at(vec2(0.0, FOOT + b as f32 * BAND)).y;
            let y1 = at(vec2(0.0, FOOT + (b + 1) as f32 * BAND)).y;
            if y0 < -10.0 || y1 > app.h + 10.0 {
                continue;
            }
            g.vgrad(Rect::new(app.x0, y1, app.pw, y0 - y1), pal.ground.0, pal.ground.1);
            // one biome shades into the next
            if b + 1 < BIOMES.len() {
                let next = palette(b + 1);
                g.vgrad(Rect::new(app.x0, y1 - 1.0, app.pw, 46.0 * u), next.ground.1, alpha(next.ground.1, 0.0));
            }
        }
        // below the first biome and above the last
        let foot = at(vec2(0.0, FOOT)).y;
        g.vgrad(Rect::new(app.x0, foot, app.pw, (app.h - foot).max(0.0) + 4.0), palette(0).ground.1, rgb(0x3f8a34));
        let top_y = at(vec2(0.0, FOOT + BIOMES.len() as f32 * BAND)).y;
        if top_y > 0.0 {
            g.vgrad(Rect::new(app.x0, 0.0, app.pw, top_y + 1.0), rgb(0x5a5650), palette(5).ground.0);
        }

        // the trail: pale where it has been walked
        let walked = if p.finished() { 18.0 } else { front as f32 };
        let mut s = -1.0;
        while s < LEVEL_COUNT as f32 - 0.2 {
            let c = at(trail(s));
            if c.y > -20.0 && c.y < app.h + 20.0 {
                let done = s <= walked;
                draw_circle(c.x, c.y, 8.5 * u, alpha(rgb(0x3a2a14), 0.35));
                draw_circle(c.x, c.y - 1.5 * u, 7.0 * u, if done { rgb(0xfff0c8) } else { alpha(rgb(0xfff0c8), 0.4) });
            }
            s += 0.085;
        }

        // scenery, homes and whoever lives in them
        for pr in &self.props {
            let c = at(pr.at);
            if c.y < -100.0 || c.y > app.h + 100.0 {
                continue;
            }
            let h = pr.size * u;
            g.spr(spr::SHADOW, c.x, c.y, h * 0.7, h * 0.22, 0.0, alpha(WHITE, 0.5));
            g.spr_ex(pr.spr, c.x, c.y - h * 0.42, h * pr.spr.w / pr.spr.h, h, 0.0, WHITE, pr.flip);
        }
        for b in 0..BIOMES.len() {
            let pal = palette(b);
            let last = b * PER_BIOME + PER_BIOME - 1;
            let n = node(last);
            let side = if n.x > 0.0 { -1.0 } else { 1.0 };
            let c = at(n + vec2(side * 108.0, -18.0));
            if c.y > -140.0 && c.y < app.h + 140.0 {
                let s = 128.0 * u;
                g.spr(pal.home, c.x, c.y, s, s, 0.0, WHITE);
                if let Some(who) = BIOMES[b].resident {
                    let home_now = p.is_friend(who) && !p.in_party(who);
                    let unmet = !p.is_friend(who);
                    if home_now || unmet {
                        let rs = 58.0 * u;
                        let x = c.x - side * 58.0 * u;
                        let wave = if home_now { (self.t * 3.0 + b as f32).sin() * 0.08 } else { 0.0 };
                        g.spr(spr::SHADOW, x, c.y + rs * 0.56, rs * 0.7, rs * 0.2, 0.0, alpha(WHITE, 0.5));
                        g.spr_ex(cr_spr(who), x, c.y + rs * 0.1, rs, rs, wave, if unmet { rgb(0x4a5a6a) } else { WHITE }, side > 0.0);
                    }
                }
            }
            // the biome's name where it begins
            let sign = at(vec2(0.0, FOOT + b as f32 * BAND + 52.0));
            if sign.y > -40.0 && sign.y < app.h + 40.0 {
                g.spr(spr::RIBBON, sign.x, sign.y, 250.0 * u, 70.0 * u, 0.0, pal.accent);
                g.label(BIOMES[b].name, sign.x, sign.y - 8.0 * u, 19.0 * u, 0.5, WHITE, Some((alpha(SOIL, 0.8), 1.8 * u)), 0.0);
            }
        }

        // ---- the stages --------------------------------------------------
        let mut tapped = None;
        for l in 0..LEVEL_COUNT {
            let c = at(node(l));
            if c.y < -60.0 || c.y > app.h + 60.0 {
                continue;
            }
            let open = p.unlocked(l);
            let here = l == front && !p.finished();
            let fin = is_finale(l);
            let pal = palette(l / PER_BIOME);
            let pulse = if here { 1.0 + 0.07 * (self.t * 4.0).sin() } else { 1.0 };
            let size = (if fin { 66.0 } else { 54.0 }) * u * pulse * if self.open == Some(l) { 1.12 } else { 1.0 };
            let g = &app.g;
            if here {
                g.spr(spr::GLOW, c.x, c.y, size * 2.2, size * 2.2, 0.0, alpha(rgb(0xfff07a), 0.6));
            }
            let color = if !open { rgb(0x8a9098) } else if here { rgb(0xffc83a) } else { pal.accent };
            g.spr_fit(spr::NODE, c.x, c.y, size, 0.0, color);
            if !open {
                g.spr_fit(spr::IC_LOCK, c.x, c.y - 2.0 * u, size * 0.45, 0.0, alpha(WHITE, 0.85));
            } else if fin {
                g.spr_fit(spr::IC_HOME, c.x, c.y - 3.0 * u, size * 0.5, 0.0, WHITE);
            } else {
                g.label(&(l + 1).to_string(), c.x, c.y - 3.0 * u, size * 0.42, 0.5, WHITE, Some((alpha(SOIL, 0.6), 1.5 * u)), 0.0);
            }
            if p.stars[l] > 0 {
                stars(g, c.x, c.y + size * 0.56, 15.0 * u, p.stars[l]);
            }
            let hit = Rect::new(c.x - 36.0 * u, c.y - 36.0 * u, 72.0 * u, 72.0 * u);
            if open {
                app.mark(&format!("stage{}", l + 1), hit);
            }
            if app.ui.tapped(hit) && in_map && !self.dragging {
                if open {
                    tapped = Some(l);
                } else {
                    app.au.play(Sfx::Error, 0.5);
                }
            }
        }
        if let Some(l) = tapped {
            self.open = Some(l);
            app.au.play(Sfx::Click, 0.8);
        }

        // ---- the pack on the trail ---------------------------------------
        let walking = (self.target - self.at).abs() > 0.001;
        let pack = p.pack();
        let g = &app.g;
        for (i, (who, _)) in pack.iter().enumerate().rev() {
            // followers trail a little way behind
            let behind = self.at - i as f32 * 0.22;
            // standing just beside the trail, so the stage marker stays in view
            let aside = if trail(self.at).x > 0.0 { -1.0 } else { 1.0 };
            let c = at(trail(behind)) + vec2(aside * (44.0 + i as f32 * 6.0) * u, if i == 0 { 0.0 } else { 10.0 * u });
            let size = (if i == 0 { 66.0 } else { 48.0 }) * u;
            let hop = if walking { (self.t * 11.0 + i as f32 * 1.3).sin().abs() * 9.0 * u } else { (self.t * 2.2 + i as f32).sin().abs() * 1.5 * u };
            let ahead = trail(behind + 0.1).x >= trail(behind).x;
            g.spr(spr::SHADOW, c.x, c.y - 16.0 * u, size * 0.7, size * 0.2, 0.0, alpha(WHITE, 0.6));
            g.spr_ex(cr_spr(*who), c.x, c.y - 16.0 * u - size * 0.44 - hop, size, size, 0.0, WHITE, !ahead);
        }

        // the finger scrolls the map, and it coasts when let go
        let y = app.ui.ptr.pos.y;
        if self.dragging {
            let moved = (y - self.prev_y) / u;
            self.scroll = (self.scroll + moved).clamp(0.0, max_scroll);
            self.vel = moved / dt.max(0.001);
            self.settled = true;
        } else {
            self.vel *= (-4.0 * dt).exp();
            if self.vel.abs() > 4.0 {
                self.scroll = (self.scroll + self.vel * dt).clamp(0.0, max_scroll);
            }
        }
        self.prev_y = y;

        // letterbox, then the top bar
        let g = &app.g;
        if app.x0 > 1.0 {
            g.rect(Rect::new(0.0, 0.0, app.x0, app.h), rgb(0x16240f));
            g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), rgb(0x16240f));
        }
        g.vgrad(Rect::new(app.x0, 0.0, app.pw, app.top + 78.0 * u), alpha(rgb(0x0c1408), 0.6), alpha(rgb(0x0c1408), 0.0));
        let mut nav = None;
        let by = app.top + 10.0 * u;
        let pack_btn = Rect::new(app.x0 + 10.0 * u, by, 124.0 * u, 46.0 * u);
        app.mark("pack", pack_btn);
        if app.ui.button(&app.g, &mut app.au, 1, pack_btn, PURPLE, "PACK", Some(spr::IC_PAW), true) {
            nav = Some(Nav::Pack);
        }
        // points waiting to be spent
        let waiting = p.skill_free() + if p.party.iter().any(|s| s.is_some()) { p.bond_free() } else { 0 };
        let g = &app.g;
        if waiting > 0 {
            let bx = pack_btn.x + pack_btn.w - 4.0 * u;
            let pop = 1.0 + 0.12 * (self.t * 5.0).sin();
            draw_circle(bx, by + 4.0 * u, 13.0 * u * pop, rgb(0xe8403a));
            g.label(&waiting.to_string(), bx, by + 4.0 * u, 14.0 * u, 0.5, WHITE, None, 0.0);
        }
        let mid = Rect::new(cx - 52.0 * u, by + 3.0 * u, 110.0 * u, 40.0 * u);
        chip(g, mid, 0.9);
        g.spr_fit(spr::IC_STAR, mid.x + 22.0 * u, mid.y + 19.0 * u, 26.0 * u, 0.0, WHITE);
        g.label(&format!("{} / {}", p.total_stars(), LEVEL_COUNT * 3), mid.x + 40.0 * u, mid.y + 20.0 * u, 16.0 * u, 0.0, WHITE, Some((alpha(SOIL, 0.8), 1.4 * u)), 0.0);
        let home = Rect::new(app.x0 + app.pw - 56.0 * u, by, 46.0 * u, 46.0 * u);
        if app.ui.button(&app.g, &mut app.au, 2, home, BLUE, "", Some(spr::IC_HOME), true) {
            nav = Some(Nav::Title);
        }
        if p.finished() && self.open.is_none() {
            let g = &app.g;
            let r = Rect::new(cx - 150.0 * u, app.h - app.bot - 74.0 * u, 300.0 * u, 54.0 * u);
            chip(g, r, 0.9);
            g.para("The road goes around now. Every home is standing.", cx, r.y + 9.0 * u, 13.0 * u, 270.0 * u, WHITE);
        }

        // ---- the stage card ----------------------------------------------
        if let Some(l) = self.open {
            let def = &LEVELS[l];
            let pal = palette(def.biome);
            let sheet = Rect::new(app.x0 + 6.0 * u, app.h - sheet_h + 4.0 * u, app.pw - 12.0 * u, sheet_h + 30.0 * u);
            panel(&app.g, sheet, u, WHITE);
            let g = &app.g;
            g.label(&format!("{}  -  stage {} of {}", BIOMES[def.biome].name, l % PER_BIOME + 1, PER_BIOME), cx, sheet.y + 22.0 * u, 12.5 * u, 0.5, pal.accent, Some((alpha(BARK, 0.9), 1.1 * u)), 0.0);
            g.label(def.name, cx, sheet.y + 46.0 * u, 26.0 * u, 0.5, BARK, None, 0.0);
            stars(g, cx, sheet.y + 76.0 * u, 24.0 * u, p.stars[l]);
            g.para(STAGE_LINES[l], cx, sheet.y + 94.0 * u, 14.0 * u, sheet.w - 50.0 * u, BARK);
            // who is coming
            let mut seen: Vec<biome_core::Foe> = def.foes.iter().map(|f| f.0).collect();
            if let Some(b) = def.boss {
                seen.push(b);
            }
            let n = seen.len() as f32;
            for (i, f) in seen.iter().enumerate() {
                let x = cx + (i as f32 - (n - 1.0) / 2.0) * 36.0 * u;
                g.spr_fit(foe_spr(*f), x, sheet.y + 148.0 * u, if f.def().boss { 40.0 } else { 30.0 } * u, 0.0, WHITE);
            }
            let by = sheet.y + 172.0 * u;
            let diff = Rect::new(sheet.x + 14.0 * u, by, 112.0 * u, 50.0 * u);
            let go = Rect::new(sheet.x + 134.0 * u, by, sheet.w - 148.0 * u, 50.0 * u);
            let d = p.difficulty;
            app.mark("difficulty", diff);
            if app.ui.button(&app.g, &mut app.au, 5, diff, [rgb(0x4ab86a), BLUE, rgb(0xd0503a)][d.index()], d.name(), None, true) {
                app.save.p_mut().difficulty = Difficulty::from_index((d.index() + 1) % 3);
                app.save.store();
            }
            app.mark("defend", go);
            if app.ui.button(&app.g, &mut app.au, 6, go, GREEN, if p.stars[l] > 0 { "DEFEND AGAIN" } else { "DEFEND" }, Some(spr::IC_PLAY), true) {
                nav = Some(Nav::Play(l));
            }
            // a tap on the map away from any stage closes the card
            let ptr = &app.ui.ptr;
            if ptr.released && ptr.travel < 8.0 * u && ptr.pos.y < sheet.y && ptr.press_pos.y > app.top + 60.0 * u && tapped.is_none() {
                self.open = None;
            }
        }
        nav
    }
}
