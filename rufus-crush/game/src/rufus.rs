//! Rufus himself: a puppet assembled from separately drawn parts that share
//! one coordinate space (300 x 330 units, feet at y = 322), so his ears can
//! flop, his tail can wag and his face can change while he sits there.

use crate::fx::{Fx, Particle};
use crate::gfx::*;
use macroquad::prelude::{vec2, Rect, Vec2};
use rufus_core::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    /// Panting, wagging gently, glancing about.
    Idle,
    Happy,
    /// Bouncing on the spot.
    Excited,
    /// Ears up, eyes wide. Something has been SEEN.
    Alert,
    Sad,
    Asleep,
    /// Head tilt.
    Confused,
    Worried,
    /// Holding something. Not giving it back.
    Proud,
    /// The look. Chin down, eyes up, enormous. Nobody has ever said no to it.
    Pitiful,
}

struct Bubble {
    text: String,
    age: f32,
    life: f32,
}

pub struct Rufus {
    mood: Mood,
    base: Mood,
    mood_left: f32,
    t: f32,
    blink_in: f32,
    blink: f32,
    wag: f32,
    barks: u8,
    bark: f32,
    tilt: f32,
    hop: f32,
    ear: f32,
    ear_vel: f32,
    holding: Option<Spr>,
    /// Something worn over the body and under the chin: the scarf. It is
    /// drawn exactly where the body is, so its art shares the body's frame.
    pub wear: Option<Spr>,
    hold_left: f32,
    bubble: Option<Bubble>,
    zzz_in: f32,
    rng: Rng,
    // where he was last drawn, for hit tests and for effects to aim at
    pub head: Vec2,
    pub mouth: Vec2,
    pub k: f32,
    bounds: Rect,
}

const NECK: Vec2 = Vec2::new(150.0, 182.0);
const TAIL_PIVOT: Vec2 = Vec2::new(208.0, 306.0);
const EAR_PIVOT: Vec2 = Vec2::new(100.0, 60.0);

fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

fn centre(s: Spr) -> Vec2 {
    vec2(s.vx + s.vw / 2.0, s.vy + s.vh / 2.0)
}

impl Rufus {
    pub fn new(seed: u64) -> Rufus {
        Rufus {
            mood: Mood::Idle,
            base: Mood::Idle,
            mood_left: 0.0,
            t: 0.0,
            blink_in: 2.0,
            blink: 0.0,
            wag: 0.0,
            barks: 0,
            bark: 0.0,
            tilt: 0.0,
            hop: 0.0,
            ear: 0.0,
            ear_vel: 0.0,
            holding: None,
            wear: None,
            hold_left: 0.0,
            bubble: None,
            zzz_in: 1.0,
            rng: Rng::new(seed),
            head: Vec2::ZERO,
            mouth: Vec2::ZERO,
            k: 1.0,
            bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    pub fn mood(&self) -> Mood {
        self.mood
    }

    /// A passing reaction; he drifts back to his resting mood afterwards.
    pub fn react(&mut self, mood: Mood, secs: f32) {
        self.mood = mood;
        self.mood_left = secs;
    }

    /// The mood he settles into between reactions.
    pub fn rest(&mut self, mood: Mood) {
        self.base = mood;
        if self.mood_left <= 0.0 {
            self.mood = mood;
        }
    }

    pub fn bark(&mut self, times: u8) {
        self.barks = self.barks.saturating_add(times).min(4);
    }

    pub fn say(&mut self, text: &str, secs: f32) {
        self.bubble = Some(Bubble { text: text.to_string(), age: 0.0, life: secs });
    }

    pub fn quiet(&mut self) {
        self.bubble = None;
    }

    pub fn speaking(&self) -> bool {
        self.bubble.is_some()
    }

    /// Put something in his mouth for a while.
    pub fn hold(&mut self, s: Spr, secs: f32) {
        self.holding = Some(s);
        self.hold_left = secs;
    }

    pub fn hit(&self, p: Vec2) -> bool {
        self.bounds.contains(p)
    }

    pub fn update(&mut self, dt: f32, fx: &mut Fx) {
        self.t += dt;
        if self.mood_left > 0.0 {
            self.mood_left -= dt;
            if self.mood_left <= 0.0 {
                self.mood = self.base;
            }
        }
        // blinking
        if self.blink > 0.0 {
            self.blink -= dt;
        } else {
            self.blink_in -= dt;
            if self.blink_in <= 0.0 {
                self.blink = 0.13;
                self.blink_in = self.rng.range(1.8, 5.0);
            }
        }
        // barking: a short pulse per bark
        if self.bark > 0.0 {
            self.bark -= dt;
        } else if self.barks > 0 {
            self.barks -= 1;
            self.bark = 0.21;
        }
        let (wag_speed, hop_speed, tilt_to) = match self.mood {
            Mood::Idle => (5.0, 0.0, 0.0),
            Mood::Happy => (17.0, 0.0, 0.05 * (self.t * 5.0).sin()),
            Mood::Excited => (24.0, 10.5, 0.07 * (self.t * 9.0).sin()),
            Mood::Alert => (0.0, 0.0, 0.0),
            Mood::Sad => (0.0, 0.0, 0.06),
            Mood::Asleep => (0.0, 0.0, 0.16),
            Mood::Confused => (2.0, 0.0, 0.34),
            Mood::Worried => (1.5, 0.0, -0.05),
            Mood::Proud => (14.0, 0.0, -0.06),
            // one slow, hopeful wag at a time
            Mood::Pitiful => (3.2, 0.0, 0.13),
        };
        self.wag += wag_speed * dt;
        if hop_speed > 0.0 {
            self.hop += hop_speed * dt;
        } else {
            // finish the hop in progress rather than snapping to the ground
            let phase = self.hop % std::f32::consts::PI;
            self.hop = if phase > 0.05 { self.hop + 10.5 * dt } else { 0.0 };
            if self.hop % std::f32::consts::PI < phase {
                self.hop = 0.0;
            }
        }
        self.tilt += (tilt_to - self.tilt) * (1.0 - (-dt * 9.0).exp());
        // ears are on springs: they lag behind hops and barks
        let target = match self.mood {
            Mood::Alert => 0.55,
            Mood::Sad | Mood::Asleep => -0.06,
            Mood::Pitiful => -0.1,
            Mood::Excited | Mood::Happy => 0.12,
            _ => 0.0,
        } + (self.hop.sin().abs()) * 0.5
            + if self.bark > 0.0 { 0.3 } else { 0.0 };
        self.ear_vel += ((target - self.ear) * 140.0 - self.ear_vel * 11.0) * dt;
        self.ear += self.ear_vel * dt;

        if self.hold_left > 0.0 {
            self.hold_left -= dt;
            if self.hold_left <= 0.0 {
                self.holding = None;
            }
        }
        if let Some(b) = &mut self.bubble {
            b.age += dt;
            if b.age >= b.life {
                self.bubble = None;
            }
        }
        if self.mood == Mood::Asleep {
            self.zzz_in -= dt;
            if self.zzz_in <= 0.0 {
                self.zzz_in = 0.9;
                fx.text("z", self.head + vec2(34.0, -40.0) * self.k, 26.0 * self.k.max(0.3), rgb(0xffffff), rgb(0x3b4a8a), 1.6).vel =
                    vec2(14.0, -26.0) * self.k.max(0.3);
            }
        }
        if self.mood == Mood::Excited && self.rng.chance(dt * 5.0) {
            fx.add(Particle {
                spr: spr::P_HEART,
                pos: self.head + vec2(self.rng.range(-50.0, 50.0), self.rng.range(-70.0, -30.0)) * self.k,
                vel: vec2(self.rng.range(-20.0, 20.0), -60.0) * self.k,
                gravity: 0.0,
                drag: 1.0,
                rot: self.rng.range(-0.3, 0.3),
                spin: 0.0,
                size: 34.0 * self.k,
                grow: 1.3,
                age: 0.0,
                life: 0.9,
                color: WHITE,
            });
        }
    }

    /// Draws him sitting with his feet at (`cx`, `ground`), `height` tall.
    pub fn draw(&mut self, g: &Gfx, cx: f32, ground: f32, height: f32) {
        let k = height / 330.0;
        self.k = k;
        let hop = self.hop.sin().abs() * 20.0;
        // rig space -> screen
        let w = |p: Vec2| vec2(cx + (p.x - 150.0) * k, ground + (p.y - 322.0 - hop) * k);
        let part = |s: Spr, at: Vec2, angle: f32, flip: bool, sx: f32, sy: f32| {
            let p = w(at);
            g.spr_ex(s, p.x, p.y, s.vw * k * sx, s.vh * k * sy, angle, WHITE, flip);
        };

        // shadow stays on the ground and shrinks as he leaves it
        let sh = 1.0 - hop / 60.0;
        g.spr(spr::P_DOT, cx, ground - 2.0 * k, 210.0 * k * sh, 26.0 * k * sh, 0.0, alpha(rgb(0x000000), 0.2 * sh));

        // tail
        let (wag_amp, droop) = match self.mood {
            Mood::Idle => (0.16, 0.0),
            Mood::Happy | Mood::Proud => (0.42, -0.08),
            Mood::Excited => (0.55, -0.12),
            Mood::Sad | Mood::Asleep => (0.0, 0.55),
            Mood::Worried => (0.05, 0.3),
            Mood::Alert => (0.0, -0.25),
            Mood::Confused => (0.08, 0.0),
            Mood::Pitiful => (0.07, 0.42),
        };
        let ta = self.wag.sin() * wag_amp + droop;
        part(spr::RUFUS_TAIL, TAIL_PIVOT + rot(centre(spr::RUFUS_TAIL) - TAIL_PIVOT, ta), ta, false, 1.0, 1.0);

        // body, breathing from the feet up
        let breath = if self.mood == Mood::Asleep { 0.02 * (self.t * 1.6).sin() } else { 0.012 * (self.t * 4.2).sin() };
        let bc = centre(spr::RUFUS_BODY);
        let body_bottom = spr::RUFUS_BODY.vy + spr::RUFUS_BODY.vh;
        part(spr::RUFUS_BODY, vec2(bc.x, body_bottom - (body_bottom - bc.y) * (1.0 + breath)), 0.0, false, 1.0, 1.0 + breath);
        if let Some(worn) = self.wear {
            let wc = centre(worn);
            let bottom = worn.vy + worn.vh;
            part(worn, vec2(wc.x, bottom - (bottom - wc.y) * (1.0 + breath)), 0.0, false, 1.0, 1.0 + breath);
        }

        // head group: everything rotates about the neck
        let jolt = if self.bark > 0.0 { (self.bark / 0.21 * std::f32::consts::PI).sin() } else { 0.0 };
        let lower = match self.mood {
            Mood::Sad => 7.0,
            Mood::Asleep => 12.0,
            Mood::Pitiful => 11.0,
            _ => 0.0,
        };
        // the look comes with a very slight tremble
        let tremble = if self.mood == Mood::Pitiful { (self.t * 31.0).sin() * 0.7 } else { 0.0 };
        let head_off = vec2(tremble, -breath * 170.0 - jolt * 9.0 + lower);
        let ha = self.tilt - jolt * 0.07;
        let head = |p: Vec2| NECK + rot(p - NECK, ha) + head_off;

        part(spr::RUFUS_HEAD, head(centre(spr::RUFUS_HEAD)), ha, false, 1.0, 1.0);

        let ec = centre(spr::RUFUS_EAR);
        let ea = self.ear + 0.03 * (self.t * 2.7).sin();
        part(spr::RUFUS_EAR, head(EAR_PIVOT + rot(ec - EAR_PIVOT, ea)), ha + ea, false, 1.0, 1.0);
        let mirror = |p: Vec2| vec2(300.0 - p.x, p.y);
        part(spr::RUFUS_EAR, head(mirror(EAR_PIVOT) + rot(mirror(ec) - mirror(EAR_PIVOT), -ea)), ha - ea, true, 1.0, 1.0);

        let eyes = match self.mood {
            Mood::Asleep => spr::RUFUS_EYES_CLOSED,
            Mood::Happy | Mood::Excited | Mood::Proud => spr::RUFUS_EYES_HAPPY,
            Mood::Sad => spr::RUFUS_EYES_SAD,
            Mood::Pitiful => spr::RUFUS_EYES_PITIFUL,
            Mood::Alert | Mood::Worried => spr::RUFUS_EYES_WIDE,
            _ if self.blink > 0.0 => spr::RUFUS_EYES_CLOSED,
            _ => spr::RUFUS_EYES_OPEN,
        };
        part(eyes, head(centre(eyes)), ha, false, 1.0, 1.0);

        let mouth = if self.bark > 0.0 {
            spr::RUFUS_MOUTH_BARK
        } else {
            match self.mood {
                Mood::Sad | Mood::Worried | Mood::Pitiful => spr::RUFUS_MOUTH_SAD,
                Mood::Asleep | Mood::Alert | Mood::Confused | Mood::Proud => spr::RUFUS_MOUTH_SMILE,
                _ => spr::RUFUS_MOUTH_OPEN,
            }
        };
        // panting: the open mouth bobs from its top edge
        let pant = if mouth.x == spr::RUFUS_MOUTH_OPEN.x && mouth.y == spr::RUFUS_MOUTH_OPEN.y { 0.05 * (self.t * 11.0).sin() } else { 0.0 };
        let mc = centre(mouth);
        part(mouth, head(vec2(mc.x, mouth.vy + (mc.y - mouth.vy) * (1.0 + pant))), ha, false, 1.0, 1.0 + pant);

        if let Some(item) = self.holding {
            let p = w(head(vec2(150.0, 170.0)));
            g.spr_fit(item, p.x, p.y, 82.0 * k, ha + 0.25, WHITE);
        }

        self.head = w(head(vec2(150.0, 100.0)));
        self.mouth = w(head(vec2(150.0, 165.0)));
        self.bounds = Rect::new(cx - 105.0 * k, ground - 300.0 * k, 210.0 * k, 300.0 * k);
    }

    /// His speech bubble, drawn in a later pass so it sits above the scene.
    /// `left`/`right` are the screen edges it must stay inside.
    pub fn draw_bubble(&self, g: &Gfx, u: f32, left: f32, right: f32) {
        let Some(b) = &self.bubble else { return };
        let fade_in = ease_back((b.age / 0.2).min(1.0));
        let fade_out = ((b.life - b.age) / 0.2).clamp(0.0, 1.0);
        let scale = fade_in * (0.6 + 0.4 * fade_out);
        if scale <= 0.01 {
            return;
        }
        let size = 15.0 * u;
        let max_w = 150.0 * u;
        let lines = g.wrap(&b.text, size, max_w);
        let tw = lines.iter().map(|l| g.text_w(l, size)).fold(0.0, f32::max);
        let (bw, bh) = ((tw + 24.0 * u).max(46.0 * u) * scale, (lines.len() as f32 * size * 1.15 + 18.0 * u) * scale);
        // sits up and to the right of his head, tail pointing back at him
        let anchor = self.head + vec2(52.0, -58.0) * self.k;
        let x = (anchor.x - 14.0 * u).clamp(left + 4.0 * u, (right - bw - 4.0 * u).max(left + 4.0 * u));
        let y = anchor.y - bh;
        let a = fade_out;
        g.nine(spr::BUBBLE, Rect::new(x, y, bw, bh), 30.0, 13.0 * u * scale, alpha(WHITE, a));
        // the tail's flat top covers the bubble's outline where they join
        let tail = 15.0 * u * scale;
        g.spr(spr::BUBBLE_TAIL, anchor.x.clamp(x + 14.0 * u, (x + bw - 14.0 * u).max(x + 14.0 * u)), y + bh + tail * 0.28, tail, tail, 0.0, alpha(WHITE, a));
        for (i, line) in lines.iter().enumerate() {
            let ly = y + (9.0 * u + size * 1.15 * (i as f32 + 0.5)) * scale;
            g.text_c(line, x + bw / 2.0, ly, size * scale, alpha(rgb(0x22283f), a));
        }
    }
}
