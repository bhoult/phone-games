//! The title screen: the logo, a rain of everything Rufus cares about, and
//! Rufus himself, who can be booped.

use crate::audio::Sfx;
use crate::content::*;
use crate::fx::Fx;
use crate::gfx::*;
use crate::play::sound_toggles;
use crate::rufus::{Mood, Rufus};
use crate::ui::*;
use crate::{App, Nav};
use macroquad::prelude::{vec2, Rect, Vec2};
use rufus_core::{Rng, KINDS, LEVELS};

struct Drop {
    kind: u8,
    pos: Vec2,
    vel: f32,
    rot: f32,
    spin: f32,
    size: f32,
}

pub struct Title {
    t: f32,
    rufus: Rufus,
    fx: Fx,
    rain: Vec<Drop>,
    rng: Rng,
    boop_cd: f32,
}

impl Title {
    pub fn new(app: &mut App) -> Title {
        if app.debug {
            macroquad::logging::info!("RUFUS SCREEN title");
        }
        let mut rng = Rng::new(app.rng.next_u64());
        let rain = (0..16)
            .map(|i| Drop {
                kind: (i % KINDS) as u8,
                pos: vec2(rng.unit(), rng.range(-0.2, 1.0)),
                vel: rng.range(0.07, 0.15),
                rot: rng.range(0.0, 6.0),
                spin: rng.range(-1.2, 1.2),
                size: rng.range(34.0, 52.0),
            })
            .collect();
        let mut rufus = Rufus::new(rng.next_u64());
        rufus.rest(Mood::Idle);
        rufus.react(Mood::Happy, 2.0);
        Title { t: 0.0, rufus, fx: Fx::new(rng.next_u64()), rain, rng, boop_cd: 0.0 }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        self.boop_cd -= dt;
        self.fx.update(dt);
        self.rufus.update(dt, &mut self.fx);
        let cx = app.x0 + app.pw / 2.0;
        let th = theme(0);
        let g = &app.g;

        g.vgrad(Rect::new(0.0, 0.0, app.w, app.h), th.top, th.bottom);
        let scene_h = app.h * 0.46;
        crate::ambient::pools(g, Rect::new(app.x0, scene_h, app.pw, app.h - scene_h), self.t, u, mix(th.top, WHITE, 0.45));
        g.scene(th.scene, Rect::new(app.x0, 0.0, app.pw, scene_h), WHITE);
        crate::ambient::draw(g, th.scene, Rect::new(app.x0, 0.0, app.pw, scene_h), self.t, u);
        if app.x0 > 1.0 {
            g.rect(Rect::new(0.0, 0.0, app.x0, app.h), alpha(rgb(0x000000), 0.28));
            g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), alpha(rgb(0x000000), 0.28));
        }

        // it's raining squirrels
        for d in &mut self.rain {
            d.pos.y += d.vel * dt;
            d.rot += d.spin * dt;
            if d.pos.y > 1.12 {
                d.pos = vec2(self.rng.unit(), -0.12);
                d.kind = self.rng.below(KINDS as u32) as u8;
            }
            let x = app.x0 + d.pos.x * app.pw;
            let y = d.pos.y * app.h;
            // keep them clear of the sky so the logo reads
            let a = ((y - scene_h * 0.72) / (scene_h * 0.2)).clamp(0.0, 1.0) * 0.9;
            g.spr_fit(kind_spr(d.kind), x, y, d.size * u, d.rot, alpha(WHITE, a));
        }

        // Rufus, front and centre
        let ground = (app.h * 0.685).min(app.h - app.bot - 214.0 * u);
        let rufus_h = (app.h * 0.31).min(250.0 * u);
        g.spr(spr::GLOW, cx, ground - rufus_h * 0.5, rufus_h * 2.2, rufus_h * 1.7, 0.0, alpha(rgb(0xfff6b8), 0.5));
        g.lights(|| {
            let k = if g.has_light() { 1.0 } else { 0.4 };
            g.spr(spr::RAYS, cx, ground - rufus_h * 0.5, rufus_h * 2.9, rufus_h * 2.9, self.t * 0.22, alpha(rgb(0xfff0a0), 0.42 * k));
            g.spr(spr::RAYS, cx, ground - rufus_h * 0.5, rufus_h * 2.1, rufus_h * 2.1, -self.t * 0.15, alpha(WHITE, 0.3 * k));
        });
        self.rufus.draw(g, cx, ground, rufus_h);

        // logo: RUFUS on a bone, CRUSH bouncing underneath
        let logo_y = app.top + (app.h * 0.115).max(74.0 * u);
        let drop_in = ease_back((self.t / 0.6).min(1.0));
        let bone_w = 344.0 * u * drop_in;
        // the whole logo catches the gleam
        g.shine_on(self.t + 3.4);
        g.spr(spr::LOGO_BONE, cx, logo_y, bone_w, bone_w * 0.375, -0.035, WHITE);
        g.label("RUFUS", cx, logo_y - 3.0 * u, 62.0 * u * drop_in, 0.5, rgb(0xe8323c), Some((rgb(0x5a1410), 3.0 * u)), -0.035);
        let word = "CRUSH";
        let size = 70.0 * u;
        let total = g.text_w(word, size);
        let mut x = cx - total / 2.0;
        for (i, ch) in word.chars().enumerate() {
            let s = ch.to_string();
            let w = g.text_w(&s, size);
            let enter = ease_back(((self.t - 0.35 - i as f32 * 0.07) / 0.4).clamp(0.0, 1.0));
            let bob = (self.t * 3.2 - i as f32 * 0.6).sin() * 5.0 * u;
            g.label(&s, x + w / 2.0, logo_y + 70.0 * u + bob, size * enter, 0.5, rgb(0xffd23a), Some((rgb(0x7a3a00), 3.4 * u)), (self.t * 2.1 + i as f32).sin() * 0.05);
            x += w;
        }
        g.shine_off();

        self.fx.draw(g);
        self.rufus.draw_bubble(g, u, app.x0, app.x0 + app.pw);
        self.fx.draw_text(g);

        // boop the dog
        if app.ui.ptr.released && app.ui.ptr.travel < 14.0 * u && self.rufus.hit(app.ui.ptr.pos) && self.boop_cd <= 0.0 {
            self.boop_cd = 0.25;
            app.save.boops += 1;
            if app.save.boops % 5 == 0 {
                app.save.store();
            }
            app.au.play(if app.save.boops % 7 == 0 { Sfx::Bark2 } else { Sfx::Bark }, 0.8);
            crate::platform::vibrate(15);
            self.rufus.bark(1);
            self.rufus.react(Mood::Excited, 1.2);
            let line = if app.save.boops % 25 == 0 { "that's a LOT of boops." } else { self.rng.pick(&BOOP_QUIPS) };
            self.rufus.say(line, 1.5);
            self.fx.hearts(self.rufus.head, 3, u);
        }

        let mut nav = None;
        let by = ground + 22.0 * u;
        let fresh = app.save.total_paws() == 0;
        let label = if fresh { "PLAY" } else { "CONTINUE" };
        let pulse = 1.0 + 0.03 * (self.t * 4.0).sin();
        let bw = 228.0 * u * pulse;
        app.mark("play", Rect::new(cx - bw / 2.0, by, bw, 64.0 * u));
        if app.ui.button(&app.g, &mut app.au, 1, Rect::new(cx - bw / 2.0, by, bw, 64.0 * u), GREEN, label, Some(spr::IC_PLAY), self.t > 0.4) {
            // brand new players go straight into level one
            nav = Some(if fresh { Nav::Play(0) } else { Nav::Map(None) });
        }
        let g = &app.g;
        if fresh {
            g.label("Tap Rufus. Go on.", cx, by + 84.0 * u, 14.0 * u, 0.5, WHITE, Some((alpha(NAVY, 0.6), 1.3 * u)), 0.0);
        } else {
            let done = app.save.paws.iter().filter(|&&p| p > 0).count();
            // centred as one line, paw and all: the numbers run to four digits
            let text = format!("{} of {} paws   level {} of {}", app.save.total_paws(), LEVELS.len() * 3, (done + 1).min(LEVELS.len()), LEVELS.len());
            let size = (14.0 * u).min(14.0 * u * (app.pw - 60.0 * u) / g.text_w(&text, 14.0 * u).max(1.0));
            let left = cx - (g.text_w(&text, size) + 26.0 * u) / 2.0;
            g.spr_fit(spr::PAW_GOLD, left + 10.0 * u, by + 84.0 * u, 20.0 * u, 0.0, WHITE);
            g.label(&text, left + 26.0 * u, by + 84.0 * u, size, 0.0, WHITE, Some((alpha(NAVY, 0.6), 1.3 * u)), 0.0);
        }
        if app.save.boops > 0 {
            g.label(&format!("boops: {}", app.save.boops), app.x0 + app.pw - 12.0 * u, app.h - app.bot - 16.0 * u, 11.0 * u, 1.0, alpha(WHITE, 0.75), None, 0.0);
        }
        let ty = (by + 106.0 * u).min(app.h - app.bot - 70.0 * u);
        sound_toggles(app, cx, ty, 2);
        app.g.label(difficulty_blurb(app.save.difficulty), cx, ty + 58.0 * u, 12.5 * u, 0.5, WHITE, Some((alpha(NAVY, 0.6), 1.2 * u)), 0.0);
        nav
    }
}
