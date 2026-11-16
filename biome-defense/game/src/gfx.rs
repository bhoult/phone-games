//! Drawing helpers over macroquad: atlas sprites, nine-slice panels, outlined
//! text that stays crisp on high-DPI phones, gradients and scene backdrops.

use macroquad::material::{gl_use_default_material, gl_use_material, load_material, Material, MaterialParams};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

// Two extra ways of putting a sprite on screen, besides the engine's own.
// Both use the engine's vertex shader unchanged.
const SPRITE_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}"#;

/// "Light": the same picture, but added to what is already on screen
/// instead of painted over it, so glows brighten rather than cover.
const LIGHT_FS: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}"#;

/// "Shine": a band of white light that slides diagonally across the screen
/// and shows up only where a sprite is, like a gleam crossing polished toys.
/// `Shine` is (band position, half-width, strength, unused), in pixels.
const SHINE_FS: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec4 Shine;
void main() {
    lowp vec4 c = color * texture2D(Texture, uv);
    float d = abs(gl_FragCoord.x - gl_FragCoord.y * 0.6 - Shine.x);
    float band = 1.0 - smoothstep(0.0, Shine.y, d);
    c.rgb += band * Shine.z;
    gl_FragColor = c;
}"#;

/// A sprite in the atlas. `x..h` is its pixel rect; `v*` is the SVG viewBox
/// it was drawn in, which is how character parts know where they belong.
#[derive(Clone, Copy, Debug)]
pub struct Spr {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub vx: f32,
    pub vy: f32,
    pub vw: f32,
    pub vh: f32,
}

include!(concat!(env!("OUT_DIR"), "/atlas.rs"));

pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
pub const NAVY: Color = Color::new(0.07, 0.11, 0.25, 1.0);

pub const fn rgb(hex: u32) -> Color {
    Color::new(
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        1.0,
    )
}

pub fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t)
}

pub struct Gfx {
    atlas: Texture2D,
    scenes: Vec<Texture2D>,
    /// Every glyph at a few sizes, made by the build script.
    glyphs: Texture2D,
    pub dpi: f32,
    // None if the device refused the shader: everything then falls back to
    // ordinary drawing, which looks plainer but is never wrong.
    light: Option<Material>,
    shine: Option<Material>,
}

impl Gfx {
    pub fn new() -> Gfx {
        // Text is drawn from a texture of glyphs made at build time, never
        // from the engine's own font cache: see the build script for why.
        let glyphs = Texture2D::from_file_with_format(FONT_PNG, Some(ImageFormat::Png));
        glyphs.set_filter(FilterMode::Linear);
        let atlas = Texture2D::from_file_with_format(ATLAS_PNG, Some(ImageFormat::Png));
        atlas.set_filter(FilterMode::Linear);
        let scenes = scene::ALL
            .iter()
            .map(|bytes| {
                let t = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
                t.set_filter(FilterMode::Linear);
                t
            })
            .collect();
        let blend = |onto: BlendFactor| PipelineParams {
            color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), onto)),
            ..Default::default()
        };
        let light = load_material(
            ShaderSource::Glsl { vertex: SPRITE_VS, fragment: LIGHT_FS },
            MaterialParams { pipeline_params: blend(BlendFactor::One), ..Default::default() },
        )
        .ok();
        let shine = load_material(
            ShaderSource::Glsl { vertex: SPRITE_VS, fragment: SHINE_FS },
            MaterialParams {
                pipeline_params: blend(BlendFactor::OneMinusValue(BlendValue::SourceAlpha)),
                uniforms: vec![UniformDesc::new("Shine", UniformType::Float4)],
                ..Default::default()
            },
        )
        .ok();
        Gfx { atlas, scenes, glyphs, dpi: 1.0, light, shine }
    }

    pub fn begin_frame(&mut self) {
        self.dpi = screen_dpi_scale().max(0.5);
    }

    // ---- light and shine ---------------------------------------------------

    /// Whether glows can be added as light (see [`Gfx::lights`]).
    pub fn has_light(&self) -> bool {
        self.light.is_some()
    }

    /// Everything `draw` puts on screen is added as light: it brightens what
    /// is underneath instead of covering it. Without shader support the same
    /// sprites are simply drawn the ordinary way.
    pub fn lights(&self, draw: impl FnOnce()) {
        if let Some(m) = &self.light {
            gl_use_material(m);
            draw();
            gl_use_default_material();
        } else {
            draw();
        }
    }

    /// Sprites drawn until [`Gfx::shine_off`] catch a gleam that crosses the
    /// whole screen every few seconds. `t` is any clock in seconds.
    pub fn shine_on(&self, t: f32) {
        if let Some(m) = &self.shine {
            let (w, h) = (screen_width() * self.dpi, screen_height() * self.dpi);
            let k = (t % 4.6) / 1.0;
            let half = 0.1 * w;
            let x = if k < 1.0 { -0.6 * h - half + (w + 0.6 * h + 2.0 * half) * k } else { -1.0e6 };
            m.set_uniform("Shine", [x, half, 0.45f32, 0.0]);
            gl_use_material(m);
        }
    }

    pub fn shine_off(&self) {
        if self.shine.is_some() {
            gl_use_default_material();
        }
    }

    // ---- sprites -----------------------------------------------------------

    /// Draws a sprite centred on (cx, cy), stretched to w x h.
    #[allow(clippy::too_many_arguments)]
    pub fn spr_ex(&self, s: Spr, cx: f32, cy: f32, w: f32, h: f32, rot: f32, color: Color, flip_x: bool) {
        if color.a <= 0.003 || w.abs() < 0.01 || h.abs() < 0.01 {
            return;
        }
        draw_texture_ex(
            &self.atlas,
            cx - w / 2.0,
            cy - h / 2.0,
            color,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                source: Some(Rect::new(s.x, s.y, s.w, s.h)),
                rotation: rot,
                flip_x,
                flip_y: false,
                pivot: None,
            },
        );
    }

    pub fn spr(&self, s: Spr, cx: f32, cy: f32, w: f32, h: f32, rot: f32, color: Color) {
        self.spr_ex(s, cx, cy, w, h, rot, color, false);
    }

    /// Draws a sprite scaled to fit a `size` square, keeping its proportions.
    pub fn spr_fit(&self, s: Spr, cx: f32, cy: f32, size: f32, rot: f32, color: Color) {
        let k = size / s.w.max(s.h);
        self.spr(s, cx, cy, s.w * k, s.h * k, rot, color);
    }

    /// Stretchable panel: corners keep their shape, edges and middle stretch.
    /// `src` is the corner size in the sprite's pixels, `dst` on screen.
    pub fn nine(&self, s: Spr, r: Rect, src: f32, dst: f32, color: Color) {
        if color.a <= 0.003 {
            return;
        }
        let dst = dst.min(r.w / 2.0).min(r.h / 2.0);
        let sx = [s.x, s.x + src, s.x + s.w - src, s.x + s.w];
        let sy = [s.y, s.y + src, s.y + s.h - src, s.y + s.h];
        let dx = [r.x, r.x + dst, r.x + r.w - dst, r.x + r.w];
        let dy = [r.y, r.y + dst, r.y + r.h - dst, r.y + r.h];
        for j in 0..3 {
            for i in 0..3 {
                let (w, h) = (dx[i + 1] - dx[i], dy[j + 1] - dy[j]);
                if w <= 0.0 || h <= 0.0 {
                    continue;
                }
                draw_texture_ex(
                    &self.atlas,
                    dx[i],
                    dy[j],
                    color,
                    DrawTextureParams {
                        dest_size: Some(vec2(w, h)),
                        source: Some(Rect::new(sx[i], sy[j], sx[i + 1] - sx[i], sy[j + 1] - sy[j])),
                        ..Default::default()
                    },
                );
            }
        }
    }

    // ---- shapes --------------------------------------------------------------

    pub fn rect(&self, r: Rect, color: Color) {
        draw_rectangle(r.x, r.y, r.w, r.h, color);
    }

    /// Vertical gradient.
    pub fn vgrad(&self, r: Rect, top: Color, bottom: Color) {
        let v = |x: f32, y: f32, c: Color| Vertex::new(x, y, 0.0, 0.0, 0.0, c);
        draw_mesh(&Mesh {
            vertices: vec![
                v(r.x, r.y, top),
                v(r.x + r.w, r.y, top),
                v(r.x + r.w, r.y + r.h, bottom),
                v(r.x, r.y + r.h, bottom),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: None,
        });
    }

    /// Chapter backdrop filling `r`, cropped (never squashed), ground kept.
    pub fn scene(&self, idx: usize, r: Rect, color: Color) {
        let tex = &self.scenes[idx % self.scenes.len()];
        let (tw, th) = (tex.width(), tex.height());
        let k = (r.w / tw).max(r.h / th);
        let (sw, sh) = (r.w / k, r.h / k);
        draw_texture_ex(
            tex,
            r.x,
            r.y,
            color,
            DrawTextureParams {
                dest_size: Some(vec2(r.w, r.h)),
                source: Some(Rect::new((tw - sw) * 0.5, th - sh, sw, sh)),
                ..Default::default()
            },
        );
    }

    // ---- text ------------------------------------------------------------------

    /// Which of the baked sizes to draw text of this size from, and how much
    /// to scale it: the smallest that is at least as big as the text will be
    /// on screen, so it is only ever scaled down a little (or, for the very
    /// biggest headings, up).
    fn face(&self, size: f32) -> (usize, f32) {
        let px = size * self.dpi * 0.94;
        let i = FONT_SIZES.iter().position(|&s| s >= px).unwrap_or(FONT_SIZES.len() - 1);
        (i, size / FONT_SIZES[i])
    }

    fn glyph(set: usize, ch: char) -> &'static Glyph {
        // the font is plain ASCII; anything else shows as a question mark
        let c = if (' '..='~').contains(&ch) { ch } else { '?' };
        &GLYPHS[set][c as usize - 32]
    }

    pub fn text_w(&self, s: &str, size: f32) -> f32 {
        let (set, scale) = self.face(size);
        s.chars().map(|ch| Self::glyph(set, ch).adv).sum::<f32>() * scale
    }

    /// Draws text with its baseline starting at (x, y), turned by `rot`
    /// about that point.
    fn raw_text(&self, s: &str, x: f32, y: f32, size: f32, rot: f32, color: Color) {
        let (set, scale) = self.face(size);
        let (sin, cos) = rot.sin_cos();
        let mut pen = 0.0;
        for ch in s.chars() {
            let g = Self::glyph(set, ch);
            if g.w > 0.0 {
                let (w, h) = (g.w * scale, g.h * scale);
                // the middle of the glyph, measured from the start of the baseline
                let mx = pen + g.ox * scale + w / 2.0;
                let my = -(g.oy + g.h) * scale + h / 2.0;
                draw_texture_ex(
                    &self.glyphs,
                    x + mx * cos - my * sin - w / 2.0,
                    y + mx * sin + my * cos - h / 2.0,
                    color,
                    DrawTextureParams { dest_size: Some(vec2(w, h)), source: Some(Rect::new(g.x, g.y, g.w, g.h)), rotation: rot, ..Default::default() },
                );
            }
            pen += g.adv * scale;
        }
    }

    /// One line of text. `anchor` is 0 for left-aligned at `x`, 0.5 centred,
    /// 1 right-aligned; `cy` is the vertical middle of the capitals.
    /// `outline` draws a contrasting edge of the given thickness.
    #[allow(clippy::too_many_arguments)]
    pub fn label(&self, s: &str, x: f32, cy: f32, size: f32, anchor: f32, color: Color, outline: Option<(Color, f32)>, rot: f32) {
        if s.is_empty() || color.a <= 0.003 || size < 1.0 {
            return;
        }
        let w = self.text_w(s, size);
        // from the anchor point to the text origin (baseline, left edge)
        let (ox, oy) = (-w * anchor, size * 0.35);
        let (sin, cos) = rot.sin_cos();
        let (bx, by) = (x + ox * cos - oy * sin, cy + ox * sin + oy * cos);
        if let Some((oc, th)) = outline {
            let oc = alpha(oc, color.a);
            let n = if th > size * 0.07 { 12 } else { 8 };
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                self.raw_text(s, bx + a.cos() * th, by + a.sin() * th, size, rot, oc);
            }
            // a little extra weight underneath, like a soft drop shadow
            self.raw_text(s, bx, by + th * 1.6, size, rot, oc);
        }
        self.raw_text(s, bx, by, size, rot, color);
    }

    /// Plain centred text.
    pub fn text_c(&self, s: &str, cx: f32, cy: f32, size: f32, color: Color) {
        self.label(s, cx, cy, size, 0.5, color, None, 0.0);
    }

    /// Centred white-on-dark outlined text, the house style for headings.
    pub fn title(&self, s: &str, cx: f32, cy: f32, size: f32, color: Color, edge: Color) {
        self.label(s, cx, cy, size, 0.5, color, Some((edge, (size * 0.075).max(1.2))), 0.0);
    }

    /// Breaks text into lines no wider than `max_w`.
    pub fn wrap(&self, s: &str, size: f32, max_w: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split_whitespace() {
                let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && self.text_w(&trial, size) > max_w {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                } else {
                    line = trial;
                }
            }
            lines.push(line);
        }
        lines
    }

    /// Wrapped, centred paragraph. Returns the height used.
    pub fn para(&self, s: &str, cx: f32, top: f32, size: f32, max_w: f32, color: Color) -> f32 {
        let lh = size * 1.18;
        let lines = self.wrap(s, size, max_w);
        for (i, line) in lines.iter().enumerate() {
            self.text_c(line, cx, top + lh * (i as f32 + 0.5), size, color);
        }
        lh * lines.len() as f32
    }

    pub fn para_height(&self, s: &str, size: f32, max_w: f32) -> f32 {
        size * 1.18 * self.wrap(s, size, max_w).len() as f32
    }
}

// ---- easing ---------------------------------------------------------------------

pub fn clamp01(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}

pub fn ease_out(t: f32) -> f32 {
    let t = 1.0 - clamp01(t);
    1.0 - t * t * t
}

pub fn ease_io(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

/// Overshoots past 1 and settles back: the "pop" in pop-in.
pub fn ease_back(t: f32) -> f32 {
    let t = clamp01(t) - 1.0;
    let s = 1.9;
    t * t * ((s + 1.0) * t + s) + 1.0
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
