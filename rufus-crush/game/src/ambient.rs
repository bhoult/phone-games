//! Small things that move in the backdrop so the scenes feel alive: clouds,
//! butterflies, falling leaves, rain, dust in a sunbeam, fireflies. Each one
//! is a pure function of the clock, so there is nothing to store or update.

use crate::gfx::*;
use macroquad::prelude::{vec2, Color, Rect, Vec2};
use std::f32::consts::TAU;

/// A fixed "random" number in 0..1 for thing number `i`.
fn h(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA6B).wrapping_add(0x1234_5678);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Fades things out as they near the left and right edges of the scene.
fn edge(x: f32) -> f32 {
    (x / 0.07).min((1.0 - x) / 0.07).clamp(0.0, 1.0)
}

const YARD: usize = 0;
const PARK: usize = 1;
const MUD: usize = 2;
const HOME: usize = 3;
const NIGHT: usize = 4;
const LAKE: usize = 5;

/// Draws the moving parts of scene number `scene` over its backdrop, which
/// fills `r`. `u` is the layout unit.
pub fn draw(g: &Gfx, scene: usize, r: Rect, t: f32, u: f32) {
    let at = |x: f32, y: f32| -> Vec2 { vec2(r.x + x * r.w, r.y + y * r.h) };

    // ---- things painted over the scene -----------------------------------
    if matches!(scene, YARD | PARK | LAKE) {
        for i in 0..3 {
            let x = (h(i, 1) + t * (0.004 + 0.004 * h(i, 2))).fract() * 1.5 - 0.25;
            let p = at(x, 0.05 + 0.2 * h(i, 3));
            let w = (84.0 + 60.0 * h(i, 4)) * u;
            g.spr(spr::CLOUD, p.x, p.y, w, w * 0.5, 0.0, alpha(WHITE, 0.8 * edge(x)));
        }
    }
    if scene == YARD {
        for i in 0..3 {
            let x = (h(i, 11) + t * (0.018 + 0.014 * h(i, 12))).fract() * 1.2 - 0.1;
            let y = 0.5 + 0.16 * h(i, 13) + 0.07 * (t * (0.9 + h(i, 14)) + h(i, 15) * TAU).sin();
            let p = at(x, y);
            let flap = (t * 13.0 + h(i, 16) * TAU).sin().abs() * 0.85 + 0.15;
            let s = (17.0 + 6.0 * h(i, 17)) * u;
            let tint = [rgb(0xffd6ec), rgb(0xfff2a0), rgb(0xbfe6ff)][i as usize % 3];
            g.spr(spr::BUTTERFLY, p.x, p.y, s * flap, s, 0.25 * (t * 2.0 + i as f32).sin(), alpha(tint, edge(x)));
        }
    }
    if matches!(scene, YARD | PARK) {
        let (n, spr, tints): (u32, Spr, [Color; 3]) = if scene == YARD {
            (5, spr::P_PETAL, [rgb(0xffc4de), rgb(0xffffff), rgb(0xffe08a)])
        } else {
            (8, spr::P_LEAF, [rgb(0x8fd45a), rgb(0xf2b43a), rgb(0xe8763a)])
        };
        for i in 0..n {
            let y = (h(i, 21) + t * (0.045 + 0.04 * h(i, 22))).fract();
            let x = h(i, 23) + 0.05 * (t * (0.8 + h(i, 24)) + h(i, 25) * TAU).sin() - 0.08 * y;
            let p = at(x, y * 0.92);
            let s = (9.0 + 7.0 * h(i, 26)) * u;
            let fade = (y / 0.1).min((0.95 - y) / 0.15).clamp(0.0, 1.0) * edge(x);
            g.spr(spr, p.x, p.y, s, s, t * (1.0 + h(i, 27)) + h(i, 28) * TAU, alpha(tints[i as usize % 3], 0.9 * fade));
        }
    }
    if scene == MUD {
        for i in 0..26 {
            let y = (h(i, 31) + t * (1.3 + 0.7 * h(i, 32))).fract();
            let x = h(i, 33) * 1.15 - 0.2 * y;
            let p = at(x, y * 0.94);
            let fade = (1.0 - y).min(1.0) * edge(x.clamp(0.0, 1.0));
            g.spr(spr::P_DROP, p.x, p.y, 3.2 * u, (13.0 + 8.0 * h(i, 34)) * u, 0.21, alpha(rgb(0xd6e8ff), 0.5 * fade));
        }
    }

    // ---- things made of light -----------------------------------------------
    g.lights(|| {
        let a = if g.has_light() { 1.0 } else { 0.5 };
        if matches!(scene, YARD | PARK | LAKE | HOME) {
            // sunbeams from the top corner, swaying slowly
            let (cx, k) = if scene == HOME { (0.62, 0.11) } else { (0.86, 0.15) };
            let c = at(cx, -0.08);
            let s = r.h * 2.5;
            g.spr(spr::RAYS, c.x, c.y, s, s, 0.3 + 0.08 * (t * 0.23).sin(), alpha(rgb(0xfff3c4), k * a));
            g.spr(spr::RAYS, c.x, c.y, s * 0.7, s * 0.7, -0.2 - 0.1 * (t * 0.17).sin(), alpha(WHITE, k * 0.7 * a));
        }
        if scene == LAKE {
            for i in 0..12 {
                let tw = (t * (1.4 + 2.2 * h(i, 41)) + h(i, 42) * TAU).sin().max(0.0).powi(4);
                let p = at(0.05 + 0.9 * h(i, 43), 0.5 + 0.3 * h(i, 44));
                let s = (9.0 + 9.0 * h(i, 45)) * u * (0.5 + 0.5 * tw);
                g.spr(spr::P_SPARK, p.x, p.y, s, s, 0.0, alpha(WHITE, 0.9 * tw * a));
            }
        }
        if scene == HOME {
            for i in 0..12 {
                let y = (h(i, 51) - t * (0.012 + 0.014 * h(i, 52))).rem_euclid(1.0);
                let x = 0.42 + 0.5 * h(i, 53) + 0.03 * (t * 0.6 + h(i, 54) * TAU).sin();
                let p = at(x, 0.1 + 0.75 * y);
                let tw = 0.45 + 0.55 * (t * (0.8 + h(i, 55)) + h(i, 56) * TAU).sin().abs();
                let s = (4.0 + 5.0 * h(i, 57)) * u;
                g.spr(spr::GLOW, p.x, p.y, s * 2.2, s * 2.2, 0.0, alpha(rgb(0xfff0c0), 0.55 * tw * a * edge(x)));
            }
        }
        if scene == NIGHT {
            for i in 0..16 {
                let tw = 0.35 + 0.65 * (t * (0.9 + 1.8 * h(i, 61)) + h(i, 62) * TAU).sin().abs().powi(2);
                let p = at(h(i, 63), 0.03 + 0.42 * h(i, 64));
                let s = (6.0 + 8.0 * h(i, 65)) * u * tw;
                g.spr(spr::P_SPARK, p.x, p.y, s, s, 0.0, alpha(rgb(0xeaf2ff), 0.85 * tw * a));
            }
            for i in 0..7 {
                let x = h(i, 71) + 0.09 * (t * (0.3 + 0.3 * h(i, 72)) + h(i, 73) * TAU).sin();
                let y = 0.56 + 0.3 * h(i, 74) + 0.05 * (t * (0.5 + 0.4 * h(i, 75)) + h(i, 76) * TAU).sin();
                let p = at(x, y);
                let pulse = (t * (1.1 + h(i, 77)) + h(i, 78) * TAU).sin().max(0.0).powf(1.5);
                let fade = pulse * edge(x.clamp(0.0, 1.0)) * a;
                g.spr(spr::GLOW, p.x, p.y, 26.0 * u, 26.0 * u, 0.0, alpha(rgb(0xd8ff6a), 0.8 * fade));
                g.spr(spr::P_DOT, p.x, p.y, 4.5 * u, 4.5 * u, 0.0, alpha(rgb(0xf6ffd0), fade));
            }
        }
    });
}

/// Big soft pools of light that drift around behind the board, so the play
/// area is not one flat gradient.
pub fn pools(g: &Gfx, r: Rect, t: f32, u: f32, tint: Color) {
    if !g.has_light() {
        return;
    }
    g.lights(|| {
        // a slow sunburst behind the board
        let c = vec2(r.x + r.w * 0.5, r.y + r.h * 0.42);
        let d = r.h * 1.9;
        g.spr(spr::RAYS, c.x, c.y, d, d, t * 0.05, alpha(tint, 0.2));
        g.spr(spr::RAYS, c.x, c.y, d * 0.7, d * 0.7, -t * 0.035, alpha(tint, 0.14));
        for i in 0..6 {
            let x = h(i, 81) + 0.22 * (t * (0.07 + 0.06 * h(i, 82)) + h(i, 83) * TAU).sin();
            let y = h(i, 84) + 0.16 * (t * (0.05 + 0.05 * h(i, 85)) + h(i, 86) * TAU).cos();
            let s = (230.0 + 190.0 * h(i, 87)) * u;
            g.spr(spr::GLOW, r.x + x * r.w, r.y + y * r.h, s, s, 0.0, alpha(tint, 0.2 + 0.1 * h(i, 88)));
        }
    });
}
