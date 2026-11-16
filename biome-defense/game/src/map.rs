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
use biome_core::{BIOMES, LEVEL_COUNT};
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
        MapScreen { t: 0.0, scroll: -1.0, vel: 0.0, dragging: false, at, target, open: None, settled: false, prev_y: 0.0 }
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
        // each biome from high above, shading into the next
        let g = &app.g;
        g.rect(Rect::new(0.0, 0.0, app.w, app.h), SHADE);
        let lap = 60.0 * u;
        for b in 0..BIOMES.len() {
            let pal = palette(b);
            let y0 = at(vec2(0.0, FOOT + b as f32 * BAND)).y;
            let y1 = at(vec2(0.0, FOOT + (b + 1) as f32 * BAND)).y;
            if y0 < -lap - 10.0 || y1 > app.h + lap + 10.0 {
                continue;
            }
            let whole = Rect::new(app.x0, y1 - lap, app.pw, y0 - y1 + 2.0 * lap);
            let tint = rgb(0xb4b4b0);
            // solid down to just above where the biome below takes over, then
            // thinning out over it
            g.scene_rows(pal.map, whole, y1 - lap, y0 - lap, 1.0, 1.0, tint);
            g.scene_rows(pal.map, whole, y0 - lap, y0 + lap, 1.0, if b == 0 { 1.0 } else { 0.0 }, tint);
        }
        // below the first biome and above the last
        let foot = at(vec2(0.0, FOOT)).y;
        g.fade(Rect::new(app.x0, foot - 10.0 * u, app.pw, 70.0 * u), SHADE, 0.0, 1.0);
        let top_y = at(vec2(0.0, FOOT + BIOMES.len() as f32 * BAND)).y;
        if top_y > -lap {
            g.fade(Rect::new(app.x0, top_y - lap - 2.0, app.pw, lap * 1.6), SHADE, 1.0, 0.0);
        }

        // the trail: bright where it has been walked
        let walked = if p.finished() { 18.0 } else { front as f32 };
        let mut s = -1.0;
        while s < LEVEL_COUNT as f32 - 0.2 {
            let c = at(trail(s));
            if c.y > -20.0 && c.y < app.h + 20.0 {
                let done = s <= walked;
                draw_circle(c.x, c.y + 1.0 * u, 4.6 * u, alpha(SHADE, 0.6));
                draw_circle(c.x, c.y, 3.2 * u, if done { INK } else { alpha(INK, 0.3) });
            }
            s += 0.07;
        }

        // each biome's name where it begins, and whoever lives at the end of it
        for b in 0..BIOMES.len() {
            let pal = palette(b);
            let last = b * PER_BIOME + PER_BIOME - 1;
            let n = node(last);
            let side = if n.x > 0.0 { -1.0 } else { 1.0 };
            let c = at(n + vec2(side * 96.0, 4.0));
            if c.y > -140.0 && c.y < app.h + 140.0 {
                if let Some(who) = BIOMES[b].resident {
                    let home_now = p.is_friend(who) && !p.in_party(who);
                    let unmet = !p.is_friend(who);
                    if home_now || unmet {
                        let rs = 54.0 * u;
                        portrait(g, cr_face(who), c.x, c.y, rs, if unmet { alpha(INK, 0.25) } else { cr_color(who) }, if unmet { rgb(0x424242) } else { WHITE });
                        g.head(if unmet { "?" } else { who.def().name }, c.x, c.y + rs * 0.5 + 12.0 * u, 10.0 * u, 0.5, if unmet { DIM } else { INK });
                    }
                }
            }
            let sign = at(vec2(0.0, FOOT + b as f32 * BAND + 52.0));
            if sign.y > -40.0 && sign.y < app.h + 40.0 {
                g.nine(spr::PLATE, Rect::new(app.x0, sign.y - 22.0 * u, app.pw, 44.0 * u), 8.0, 8.0 * u, WHITE);
                g.spaced(&BIOMES[b].name.to_uppercase(), sign.x, sign.y, 17.0 * u, 0.22, pal.accent, None);
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
            let size = (if fin { 62.0 } else { 50.0 }) * u * if self.open == Some(l) { 1.1 } else { 1.0 };
            let g = &app.g;
            if here {
                let pulse = 0.5 + 0.5 * (self.t * 3.0).sin();
                g.spr(spr::RIM, c.x, c.y, size * (1.25 + 0.12 * pulse), size * (1.25 + 0.12 * pulse), 0.0, alpha(GOLD, 0.9 - 0.5 * pulse));
            }
            let color = if !open { rgb(0x2e302c) } else if here { rgb(0x8a6a2a) } else { mix(pal.accent, SHADE, 0.5) };
            g.spr_fit(spr::NODE, c.x, c.y, size, 0.0, color);
            if !open {
                g.spr_fit(spr::IC_LOCK, c.x, c.y, size * 0.4, 0.0, alpha(INK, 0.4));
            } else if fin {
                g.spr_fit(spr::IC_HOME, c.x, c.y - 1.0 * u, size * 0.46, 0.0, INK);
            } else {
                g.head(&(l + 1).to_string(), c.x, c.y, size * 0.36, 0.5, INK);
            }
            if p.stars[l] > 0 {
                stars(g, c.x, c.y + size * 0.6, 11.0 * u, p.stars[l]);
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
            let behind = self.at - i as f32 * 0.24;
            // just beside the trail, so the stage marker stays in view
            let aside = if trail(self.at).x > 0.0 { -1.0 } else { 1.0 };
            let c = at(trail(behind)) + vec2(aside * (50.0 + i as f32 * 4.0) * u, if i == 0 { 0.0 } else { 8.0 * u });
            let size = (if i == 0 { 52.0 } else { 38.0 }) * u;
            let hop = if walking { (self.t * 9.0 + i as f32 * 1.3).sin().abs() * 4.0 * u } else { 0.0 };
            g.spr(spr::SHADOW, c.x, c.y + size * 0.52, size * 0.9, size * 0.26, 0.0, alpha(WHITE, 0.8));
            portrait(g, cr_face(*who), c.x, c.y - hop, size, cr_color(*who), WHITE);
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
        crate::title::letterbox(app);
        let g = &app.g;
        g.fade(Rect::new(app.x0, 0.0, app.pw, app.top + 84.0 * u), SHADE, 0.85, 0.0);
        let mut nav = None;
        let by = app.top + 10.0 * u;
        let pack_btn = Rect::new(app.x0 + 10.0 * u, by, 118.0 * u, 44.0 * u);
        app.mark("pack", pack_btn);
        if app.ui.button(&app.g, &mut app.au, 1, pack_btn, STEEL, "PACK", Some(spr::IC_PAW), true) {
            nav = Some(Nav::Pack);
        }
        // points waiting to be spent
        let waiting = p.skill_free() + if p.party.iter().any(|s| s.is_some()) { p.bond_free() } else { 0 };
        let g = &app.g;
        if waiting > 0 {
            let bx = pack_btn.x + pack_btn.w - 3.0 * u;
            draw_circle(bx, by + 3.0 * u, 12.0 * u, SHADE);
            draw_circle(bx, by + 3.0 * u, 10.5 * u, GOLD);
            g.label(&waiting.to_string(), bx, by + 3.0 * u, 13.0 * u, 0.5, SHADE, None, 0.0);
        }
        let mid = Rect::new(cx - 50.0 * u, by + 3.0 * u, 106.0 * u, 38.0 * u);
        chip(g, mid, 1.0);
        g.spr_fit(spr::IC_STAR, mid.x + 20.0 * u, mid.y + 19.0 * u, 17.0 * u, 0.0, GOLD);
        g.label(&format!("{} / {}", p.total_stars(), LEVEL_COUNT * 3), mid.x + 36.0 * u, mid.y + 19.0 * u, 15.0 * u, 0.0, INK, None, 0.0);
        let home = Rect::new(app.x0 + app.pw - 54.0 * u, by, 44.0 * u, 44.0 * u);
        if app.ui.button(&app.g, &mut app.au, 2, home, GREY, "", Some(spr::IC_HOME), true) {
            nav = Some(Nav::Title);
        }
        if p.finished() && self.open.is_none() {
            let g = &app.g;
            let r = Rect::new(cx - 150.0 * u, app.h - app.bot - 74.0 * u, 300.0 * u, 54.0 * u);
            chip(g, r, 1.0);
            g.para("The road goes around now. Every home is standing.", cx, r.y + 9.0 * u, 13.0 * u, 270.0 * u, INK);
        }

        // ---- the stage card ----------------------------------------------
        if let Some(l) = self.open {
            let def = &LEVELS[l];
            let pal = palette(def.biome);
            let sheet = Rect::new(app.x0 + 6.0 * u, app.h - sheet_h + 4.0 * u, app.pw - 12.0 * u, sheet_h + 30.0 * u);
            panel(&app.g, sheet, u, WHITE);
            let g = &app.g;
            g.head(&format!("{}  -  STAGE {} OF {}", BIOMES[def.biome].name.to_uppercase(), l % PER_BIOME + 1, PER_BIOME), cx, sheet.y + 22.0 * u, 10.5 * u, 0.5, pal.accent);
            g.head(&def.name.to_uppercase(), cx, sheet.y + 46.0 * u, 22.0 * u, 0.5, INK);
            stars(g, cx, sheet.y + 74.0 * u, 15.0 * u, p.stars[l]);
            g.para(STAGE_LINES[l], cx, sheet.y + 92.0 * u, 14.0 * u, sheet.w - 50.0 * u, rgb(0xcfc8b6));
            // who is coming
            let mut seen: Vec<biome_core::Foe> = def.foes.iter().map(|f| f.0).collect();
            if let Some(b) = def.boss {
                seen.push(b);
            }
            let n = seen.len() as f32;
            for (i, f) in seen.iter().enumerate() {
                let x = cx + (i as f32 - (n - 1.0) / 2.0) * 38.0 * u;
                let rot = if foe_pose(*f) == Pose::Above { -std::f32::consts::FRAC_PI_2 } else { 0.0 };
                g.spr_fit(foe_spr(*f), x, sheet.y + 148.0 * u, if f.def().boss { 42.0 } else { 32.0 } * u, rot, WHITE);
            }
            let by = sheet.y + 174.0 * u;
            let diff = Rect::new(sheet.x + 14.0 * u, by, 112.0 * u, 50.0 * u);
            let go = Rect::new(sheet.x + 134.0 * u, by, sheet.w - 148.0 * u, 50.0 * u);
            let d = p.difficulty;
            app.mark("difficulty", diff);
            if app.ui.button(&app.g, &mut app.au, 5, diff, [rgb(0x4a5a3a), STEEL, RUST][d.index()], &d.name().to_uppercase(), None, true) {
                app.save.p_mut().difficulty = Difficulty::from_index((d.index() + 1) % 3);
                app.save.store();
            }
            app.mark("defend", go);
            if app.ui.button(&app.g, &mut app.au, 6, go, GO, if p.stars[l] > 0 { "DEFEND AGAIN" } else { "DEFEND" }, Some(spr::IC_PLAY), true) {
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
