//! The pack: who is walking with you, who is waiting at home, and what
//! everyone has learned.
//!
//! Your own creature's skill points are spent for good. Followers share a
//! pool of bond points, and a follower who goes home hands theirs back.

use crate::art::*;
use crate::audio::{Sfx, Track};
use crate::fx::Fx;
use crate::gfx::*;
use crate::ui::*;
use crate::widgets::*;
use crate::{App, Nav};
use biome_core::campaign::Refusal;
use biome_core::creature::{skill_cost, spent, Cr};
use macroquad::prelude::{draw_line, vec2, Rect};

pub struct Pack {
    t: f32,
    /// Whose tree is on show.
    who: Cr,
    /// The skill being looked at: (branch, tier).
    skill: Option<(usize, usize)>,
    /// A friend at home is being brought along and the pack is full: tap
    /// who they replace.
    swapping: bool,
    fx: Fx,
    learned: f32,
}

impl Pack {
    pub fn new(app: &mut App) -> Pack {
        app.au.track(Track::Trail);
        let p = app.save.p();
        app.log(&format!("SCREEN pack skill free {} bond free {} walking {:?}", p.skill_free(), p.bond_free(), p.party));
        let who = p.hero;
        Pack { t: 0.0, who, skill: None, swapping: false, fx: Fx::new(app.rng.next_u64()), learned: 0.0 }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        self.learned = (self.learned - dt * 2.0).max(0.0);
        self.fx.update(dt);
        let cx = app.x0 + app.pw / 2.0;
        let p = app.save.p().clone();
        let g = &app.g;
        g.vgrad(Rect::new(0.0, 0.0, app.w, app.h), rgb(0x2f5a3a), rgb(0x1a3324));
        // a scatter of leaves behind everything
        for k in 0..14 {
            let x = app.x0 + ((k * 73 + 11) % 100) as f32 / 100.0 * app.pw;
            let y = ((k * 131 + 40) % 100) as f32 / 100.0 * app.h + (self.t * 0.4 + k as f32).sin() * 6.0 * u;
            g.spr_fit(spr::P_LEAF, x, y, (26.0 + (k % 3) as f32 * 10.0) * u, k as f32 * 0.9, alpha(rgb(0x4a8a4a), 0.35));
        }
        if app.x0 > 1.0 {
            g.rect(Rect::new(0.0, 0.0, app.x0, app.h), rgb(0x16240f));
            g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), rgb(0x16240f));
        }

        // ---- header ---------------------------------------------------------
        let mut nav = None;
        let top = app.top + 8.0 * u;
        let back = Rect::new(app.x0 + 10.0 * u, top, 46.0 * u, 46.0 * u);
        app.mark("back", back);
        if app.ui.button(&app.g, &mut app.au, 1, back, BLUE, "", Some(spr::IC_BACK), true) {
            nav = Some(Nav::Map);
        }
        let g = &app.g;
        g.title("THE PACK", cx - 40.0 * u, top + 23.0 * u, 26.0 * u, rgb(0xfff2a8), rgb(0x16240f));
        // the two purses
        for (i, (label, n, color)) in [("skill", p.skill_free(), rgb(0xfff07a)), ("bond", p.bond_free(), rgb(0x9fe0ff))].into_iter().enumerate() {
            let r = Rect::new(app.x0 + app.pw - 148.0 * u + i as f32 * 72.0 * u, top + 2.0 * u, 68.0 * u, 42.0 * u);
            chip(g, r, 0.9);
            let pop = if n > 0 { 1.0 + 0.08 * (self.t * 5.0 + i as f32).sin() } else { 1.0 };
            g.label(&n.to_string(), r.x + r.w / 2.0, r.y + 15.0 * u, 20.0 * u * pop, 0.5, color, Some((alpha(SOIL, 0.8), 1.5 * u)), 0.0);
            g.label(label, r.x + r.w / 2.0, r.y + 33.0 * u, 10.0 * u, 0.5, alpha(WHITE, 0.8), None, 0.0);
        }

        // ---- who walks with you ---------------------------------------------
        let row = top + 60.0 * u;
        let mut pick = None;
        let mut swap_into = None;
        let slots: [Option<Cr>; 3] = [p.party[0], Some(p.hero), p.party[1]];
        for (i, slot) in slots.iter().enumerate() {
            let lead = i == 1;
            let size = (if lead { 100.0 } else { 80.0 }) * u;
            let x = cx + (i as f32 - 1.0) * 122.0 * u;
            let y = row + 56.0 * u;
            let r = Rect::new(x - 56.0 * u, row, 112.0 * u, 112.0 * u);
            let g = &app.g;
            match slot {
                Some(who) => {
                    let chosen = *who == self.who;
                    g.spr(spr::NODE, x, y, size * 1.16, size * 1.16, 0.0, if chosen { cr_color(*who) } else { mix(cr_color(*who), rgb(0x1a3324), 0.55) });
                    if chosen {
                        g.spr(spr::RING, x, y - 2.0 * u, size * 1.36, size * 1.36, 0.0, alpha(WHITE, 0.9));
                    }
                    g.spr_fit(cr_spr(*who), x, y - 5.0 * u, size * 0.9, 0.0, WHITE);
                    g.label(who.def().name, x, y + size * 0.62, 13.0 * u, 0.5, WHITE, Some((alpha(SOIL, 0.8), 1.3 * u)), 0.0);
                    if self.swapping && !lead {
                        let bob = (self.t * 6.0).sin().abs() * 5.0 * u;
                        g.spr_fit(spr::IC_NEXT, x, y - size * 0.75 - bob, 26.0 * u, std::f32::consts::FRAC_PI_2, rgb(0xfff07a));
                    }
                    if app.ui.tapped(r) {
                        if self.swapping && !lead {
                            swap_into = Some(if i == 0 { 0 } else { 1 });
                        } else {
                            pick = Some(*who);
                        }
                    }
                }
                None => {
                    g.spr(spr::NODE, x, y, size * 1.1, size * 1.1, 0.0, alpha(rgb(0x4a6a52), 0.7));
                    g.label("room", x, y - 8.0 * u, 13.0 * u, 0.5, alpha(WHITE, 0.6), None, 0.0);
                    g.label("for one", x, y + 8.0 * u, 13.0 * u, 0.5, alpha(WHITE, 0.6), None, 0.0);
                }
            }
            app.mark(&format!("slot{i}"), r);
        }

        // ---- who is waiting at home -------------------------------------------
        let home_y = row + 132.0 * u;
        let waiting = p.at_home();
        let g = &app.g;
        g.label("AT HOME", app.x0 + 14.0 * u, home_y + 22.0 * u, 11.0 * u, 0.0, alpha(WHITE, 0.6), None, 0.0);
        if waiting.is_empty() {
            let text = if p.friends == 0 { "Hold a home and whoever lives there may come along." } else { "Everyone you have met is walking with you." };
            g.label(text, app.x0 + 74.0 * u, home_y + 22.0 * u, 11.5 * u, 0.0, alpha(WHITE, 0.7), None, 0.0);
        }
        for (i, who) in waiting.iter().enumerate() {
            let x = app.x0 + 100.0 * u + i as f32 * 62.0 * u;
            let y = home_y + 22.0 * u;
            let chosen = *who == self.who;
            let r = Rect::new(x - 29.0 * u, y - 26.0 * u, 58.0 * u, 54.0 * u);
            let g = &app.g;
            g.spr(spr::NODE, x, y, 54.0 * u, 54.0 * u, 0.0, if chosen { cr_color(*who) } else { mix(cr_color(*who), rgb(0x1a3324), 0.6) });
            g.spr_fit(cr_spr(*who), x, y - 3.0 * u, 42.0 * u, 0.0, WHITE);
            if chosen {
                g.spr(spr::RING, x, y - 1.0 * u, 62.0 * u, 62.0 * u, 0.0, alpha(WHITE, 0.9));
            }
            app.mark(&format!("home{i}"), r);
            if app.ui.tapped(r) {
                pick = Some(*who);
            }
        }
        if let Some(who) = pick {
            self.who = who;
            self.skill = None;
            self.swapping = false;
            app.au.play(Sfx::Click, 0.8);
        }
        if let Some(slot) = swap_into {
            app.save.p_mut().join(self.who, slot);
            app.save.store();
            self.swapping = false;
            app.au.play(Sfx::Join, 0.8);
            app.log(&format!("JOINED {:?} slot {} bond free {}", self.who, slot, app.save.p().bond_free()));
            return None;
        }

        // ---- the creature on show ----------------------------------------------
        let who = self.who;
        let d = who.def();
        let is_hero = who == p.hero;
        let walking = p.in_party(who);
        let info = Rect::new(app.x0 + 8.0 * u, home_y + 54.0 * u, app.pw - 16.0 * u, 86.0 * u);
        panel(&app.g, info, u, WHITE);
        let g = &app.g;
        g.label(&format!("{} the {}", d.name, d.species), info.x + 16.0 * u, info.y + 20.0 * u, 17.0 * u, 0.0, cr_color(who), Some((BARK, 1.4 * u)), 0.0);
        let role = if is_hero { format!("{}  -  you", d.role) } else { d.role.to_string() };
        g.label(&role, info.x + 16.0 * u, info.y + 39.0 * u, 11.5 * u, 0.0, alpha(BARK, 0.75), None, 0.0);
        let ability = format!("{}: {}", d.ability_name, d.ability_text);
        for (i, line) in g.wrap(&ability, 11.0 * u, info.w - 150.0 * u).iter().take(3).enumerate() {
            g.label(line, info.x + 16.0 * u, info.y + 54.0 * u + i as f32 * 12.5 * u, 11.0 * u, 0.0, BARK, None, 0.0);
        }
        let side = Rect::new(info.x + info.w - 126.0 * u, info.y + 14.0 * u, 114.0 * u, 44.0 * u);
        if !is_hero {
            if walking {
                app.mark("sendhome", side);
                if app.ui.button(&app.g, &mut app.au, 5, side, ORANGE, "SEND HOME", None, true) {
                    if let Some(slot) = p.party.iter().position(|s| *s == Some(who)) {
                        app.save.p_mut().dismiss(slot);
                        app.save.store();
                        app.au.play(Sfx::Sell, 0.8);
                        app.log(&format!("DISMISSED {who:?} bond free {}", app.save.p().bond_free()));
                    }
                }
                let back = spent(who, p.skills_of(who));
                app.g.label(&format!("gives back {back} bond"), side.x + side.w / 2.0, side.y + 56.0 * u, 10.0 * u, 0.5, alpha(BARK, 0.7), None, 0.0);
            } else {
                app.mark("bring", side);
                let label = if self.swapping { "TAP WHO" } else { "BRING ALONG" };
                if app.ui.button(&app.g, &mut app.au, 6, side, GREEN, label, None, true) {
                    match p.free_slot() {
                        Some(slot) => {
                            app.save.p_mut().join(who, slot);
                            app.save.store();
                            app.au.play(Sfx::Join, 0.8);
                            app.log(&format!("JOINED {who:?} slot {slot} bond free {}", app.save.p().bond_free()));
                        }
                        None => self.swapping = !self.swapping,
                    }
                }
                let note = if self.swapping { "tap who goes home" } else if p.free_slot().is_some() { "there is room" } else { "someone goes home" };
                app.g.label(note, side.x + side.w / 2.0, side.y + 56.0 * u, 10.0 * u, 0.5, alpha(BARK, 0.7), None, 0.0);
            }
        } else {
            g.spr_fit(ability_icon(who), side.x + side.w / 2.0, side.y + 28.0 * u, 50.0 * u, 0.0, cr_color(who));
        }

        // ---- the tree ------------------------------------------------------------
        let branches = d.tree.len();
        let tiers = who.tiers();
        let detail_h = 124.0 * u;
        let tree_top = info.y + info.h + 8.0 * u;
        let tree_bottom = app.h - app.bot - detail_h - 12.0 * u;
        let step = ((tree_bottom - tree_top - 34.0 * u) / tiers as f32).clamp(40.0 * u, 62.0 * u);
        let node = (step * 0.84).min(52.0 * u);
        let col_w = (app.pw - 20.0 * u) / branches as f32;
        let mut tapped = None;
        for b in 0..branches {
            let bx = app.x0 + 10.0 * u + col_w * (b as f32 + 0.5);
            let color = BRANCH_COLORS[b % 3];
            let br = &d.tree[b];
            let g = &app.g;
            g.label(br.name, bx, tree_top + 10.0 * u, 15.0 * u, 0.5, color, Some((alpha(SOIL, 0.8), 1.4 * u)), 0.0);
            g.label(br.about, bx, tree_top + 25.0 * u, 10.0 * u, 0.5, alpha(WHITE, 0.65), None, 0.0);
            for t in 0..tiers {
                let y = tree_top + 34.0 * u + step * (t as f32 + 0.5);
                let known = p.knows(who, b, t);
                let can = p.can_learn(who, b, t);
                if t + 1 < tiers {
                    draw_line(bx, y + node * 0.4, bx, y + step - node * 0.4, 5.0 * u, if known && p.knows(who, b, t + 1) { color } else { alpha(rgb(0x0c1408), 0.5) });
                }
                let (skill, _) = who.skill(b, t);
                let chosen = self.skill == Some((b, t));
                let ready = can.is_ok();
                let pulse = if ready { 1.0 + 0.05 * (self.t * 5.0 + b as f32).sin() } else { 1.0 };
                let s = node * pulse * if chosen { 1.12 } else { 1.0 };
                let g = &app.g;
                if ready {
                    g.spr(spr::GLOW, bx, y, s * 1.9, s * 1.9, 0.0, alpha(color, 0.55));
                }
                let fill = if known { color } else if ready { mix(color, rgb(0x2a3a2a), 0.45) } else { rgb(0x4a5a52) };
                g.spr_fit(spr::NODE, bx, y, s, 0.0, fill);
                g.spr_fit(skill_icon(skill.icon), bx, y - s * 0.04, s * 0.52, 0.0, if known || ready { WHITE } else { alpha(WHITE, 0.45) });
                if known {
                    g.spr_fit(spr::IC_CHECK, bx + s * 0.34, y + s * 0.3, s * 0.34, 0.0, rgb(0xb8ff8a));
                } else {
                    // what it costs
                    let cost = skill_cost(t);
                    g.label(&cost.to_string(), bx + s * 0.38, y + s * 0.32, 12.0 * u, 0.5, if ready { rgb(0xfff07a) } else { alpha(WHITE, 0.6) }, Some((alpha(SOIL, 0.9), 1.2 * u)), 0.0);
                }
                if chosen {
                    g.spr(spr::RING, bx, y, s * 1.3, s * 1.3, 0.0, WHITE);
                }
                let hit = Rect::new(bx - col_w * 0.42, y - step * 0.48, col_w * 0.84, step * 0.96);
                app.mark(&format!("skill{b}{t}"), hit);
                if app.ui.tapped(hit) {
                    tapped = Some((b, t));
                }
            }
        }
        if let Some(sel) = tapped {
            self.skill = Some(sel);
            app.au.play(Sfx::Click, 0.7);
        }

        // ---- the skill being looked at ---------------------------------------------
        let detail = Rect::new(app.x0 + 8.0 * u, app.h - app.bot - detail_h - 6.0 * u, app.pw - 16.0 * u, detail_h);
        panel(&app.g, detail, u, WHITE);
        let g = &app.g;
        match self.skill {
            None => {
                let text = if is_hero {
                    "Tap a skill to read it. Every stage held for the first time earns one skill point. Each skill costs one more than the one before it, and there are not enough points for everything: choose a branch. Skills are for keeps."
                } else {
                    "Followers learn with bond points, one earned per stage held. A follower who goes home gives theirs back, to spend on whoever walks with you next."
                };
                g.para(text, cx, detail.y + 14.0 * u, 12.0 * u, detail.w - 36.0 * u, BARK);
            }
            Some((b, t)) => {
                let (skill, _) = who.skill(b, t);
                let color = BRANCH_COLORS[b % 3];
                g.spr_fit(spr::NODE, detail.x + 34.0 * u, detail.y + 36.0 * u, 48.0 * u, 0.0, color);
                g.spr_fit(skill_icon(skill.icon), detail.x + 34.0 * u, detail.y + 34.0 * u, 26.0 * u, 0.0, WHITE);
                g.label(skill.name, detail.x + 66.0 * u, detail.y + 22.0 * u, 17.0 * u, 0.0, BARK, None, 0.0);
                let cost = skill_cost(t);
                let purse = if is_hero { "skill" } else { "bond" };
                g.label(&format!("{} {} point{}", cost, purse, if cost == 1 { "" } else { "s" }), detail.x + detail.w - 16.0 * u, detail.y + 22.0 * u, 12.0 * u, 1.0, alpha(BARK, 0.7), None, 0.0);
                for (i, line) in g.wrap(skill.text, 12.5 * u, detail.w - 90.0 * u).iter().take(3).enumerate() {
                    g.label(line, detail.x + 66.0 * u, detail.y + 42.0 * u + i as f32 * 14.5 * u, 12.5 * u, 0.0, BARK, None, 0.0);
                }
                let state = p.can_learn(who, b, t);
                let btn = Rect::new(detail.x + detail.w - 150.0 * u, detail.y + detail_h - 50.0 * u, 138.0 * u, 42.0 * u);
                let note = |text: &str| g.label(text, detail.x + 18.0 * u, detail.y + detail_h - 28.0 * u, 12.0 * u, 0.0, alpha(BARK, 0.75), None, 0.0);
                match state {
                    Ok(()) => {
                        note(if is_hero { "For keeps." } else { "Comes back if they go home." });
                        app.mark("learn", btn);
                        if app.ui.button(&app.g, &mut app.au, 9, btn, GREEN, "LEARN", Some(spr::IC_CHECK), true) && app.save.p_mut().learn(who, b, t).is_ok() {
                            app.save.store();
                            app.au.play(Sfx::Learn, 0.9);
                            self.learned = 1.0;
                            let bx = app.x0 + 10.0 * u + col_w * (b as f32 + 0.5);
                            let y = tree_top + 34.0 * u + step * (t as f32 + 0.5);
                            self.fx.sparkle(vec2(bx, y), node * 0.6, 14, 10.0 * u, color);
                            self.fx.ring(vec2(bx, y), node * 0.4, node * 1.4, 0.4, color, 0.0);
                            let p = app.save.p();
                            app.log(&format!("LEARNED {} {} skill free {} bond free {}", who.def().name, skill.name, p.skill_free(), p.bond_free()));
                        }
                    }
                    Err(Refusal::AlreadyKnown) => note("Learned."),
                    Err(Refusal::OutOfOrder) => note(&format!("Learn {} first.", who.skill(b, t - 1).0.name)),
                    Err(Refusal::NotEnoughPoints) => note(&format!("Needs {} {} point{}. Hold more stages.", cost, purse, if cost == 1 { "" } else { "s" })),
                    Err(Refusal::NotInPack) => note(&format!("{} has to be walking with you to learn.", d.name)),
                }
            }
        }
        self.fx.draw(&app.g);
        nav
    }
}
