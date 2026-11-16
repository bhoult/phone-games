//! Particles, floating text, shockwave rings and screen shake.

use crate::gfx::*;
use macroquad::prelude::{vec2, Color, Vec2};
use biome_core::Rng;

pub struct Particle {
    pub spr: Spr,
    pub pos: Vec2,
    pub vel: Vec2,
    pub gravity: f32,
    pub drag: f32,
    pub rot: f32,
    pub spin: f32,
    pub size: f32,
    /// Size multiplier reached at end of life.
    pub grow: f32,
    pub age: f32,
    pub life: f32,
    pub color: Color,
}

pub struct Floater {
    pub text: String,
    pub pos: Vec2,
    pub vel: Vec2,
    pub age: f32,
    pub life: f32,
    pub size: f32,
    pub color: Color,
    pub edge: Color,
    pub rot: f32,
}

struct Ring {
    pos: Vec2,
    r0: f32,
    r1: f32,
    age: f32,
    life: f32,
    color: Color,
}

pub struct Fx {
    parts: Vec<Particle>,
    floaters: Vec<Floater>,
    rings: Vec<Ring>,
    shake: f32,
    shake_t: f32,
    pub flash: f32,
    pub rng: Rng,
}

impl Fx {
    pub fn new(seed: u64) -> Fx {
        Fx { parts: Vec::new(), floaters: Vec::new(), rings: Vec::new(), shake: 0.0, shake_t: 0.0, flash: 0.0, rng: Rng::new(seed) }
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.parts {
            p.age += dt;
            if p.age < 0.0 {
                continue;
            }
            p.vel.y += p.gravity * dt;
            let k = (-p.drag * dt).exp();
            p.vel *= k;
            p.pos += p.vel * dt;
            p.rot += p.spin * dt;
        }
        self.parts.retain(|p| p.age < p.life);
        for f in &mut self.floaters {
            f.age += dt;
            if f.age >= 0.0 {
                f.pos += f.vel * dt;
                f.vel *= (-2.5 * dt).exp();
            }
        }
        self.floaters.retain(|f| f.age < f.life);
        for r in &mut self.rings {
            r.age += dt;
        }
        self.rings.retain(|r| r.age < r.life);
        self.shake = (self.shake - dt * 26.0 * (0.3 + self.shake * 0.12)).max(0.0);
        self.shake_t += dt;
        self.flash = (self.flash - dt * 2.4).max(0.0);
        if self.parts.len() > 900 {
            let extra = self.parts.len() - 900;
            self.parts.drain(0..extra);
        }
    }

    /// Screen-shake offset to add to whatever should rattle.
    pub fn offset(&self) -> Vec2 {
        let t = self.shake_t * 47.0;
        vec2(t.sin() * self.shake, (t * 1.31 + 1.0).cos() * self.shake)
    }

    pub fn shake(&mut self, amount: f32) {
        self.shake = self.shake.max(amount);
    }

    pub fn draw(&self, g: &Gfx) {
        for r in &self.rings {
            if r.age < 0.0 {
                continue;
            }
            let t = r.age / r.life;
            let rad = lerp(r.r0, r.r1, ease_out(t));
            // the ring art has its stroke at 84% of the sprite's half-width
            let d = rad * 2.0 / 0.84;
            g.spr(spr::RING, r.pos.x, r.pos.y, d, d, 0.0, alpha(r.color, 1.0 - t * t));
        }
        for p in &self.parts {
            if p.age < 0.0 {
                continue;
            }
            let t = p.age / p.life;
            let size = p.size * lerp(1.0, p.grow, t);
            let fade = if t > 0.6 { 1.0 - (t - 0.6) / 0.4 } else { 1.0 };
            let k = size / p.spr.w.max(p.spr.h);
            g.spr(p.spr, p.pos.x, p.pos.y, p.spr.w * k, p.spr.h * k, p.rot, alpha(p.color, fade));
        }
        // sparks and stars also give off a little light
        if g.has_light() {
            g.lights(|| {
                for p in &self.parts {
                    let sparkly = (p.spr.x == spr::P_SPARK.x && p.spr.y == spr::P_SPARK.y) || (p.spr.x == spr::P_STAR.x && p.spr.y == spr::P_STAR.y);
                    if p.age < 0.0 || !sparkly {
                        continue;
                    }
                    let t = p.age / p.life;
                    let size = p.size * lerp(1.0, p.grow, t) * 2.6;
                    let fade = if t > 0.6 { 1.0 - (t - 0.6) / 0.4 } else { 1.0 };
                    g.spr(spr::GLOW, p.pos.x, p.pos.y, size, size, 0.0, alpha(p.color, 0.45 * fade));
                }
                for r in self.rings.iter().filter(|r| r.age >= 0.0) {
                    let t = r.age / r.life;
                    let d = lerp(r.r0, r.r1, ease_out(t)) * 2.0 / 0.84;
                    g.spr(spr::RING, r.pos.x, r.pos.y, d, d, 0.0, alpha(r.color, 0.5 * (1.0 - t * t)));
                }
            });
        }
    }

    pub fn draw_text(&self, g: &Gfx) {
        for f in &self.floaters {
            if f.age < 0.0 {
                continue;
            }
            let t = f.age / f.life;
            let pop = ease_back((f.age / 0.22).min(1.0));
            let fade = if t > 0.7 { 1.0 - (t - 0.7) / 0.3 } else { 1.0 };
            let size = f.size * pop;
            g.label(&f.text, f.pos.x, f.pos.y, size, 0.5, alpha(f.color, fade), Some((f.edge, (size * 0.085).max(1.2))), f.rot);
        }
    }

    // ---- emitters ----------------------------------------------------------

    pub fn add(&mut self, p: Particle) {
        self.parts.push(p);
    }

    /// A pop of small bits flying outwards.
    pub fn burst(&mut self, pos: Vec2, color: Color, n: usize, speed: f32, size: f32, delay: f32) {
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let v = speed * self.rng.range(0.45, 1.0);
            let star = self.rng.chance(0.35);
            self.parts.push(Particle {
                spr: if star { spr::P_STAR } else { spr::P_DOT },
                pos,
                vel: vec2(a.cos() * v, a.sin() * v - speed * 0.25),
                gravity: speed * 2.6,
                drag: 2.2,
                rot: self.rng.range(0.0, 6.0),
                spin: self.rng.range(-8.0, 8.0),
                size: size * self.rng.range(0.6, 1.0) * if star { 1.5 } else { 1.0 },
                grow: 0.25,
                age: -delay,
                life: self.rng.range(0.35, 0.6),
                color: if star { mix(color, WHITE, 0.55) } else { color },
            });
        }
    }

    pub fn sparkle(&mut self, pos: Vec2, spread: f32, n: usize, size: f32, color: Color) {
        for _ in 0..n {
            let off = vec2(self.rng.range(-spread, spread), self.rng.range(-spread, spread));
            self.parts.push(Particle {
                spr: spr::P_SPARK,
                pos: pos + off,
                vel: vec2(0.0, -spread * 0.4),
                gravity: 0.0,
                drag: 1.0,
                rot: self.rng.range(0.0, 3.0),
                spin: self.rng.range(-3.0, 3.0),
                size: size * self.rng.range(0.5, 1.0),
                grow: 0.0,
                age: -self.rng.range(0.0, 0.25),
                life: self.rng.range(0.3, 0.6),
                color,
            });
        }
    }

    /// Water thrown up: a splash.
    pub fn spray(&mut self, pos: Vec2, n: usize, speed: f32, size: f32) {
        for _ in 0..n {
            let a = self.rng.range(-std::f32::consts::PI, 0.0);
            let v = self.rng.range(0.35, 1.0) * speed;
            let vel = vec2(a.cos() * v * 1.3, a.sin() * v);
            self.parts.push(Particle {
                spr: spr::P_DROP,
                pos: pos + vec2(self.rng.range(-size, size), self.rng.range(-size, size) * 0.5),
                vel,
                gravity: speed * 2.6,
                drag: 0.6,
                rot: vel.x.atan2(-vel.y),
                spin: 0.0,
                size: size * self.rng.range(0.5, 1.1),
                grow: 0.7,
                age: 0.0,
                life: self.rng.range(0.45, 0.9),
                color: if self.rng.chance(0.3) { WHITE } else { rgb(0x9fdcff) },
            });
        }
    }

    /// A puff of dust or smoke.
    pub fn puff(&mut self, pos: Vec2, size: f32, drift: Vec2, color: Color) {
        self.parts.push(Particle {
            spr: spr::P_PUFF,
            pos,
            vel: drift + vec2(self.rng.range(-size, size) * 0.6, self.rng.range(-size, 0.0)),
            gravity: -size * 0.8,
            drag: 2.5,
            rot: self.rng.range(0.0, 6.0),
            spin: self.rng.range(-1.5, 1.5),
            size: size * self.rng.range(0.7, 1.1),
            grow: 1.7,
            age: 0.0,
            life: self.rng.range(0.35, 0.6),
            color,
        });
    }

    /// Bits of a specific sprite thrown outwards (leaves, splinters, snow).
    #[allow(clippy::too_many_arguments)]
    pub fn chunks(&mut self, s: Spr, pos: Vec2, color: Color, n: usize, speed: f32, size: f32, gravity: f32) {
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let v = speed * self.rng.range(0.4, 1.0);
            self.parts.push(Particle {
                spr: s,
                pos,
                vel: vec2(a.cos() * v, a.sin() * v - speed * 0.5),
                gravity,
                drag: 1.2,
                rot: self.rng.range(0.0, 6.0),
                spin: self.rng.range(-10.0, 10.0),
                size: size * self.rng.range(0.6, 1.1),
                grow: 0.6,
                age: 0.0,
                life: self.rng.range(0.45, 0.8),
                color,
            });
        }
    }

    /// Party over a whole area: for level-complete.
    pub fn confetti(&mut self, x0: f32, x1: f32, y: f32, n: usize, u: f32) {
        const COLORS: [u32; 6] = [0xff5a7a, 0xffd23a, 0x5fd6cb, 0x8fb2ee, 0xc8ea24, 0xee86fb];
        for _ in 0..n {
            let c = rgb(COLORS[self.rng.below(COLORS.len() as u32) as usize]);
            let s = [spr::P_BIT, spr::P_STAR, spr::P_LEAF, spr::P_HEART][self.rng.below(4) as usize];
            self.parts.push(Particle {
                spr: s,
                pos: vec2(self.rng.range(x0, x1), y - self.rng.range(0.0, 60.0 * u)),
                vel: vec2(self.rng.range(-60.0, 60.0) * u, self.rng.range(40.0, 200.0) * u),
                gravity: 240.0 * u,
                drag: 1.6,
                rot: self.rng.range(0.0, 6.0),
                spin: self.rng.range(-7.0, 7.0),
                size: self.rng.range(9.0, 17.0) * u,
                grow: 0.8,
                age: -self.rng.range(0.0, 0.9),
                life: self.rng.range(1.6, 2.8),
                color: if matches!(s.w as i32, 64) && s.x == spr::P_HEART.x && s.y == spr::P_HEART.y { WHITE } else { c },
            });
        }
    }

    pub fn hearts(&mut self, pos: Vec2, n: usize, u: f32) {
        for i in 0..n {
            self.parts.push(Particle {
                spr: spr::P_HEART,
                pos: pos + vec2(self.rng.range(-18.0, 18.0) * u, self.rng.range(-8.0, 8.0) * u),
                vel: vec2(self.rng.range(-20.0, 20.0) * u, -self.rng.range(40.0, 70.0) * u),
                gravity: -20.0 * u,
                drag: 0.8,
                rot: self.rng.range(-0.3, 0.3),
                spin: 0.0,
                size: self.rng.range(12.0, 20.0) * u,
                grow: 1.3,
                age: -(i as f32) * 0.12,
                life: 1.2,
                color: WHITE,
            });
        }
    }

    /// One sprite that drifts and fades: a feather, a note, a snowflake.
    #[allow(clippy::too_many_arguments)]
    pub fn drift(&mut self, s: Spr, pos: Vec2, vel: Vec2, size: f32, life: f32, color: Color, gravity: f32) {
        self.parts.push(Particle {
            spr: s,
            pos,
            vel,
            gravity,
            drag: 1.0,
            rot: self.rng.range(-0.4, 0.4),
            spin: self.rng.range(-1.5, 1.5),
            size,
            grow: 1.2,
            age: 0.0,
            life,
            color,
        });
    }

    pub fn ring(&mut self, pos: Vec2, r0: f32, r1: f32, life: f32, color: Color, delay: f32) {
        self.rings.push(Ring { pos, r0, r1, age: -delay, life, color });
    }

    /// Floating text that pops in, drifts up and fades.
    pub fn text(&mut self, text: &str, pos: Vec2, size: f32, color: Color, edge: Color, life: f32) -> &mut Floater {
        self.floaters.push(Floater {
            text: text.to_string(),
            pos,
            vel: vec2(0.0, -size * 1.1),
            age: 0.0,
            life,
            size,
            color,
            edge,
            rot: 0.0,
        });
        self.floaters.last_mut().unwrap()
    }
}
