//! The ground a stage is played on: the biome's earth with the trail and
//! any water worked into it, and the scenery standing on it. Everything
//! here is still; what moves on top is drawn by `play`.

use crate::art::Palette;
use crate::gfx::*;
use biome_core::map::{Map, Tile, COLS, ROWS};
use biome_core::{v2, Rng, V2};
use macroquad::prelude::{draw_circle, draw_line, vec2, Color, FilterMode, Rect, Texture2D, Vec2};

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
    /// Longest side, in tiles.
    pub size: f32,
    pub flip: bool,
}

/// The fixed dressing of one stage, worked out once from its map.
pub struct Scenery {
    /// What the ground shader paints from.
    pub look: GroundLook,
    /// Things with height: trees, rocks.
    pub props: Vec<Prop>,
    /// The trail as plain lines, for devices without the shader.
    lines: Vec<(V2, V2)>,
    dust: Color,
}

// The mask covers the map and a margin all round it (the trail runs on off
// the edge where invaders come in, and tall screens show ground above and
// below the map).
const MASK_X: f32 = -1.5;
const MASK_Y: f32 = -6.0;
const MASK_W: f32 = COLS as f32 + 3.0;
const MASK_H: f32 = ROWS as f32 + 12.0;
/// Mask pixels per tile. It holds smooth ramps, so it needs very few.
const MASK_PX: f32 = 8.0;

fn seg_dist(p: V2, a: V2, b: V2) -> f32 {
    let ab = b - a;
    let l2 = ab.x * ab.x + ab.y * ab.y;
    let t = if l2 > 1e-9 { (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / l2).clamp(0.0, 1.0) } else { 0.0 };
    a.lerp(b, t).dist(p)
}

impl Scenery {
    pub fn new(map: &Map, pal: &Palette, level: usize, seed: u64) -> Scenery {
        let mut rng = Rng::new(seed);
        let mut props = Vec::new();
        // a few tiles beyond the map on every side, so tall screens are not bare
        for r in -4..ROWS as i32 + 5 {
            for c in -2..COLS as i32 + 2 {
                let inside = c >= 0 && r >= 0 && c < COLS as i32 && r < ROWS as i32;
                let centre = v2(c as f32 + 0.5, r as f32 + 0.5);
                if inside {
                    match map.tile(c, r) {
                        Tile::Tall => props.push(Prop { pos: centre + v2(0.0, 0.32), spr: pal.tall, size: 1.75, flip: rng.chance(0.5) }),
                        Tile::Rock => props.push(Prop { pos: centre + v2(0.0, 0.22), spr: pal.rock, size: 1.0, flip: rng.chance(0.5) }),
                        _ => {}
                    }
                } else if rng.chance(0.13) {
                    // off the map nothing is ever built, so anything can stand there
                    let tall = rng.chance(0.6);
                    let pos = centre + v2(rng.range(-0.3, 0.3), 0.3);
                    if map.nearest_trail(pos).2 > 1.4 {
                        props.push(Prop { pos, spr: if tall { pal.tall } else { pal.rock }, size: if tall { rng.range(1.4, 1.9) } else { rng.range(0.8, 1.1) }, flip: rng.chance(0.5) });
                    }
                }
            }
        }

        // every stretch of trail as a line, each route run on off the map
        // from where it starts
        let mut lines: Vec<(V2, V2)> = Vec::new();
        for route in &map.routes {
            let first = route.pts[0];
            let out = (first - route.pts[1]).norm();
            lines.push((first + out * 9.0, first));
            for w in route.pts.windows(2) {
                if !lines.iter().any(|l| l.0 == w[0] && l.1 == w[1]) {
                    lines.push((w[0], w[1]));
                }
            }
        }
        let mut water: Vec<V2> = Vec::new();
        for r in 0..ROWS {
            for c in 0..COLS {
                if map.tiles[r][c] == Tile::Water {
                    water.push(v2(c as f32 + 0.5, r as f32 + 0.5));
                }
            }
        }

        let (mw, mh) = ((MASK_W * MASK_PX) as usize, (MASK_H * MASK_PX) as usize);
        let mut px = vec![0u8; mw * mh * 4];
        for y in 0..mh {
            for x in 0..mw {
                let p = v2(MASK_X + (x as f32 + 0.5) / MASK_PX, MASK_Y + (y as f32 + 0.5) / MASK_PX);
                let mut d = f32::MAX;
                for &(a, b) in &lines {
                    // most of the trail is nowhere near: skip it cheaply
                    if (p.x - a.x).abs().min((p.x - b.x).abs()) > 2.2 && (p.y - a.y).abs().min((p.y - b.y).abs()) > 2.2 {
                        continue;
                    }
                    d = d.min(seg_dist(p, a, b));
                }
                // how much water is near: every water tile adds a soft round
                // pool, and neighbouring pools run together into one
                let mut wet = 0.0f32;
                for w in &water {
                    let d = p.dist(*w);
                    if d < 2.0 {
                        wet += (-d * d / 0.353).exp();
                    }
                }
                let i = (y * mw + x) * 4;
                px[i] = ((1.0 - d).clamp(0.0, 1.0) * 255.0) as u8;
                // scaled so a lone pool keeps just inside its own tile
                px[i + 1] = ((wet * 1.038).clamp(0.0, 1.0) * 255.0) as u8;
                px[i + 3] = 255;
            }
        }
        let mask = Texture2D::from_rgba8(mw as u16, mh as u16, &px);
        mask.set_filter(FilterMode::Linear);
        // each stage of a biome sees the same photograph a different way round
        let k = level % 3;
        let look = GroundLook {
            ground: pal.ground,
            trail: pal.trail,
            water: tex::WATER,
            mask,
            mask_at: [MASK_X, MASK_Y, MASK_W, MASK_H],
            flip: (k == 1, k == 2),
            trail_gain: pal.trail_gain,
            grade: pal.grade,
            vignette: 0.42,
        };
        Scenery { look, props, lines, dust: pal.dust }
    }
}

/// The ground, water and trail. `full` is everything that needs covering
/// (the whole play column), `t` a clock for the water.
pub fn draw_ground(g: &Gfx, v: &View, sc: &Scenery, full: Rect, t: f32) {
    let at = v.tile(vec2(full.x, full.y));
    if g.ground(&sc.look, full, vec2(at.x, at.y), v.cell, t) {
        return;
    }
    // no shader: the same photograph, and the trail as a plain band
    g.tiled(sc.look.ground, full, v.cell * 9.0, sc.look.grade);
    for (half, col) in [(0.5, alpha(BLACK, 0.3)), (0.42, sc.dust)] {
        for &(a, b) in &sc.lines {
            let (pa, pb) = (v.at(a), v.at(b));
            draw_line(pa.x, pa.y, pb.x, pb.y, half * 2.0 * v.cell, col);
            draw_circle(pb.x, pb.y, half * v.cell, col);
        }
    }
}
