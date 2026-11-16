//! The ground a stage is played on: the biome's earth, the trail, water,
//! and the scenery scattered round them. Everything here is still; what
//! moves on top is drawn by `play`.

use crate::art::Palette;
use crate::gfx::*;
use biome_core::map::{Map, Tile, COLS, ROWS};
use biome_core::{v2, Rng, V2};
use macroquad::prelude::{draw_circle, draw_rectangle, vec2, Rect, Vec2};

/// Where the map sits on screen: tile coordinates to pixels and back.
#[derive(Clone, Copy)]
pub struct View {
    pub origin: Vec2,
    pub cell: f32,
}

impl View {
    /// Fits the 9 x 13 map into a rectangle, centred.
    pub fn fit(r: Rect) -> View {
        let cell = (r.w / COLS as f32).min(r.h / ROWS as f32);
        View { origin: vec2(r.x + (r.w - cell * COLS as f32) / 2.0, r.y + (r.h - cell * ROWS as f32) / 2.0), cell }
    }
    pub fn at(&self, p: V2) -> Vec2 {
        vec2(self.origin.x + p.x * self.cell, self.origin.y + p.y * self.cell)
    }
    pub fn tile(&self, s: Vec2) -> V2 {
        v2((s.x - self.origin.x) / self.cell, (s.y - self.origin.y) / self.cell)
    }
    pub fn rect(&self) -> Rect {
        Rect::new(self.origin.x, self.origin.y, self.cell * COLS as f32, self.cell * ROWS as f32)
    }
}

/// A piece of scenery standing on the map (drawn in depth order with
/// everything else).
pub struct Prop {
    pub pos: V2,
    pub spr: Spr,
    /// Height in tiles.
    pub size: f32,
    pub flip: bool,
}

/// The fixed dressing of one stage, worked out once from its map.
pub struct Scenery {
    /// Small things lying flat on the ground: tufts, flowers, pebbles.
    tufts: Vec<Prop>,
    /// Darker flecks along the trail.
    flecks: Vec<(V2, f32)>,
    /// Things with height: trees, rocks.
    pub props: Vec<Prop>,
}

impl Scenery {
    pub fn new(map: &Map, pal: &Palette, seed: u64) -> Scenery {
        let mut rng = Rng::new(seed);
        let mut tufts = Vec::new();
        let mut props = Vec::new();
        // a few tiles beyond the map on every side, so tall screens are not bare
        for r in -3..ROWS as i32 + 3 {
            for c in -2..COLS as i32 + 2 {
                let inside = c >= 0 && r >= 0 && c < COLS as i32 && r < ROWS as i32;
                let centre = v2(c as f32 + 0.5, r as f32 + 0.5);
                match map.tile(c, r) {
                    Tile::Tall if inside => props.push(Prop { pos: centre + v2(0.0, 0.3), spr: pal.tall, size: 1.5, flip: rng.chance(0.5) }),
                    Tile::Rock if inside => props.push(Prop { pos: centre + v2(0.0, 0.25), spr: pal.rock, size: 0.95, flip: rng.chance(0.5) }),
                    Tile::Water if inside => {
                        if rng.chance(0.45) {
                            tufts.push(Prop { pos: centre + v2(rng.range(-0.2, 0.2), rng.range(-0.15, 0.15)), spr: spr::TUFT_MARSH_A, size: rng.range(0.5, 0.65), flip: rng.chance(0.5) });
                        }
                    }
                    Tile::Ground => {
                        // keep clear of the trail and of mounds next door
                        let crowded = (-1..=1).any(|dr| (-1..=1).any(|dc| matches!(map.tile(c + dc, r + dr), Tile::Trail | Tile::Mound | Tile::Home | Tile::Spawn(_))));
                        let roll = rng.unit();
                        if !inside && r >= 0 && roll < 0.11 {
                            props.push(Prop { pos: centre + v2(rng.range(-0.3, 0.3), 0.3), spr: if rng.chance(0.6) { pal.tall } else { pal.rock }, size: rng.range(0.9, 1.3), flip: rng.chance(0.5) });
                        } else if inside && !crowded && roll < 0.07 {
                            props.push(Prop { pos: centre + v2(rng.range(-0.2, 0.2), 0.25), spr: if rng.chance(0.5) { pal.tall } else { pal.rock }, size: rng.range(0.8, 1.15), flip: rng.chance(0.5) });
                        } else if roll < 0.55 {
                            // mostly the plain tuft, now and then the fancy one
                            let spr = pal.tufts[if rng.chance(0.25) { 1 } else { 0 }];
                            tufts.push(Prop { pos: centre + v2(rng.range(-0.32, 0.32), rng.range(-0.32, 0.32)), spr, size: rng.range(0.34, 0.56), flip: rng.chance(0.5) });
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut flecks = Vec::new();
        for route in &map.routes {
            let mut d = 0.3;
            while d < route.len {
                let (p, dir) = route.at(d);
                flecks.push((p + dir.perp() * rng.range(-0.28, 0.28), rng.range(0.04, 0.09)));
                d += rng.range(0.35, 0.8);
            }
        }
        Scenery { tufts, flecks, props }
    }
}

fn trailish(t: Tile) -> bool {
    matches!(t, Tile::Trail | Tile::Spawn(_) | Tile::Home)
}

/// Draws a band through every tile of a kind, with round joints, so a run
/// of tiles reads as one winding strip rather than as squares.
fn strip(v: &View, map: &Map, want: impl Fn(Tile) -> bool, half: f32, color: macroquad::prelude::Color) {
    let r = half * v.cell;
    for row in 0..ROWS as i32 {
        for col in 0..COLS as i32 {
            if !want(map.tile(col, row)) {
                continue;
            }
            let c = v.at(v2(col as f32 + 0.5, row as f32 + 0.5));
            draw_circle(c.x, c.y, r, color);
            if col + 1 < COLS as i32 && want(map.tile(col + 1, row)) {
                draw_rectangle(c.x, c.y - r, v.cell, 2.0 * r, color);
            }
            if row + 1 < ROWS as i32 && want(map.tile(col, row + 1)) {
                draw_rectangle(c.x - r, c.y, 2.0 * r, v.cell, color);
            }
            // where invaders come in, the trail runs on off the edge of the map
            if let Tile::Spawn(_) = map.tile(col, row) {
                let far = v.cell * 6.0;
                if row == 0 {
                    draw_rectangle(c.x - r, c.y - far, 2.0 * r, far, color);
                } else if row == ROWS as i32 - 1 {
                    draw_rectangle(c.x - r, c.y, 2.0 * r, far, color);
                } else if col == 0 {
                    draw_rectangle(c.x - far, c.y - r, far, 2.0 * r, color);
                } else if col == COLS as i32 - 1 {
                    draw_rectangle(c.x, c.y - r, far, 2.0 * r, color);
                }
            }
        }
    }
}

/// The ground, water and trail. `full` is everything that needs covering
/// (the whole play column), `t` a clock for the water.
pub fn draw_ground(g: &Gfx, v: &View, map: &Map, pal: &Palette, sc: &Scenery, full: Rect, t: f32) {
    g.vgrad(full, pal.ground.0, pal.ground.1);
    // a faint chequer, so distances can be judged
    for row in 0..ROWS {
        for col in 0..COLS {
            if (row + col) % 2 == 0 {
                let p = v.at(v2(col as f32, row as f32));
                draw_rectangle(p.x, p.y, v.cell, v.cell, alpha(WHITE, 0.045));
            }
        }
    }
    // water: a dark rim, the pool, a lighter middle
    let water = |tile: Tile| tile == Tile::Water;
    strip(v, map, water, 0.6, mix(pal.water, rgb(0x1a3a52), 0.45));
    strip(v, map, water, 0.52, pal.water);
    strip(v, map, water, 0.36, mix(pal.water, WHITE, 0.16));
    for row in 0..ROWS as i32 {
        for col in 0..COLS as i32 {
            if map.tile(col, row) == Tile::Water {
                // slow glints sliding across
                let c = v.at(v2(col as f32 + 0.5, row as f32 + 0.5));
                let k = (t * 0.5 + col as f32 * 0.37 + row as f32 * 0.61).fract();
                let a = (k * std::f32::consts::PI).sin() * 0.5;
                g.spr(spr::STREAK, c.x + (k - 0.5) * v.cell * 0.6, c.y - v.cell * 0.12, v.cell * 0.34, v.cell * 0.07, 0.0, alpha(WHITE, a));
            }
        }
    }
    for p in &sc.tufts {
        let c = v.at(p.pos);
        let s = p.size * v.cell;
        g.spr_ex(p.spr, c.x, c.y, s, s * p.spr.h / p.spr.w, 0.0, WHITE, p.flip);
    }
    // the trail: a darker edge, the trodden middle, a pale crown
    strip(v, map, trailish, 0.47, pal.trail_edge);
    strip(v, map, trailish, 0.4, pal.trail);
    strip(v, map, trailish, 0.2, alpha(pal.trail_light, 0.45));
    for &(p, r) in &sc.flecks {
        let c = v.at(p);
        draw_circle(c.x, c.y, r * v.cell, alpha(pal.trail_edge, 0.28));
    }
}
