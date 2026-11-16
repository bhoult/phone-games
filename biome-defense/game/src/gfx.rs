//! Drawing helpers over macroquad: atlas sprites, nine-slice panels, text
//! from baked glyphs, the ground of a stage, and whole-picture backdrops.

use macroquad::material::{gl_use_default_material, gl_use_material, load_material, Material, MaterialParams};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use std::cell::RefCell;

// The engine's own vertex shader, unchanged.
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

/// The ground shader's vertex half: the engine's, but handing the texture
/// coordinate on at full precision. The ground works out where on the map
/// each pixel is from it, across a whole screen, and at the "low" precision
/// the engine's own shader asks for some phones would do that in steps.
const GROUND_VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying highp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}"#;

/// The ground of a stage, in one pass over the whole play column: the
/// biome's earth, with the trail and any water worked into it.
///
/// `Texture` is the ground photograph. `Mask` holds, for every point of
/// the map, how near the trail is (red) and how near water is (green), as
/// smooth ramps; the edges are roughened here with a little noise so they
/// read as worn earth, not as a drawn line. Photographs are repeated by
/// mirroring, done by hand: this has to work on WebGL 1, where textures of
/// odd sizes cannot repeat.
///
///   Area   the map coordinates (in tiles) of the quad's corner, and its size
///   MaskAt the map coordinates the mask covers: corner and size
///   Look   (ground photo mirrored left-right, mirrored top-bottom, how much
///          to brighten the trail, time)
///   Grade  colour multiplied in at the end (alpha: strength of the vignette)
const GROUND_FS: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
varying highp vec2 uv;
#else
precision mediump float;
varying mediump vec2 uv;
#endif
uniform sampler2D Texture;
uniform sampler2D Trail;
uniform sampler2D Water;
uniform sampler2D Mask;
uniform vec4 Area;
uniform vec4 MaskAt;
uniform vec4 Look;
uniform vec4 Grade;
vec2 tri(vec2 v) {
    return 1.0 - abs(mod(v, 2.0) - 1.0);
}
void main() {
    vec2 m = Area.xy + uv * Area.zw;
    vec4 k = texture2D(Mask, clamp((m - MaskAt.xy) / MaskAt.zw, 0.0, 1.0));
    // The ground photo is two wide by three high and spans the map's nine
    // tiles, so it covers all thirteen rows without repeating: the mirrored
    // copies only start off the top and bottom of the map.
    vec2 gp = vec2(m.x / 9.0, (m.y + 0.25) / 13.5);
    gp = mix(gp, 1.0 - gp, Look.xy);
    vec3 c = texture2D(Texture, tri(gp)).rgb;
    vec3 dirt = texture2D(Trail, tri(m / 3.6)).rgb;
    float n = texture2D(Trail, tri(m / 1.3 + 0.37)).g - 0.45;
    float n2 = texture2D(Trail, tri(m.yx / 0.31 + 0.11)).r - 0.45;
    float edge = k.r + n * 0.16 + n2 * 0.05;
    // the verge: trampled and darker, fading into the ground
    c *= 1.0 - 0.34 * smoothstep(0.28, 0.60, edge);
    // the trail itself, a little sunk: darker just inside its edge
    float t = smoothstep(0.545, 0.615, edge);
    float inner = smoothstep(0.60, 0.80, edge);
    c = mix(c, dirt * Look.z * (0.80 + 0.20 * inner), t);
    // water: a wet rim of bank, then the pool, lighter where it catches the sky
    float we = k.g + n * 0.14 + n2 * 0.05;
    // (never over the trail: where they meet, the trail is the bank)
    float w = smoothstep(0.54, 0.60, we) * (1.0 - t);
    c *= 1.0 - 0.5 * smoothstep(0.30, 0.56, we) * (1.0 - t);
    vec2 drift = vec2(Look.w * 0.012, Look.w * 0.007);
    vec3 pond = texture2D(Water, tri(m / 3.1 + drift)).rgb * 0.7 + texture2D(Water, tri(m / 2.3 - drift * 1.7 + 0.5)).rgb * 0.5;
    float sky = smoothstep(0.62, 0.95, we);
    pond = pond * 0.8 + vec3(0.05, 0.09, 0.11) + vec3(0.07, 0.10, 0.12) * sky;
    c = mix(c, pond, w);
    // a darkening toward the corners of the quad
    vec2 q = uv - 0.5;
    c *= 1.0 - Grade.a * smoothstep(0.18, 0.62, dot(q, q) * 1.6);
    gl_FragColor = vec4(c * Grade.rgb, 1.0);
}"#;

/// A sprite in the atlas. `x..h` is its pixel rect; `v*` is the box it was
/// drawn in (an SVG's viewBox; for a photo cut-out, its own size).
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
pub const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);

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

/// The three typefaces baked into the build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    /// Capitals for headings and buttons.
    Head = 0,
    /// Running text.
    Text = 1,
    /// Labels and numbers.
    Bold = 2,
}

fn decode_jpeg(bytes: &[u8]) -> (u16, u16, Vec<u8>) {
    let mut dec = jpeg_decoder::Decoder::new(bytes);
    let px = dec.decode().expect("a picture in the build is not a JPEG");
    let info = dec.info().expect("jpeg info");
    let n = info.width as usize * info.height as usize;
    let mut rgba = Vec::with_capacity(n * 4);
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => px.chunks_exact(3).for_each(|p| rgba.extend_from_slice(&[p[0], p[1], p[2], 255])),
        jpeg_decoder::PixelFormat::L8 => px.iter().for_each(|&v| rgba.extend_from_slice(&[v, v, v, 255])),
        _ => panic!("unsupported JPEG pixel format"),
    }
    (info.width, info.height, rgba)
}

fn jpeg_texture(bytes: &[u8]) -> Texture2D {
    let (w, h, rgba) = decode_jpeg(bytes);
    let t = Texture2D::from_rgba8(w, h, &rgba);
    t.set_filter(FilterMode::Linear);
    t
}

/// What the ground shader needs to know about one stage.
pub struct GroundLook {
    pub ground: usize,
    pub trail: usize,
    pub water: usize,
    pub mask: Texture2D,
    /// Map coordinates the mask covers: (x, y, w, h) in tiles.
    pub mask_at: [f32; 4],
    /// The ground photo mirrored left-right or top-bottom, so the stages of
    /// one biome do not all lie on the same ground.
    pub flip: (bool, bool),
    /// How much to brighten the trail, where the ground is dark enough to lose it.
    pub trail_gain: f32,
    pub grade: Color,
    pub vignette: f32,
}

pub struct Gfx {
    atlas: Texture2D,
    /// Every glyph at a few sizes, made by the build script.
    glyphs: Texture2D,
    /// Surfaces and whole pictures, decoded the first time each is wanted.
    texs: RefCell<Vec<Option<Texture2D>>>,
    scenes: RefCell<Vec<Option<Texture2D>>>,
    pub dpi: f32,
    // None if the device refused the shader: everything then falls back to
    // ordinary drawing, which looks plainer but is never wrong.
    light: Option<Material>,
    ground: Option<Material>,
}

impl Gfx {
    pub fn new() -> Gfx {
        // Text is drawn from a texture of glyphs made at build time, never
        // from the engine's own font cache: see the build script for why.
        // The file holds only coverage; make it white with that as alpha.
        let mut font = Image::from_file_with_format(FONT_PNG, Some(ImageFormat::Png)).expect("font texture");
        for p in font.bytes.chunks_exact_mut(4) {
            p[3] = p[0];
            p[0] = 255;
            p[1] = 255;
            p[2] = 255;
        }
        let glyphs = Texture2D::from_image(&font);
        glyphs.set_filter(FilterMode::Linear);
        // the atlas: colour from a JPEG, coverage from a PNG
        let (w, h, mut rgba) = decode_jpeg(ATLAS_JPG);
        let cover = Image::from_file_with_format(ATLAS_COVER_PNG, Some(ImageFormat::Png)).expect("atlas coverage");
        for (p, c) in rgba.chunks_exact_mut(4).zip(cover.bytes.chunks_exact(4)) {
            p[3] = c[0];
        }
        let atlas = Texture2D::from_rgba8(w, h, &rgba);
        atlas.set_filter(FilterMode::Linear);
        let blend = |onto: BlendFactor| PipelineParams {
            color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), onto)),
            ..Default::default()
        };
        let light = load_material(
            ShaderSource::Glsl { vertex: SPRITE_VS, fragment: LIGHT_FS },
            MaterialParams { pipeline_params: blend(BlendFactor::One), ..Default::default() },
        )
        .ok();
        let ground = load_material(
            ShaderSource::Glsl { vertex: GROUND_VS, fragment: GROUND_FS },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new("Area", UniformType::Float4),
                    UniformDesc::new("MaskAt", UniformType::Float4),
                    UniformDesc::new("Look", UniformType::Float4),
                    UniformDesc::new("Grade", UniformType::Float4),
                ],
                textures: vec!["Trail".to_string(), "Water".to_string(), "Mask".to_string()],
                ..Default::default()
            },
        )
        .ok();
        Gfx {
            atlas,
            glyphs,
            texs: RefCell::new(vec![None; tex::ALL.len()]),
            scenes: RefCell::new(vec![None; scene::ALL.len()]),
            dpi: 1.0,
            light,
            ground,
        }
    }

    pub fn begin_frame(&mut self) {
        self.dpi = screen_dpi_scale().max(0.5);
    }

    /// A surface photograph (`tex::NAME`), decoded on first use.
    pub fn tex(&self, idx: usize) -> Texture2D {
        let mut all = self.texs.borrow_mut();
        let i = idx % all.len();
        all[i].get_or_insert_with(|| jpeg_texture(tex::ALL[i])).clone()
    }

    fn scene_tex(&self, idx: usize) -> Texture2D {
        let mut all = self.scenes.borrow_mut();
        let i = idx % all.len();
        all[i].get_or_insert_with(|| jpeg_texture(scene::ALL[i])).clone()
    }

    /// Lets go of every surface but those listed, so a long session does
    /// not end up holding all six biomes' ground at once.
    pub fn keep_only(&self, keep: &[usize]) {
        for (i, t) in self.texs.borrow_mut().iter_mut().enumerate() {
            if !keep.contains(&i) {
                *t = None;
            }
        }
    }

    // ---- light ---------------------------------------------------------------

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

    // ---- the ground -----------------------------------------------------------

    /// Paints the ground of a stage over `r`, whose top-left corner is at
    /// map coordinates `at` and which is `cell` pixels to the tile. Returns
    /// false if the device has no shader for it (the caller then draws
    /// something plainer).
    pub fn ground(&self, look: &GroundLook, r: Rect, at: Vec2, cell: f32, t: f32) -> bool {
        let Some(m) = &self.ground else { return false };
        let ground = self.tex(look.ground);
        m.set_texture("Trail", self.tex(look.trail));
        m.set_texture("Water", self.tex(look.water));
        m.set_texture("Mask", look.mask.clone());
        m.set_uniform("Area", [at.x, at.y, r.w / cell, r.h / cell]);
        m.set_uniform("MaskAt", look.mask_at);
        m.set_uniform("Look", [look.flip.0 as u8 as f32, look.flip.1 as u8 as f32, look.trail_gain, t]);
        m.set_uniform("Grade", [look.grade.r, look.grade.g, look.grade.b, look.vignette]);
        gl_use_material(m);
        draw_texture_ex(&ground, r.x, r.y, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
        gl_use_default_material();
        true
    }

    /// A surface photograph laid flat over `r`, repeated by mirroring,
    /// `span` pixels to one copy.
    pub fn tiled(&self, idx: usize, r: Rect, span: f32, color: Color) {
        let t = self.tex(idx);
        let (tw, th) = (t.width(), t.height());
        let k = span / tw;
        let (cw, ch) = (tw * k, th * k);
        let mut y = r.y;
        let mut row = 0;
        while y < r.y + r.h {
            let mut x = r.x;
            let mut col = 0;
            while x < r.x + r.w {
                let (w, h) = (cw.min(r.x + r.w - x), ch.min(r.y + r.h - y));
                let (sw, sh) = (w / k, h / k);
                let (fx, fy) = (col % 2 == 1, row % 2 == 1);
                // a flipped copy shows the far part of the photo: take the source from that end
                let sx = if fx { tw - sw } else { 0.0 };
                let sy = if fy { th - sh } else { 0.0 };
                draw_texture_ex(&t, x, y, color, DrawTextureParams { dest_size: Some(vec2(w, h)), source: Some(Rect::new(sx, sy, sw, sh)), flip_x: fx, flip_y: fy, ..Default::default() });
                x += cw;
                col += 1;
            }
            y += ch;
            row += 1;
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

    /// A whole picture filling `r`, cropped (never squashed). `ax`, `ay`
    /// say which part survives the crop: 0 keeps the left/top, 1 the
    /// right/bottom, 0.5 the middle.
    pub fn scene(&self, idx: usize, r: Rect, ax: f32, ay: f32, color: Color) {
        let tex = self.scene_tex(idx);
        let (tw, th) = (tex.width(), tex.height());
        let k = (r.w / tw).max(r.h / th);
        let (sw, sh) = (r.w / k, r.h / k);
        draw_texture_ex(
            &tex,
            r.x,
            r.y,
            color,
            DrawTextureParams {
                dest_size: Some(vec2(r.w, r.h)),
                source: Some(Rect::new((tw - sw) * ax, (th - sh) * ay, sw, sh)),
                ..Default::default()
            },
        );
    }

    /// A band of a picture that covers `whole` (cropped about its middle,
    /// never squashed): only the rows from `y0` to `y1` on screen are drawn,
    /// and they fade from `a0` at the top to `a1` at the bottom. This is
    /// how one biome's ground shades into the next on the map.
    #[allow(clippy::too_many_arguments)]
    pub fn scene_rows(&self, idx: usize, whole: Rect, y0: f32, y1: f32, a0: f32, a1: f32, tint: Color) {
        if y1 <= y0 {
            return;
        }
        let tex = self.scene_tex(idx);
        let (tw, th) = (tex.width(), tex.height());
        let k = (whole.w / tw).max(whole.h / th);
        let (sw, sh) = (whole.w / k, whole.h / k);
        let (sx, sy) = ((tw - sw) * 0.5, (th - sh) * 0.5);
        let (u0, u1) = (sx / tw, (sx + sw) / tw);
        let v = |y: f32| (sy + (y - whole.y) / whole.h * sh) / th;
        let vert = |x: f32, y: f32, u: f32, a: f32| Vertex::new(x, y, 0.0, u, v(y), alpha(tint, a));
        draw_mesh(&Mesh {
            vertices: vec![vert(whole.x, y0, u0, a0), vert(whole.x + whole.w, y0, u1, a0), vert(whole.x + whole.w, y1, u1, a1), vert(whole.x, y1, u0, a1)],
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: Some(tex),
        });
    }

    /// Part of a whole picture: the source rect is given as fractions of it.
    pub fn scene_part(&self, idx: usize, src: Rect, r: Rect, color: Color) {
        let tex = self.scene_tex(idx);
        let (tw, th) = (tex.width(), tex.height());
        draw_texture_ex(
            &tex,
            r.x,
            r.y,
            color,
            DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), source: Some(Rect::new(src.x * tw, src.y * th, src.w * tw, src.h * th)), ..Default::default() },
        );
    }

    /// Width over height of a whole picture.
    pub fn scene_aspect(&self, idx: usize) -> f32 {
        let tex = self.scene_tex(idx);
        tex.width() / tex.height()
    }

    /// Top-to-bottom fade of one colour: `a0` opaque at the top, `a1` at the bottom.
    pub fn fade(&self, r: Rect, c: Color, a0: f32, a1: f32) {
        self.vgrad(r, alpha(c, a0), alpha(c, a1));
    }

    // ---- text ------------------------------------------------------------------

    /// Which of a face's baked sizes to draw text of this size from: the
    /// smallest that is at least as big as the text will be on screen, so it
    /// is only ever scaled down a little (or, for the very biggest headings,
    /// up).
    fn set(&self, face: Face, size: f32) -> (&'static GlyphSet, f32) {
        let sets = FACES[face as usize];
        let px = size * self.dpi * 0.94;
        let set = sets.iter().find(|s| s.size >= px).unwrap_or(&sets[sets.len() - 1]);
        (set, size / set.size)
    }

    fn glyph(set: &'static GlyphSet, ch: char) -> &'static Glyph {
        // the fonts are baked as plain ASCII; anything else shows as a question mark
        let c = if (' '..='~').contains(&ch) { ch } else { '?' };
        &set.glyphs[c as usize - 32]
    }

    pub fn text_w_in(&self, face: Face, s: &str, size: f32) -> f32 {
        let (set, scale) = self.set(face, size);
        s.chars().map(|ch| Self::glyph(set, ch).adv).sum::<f32>() * scale
    }

    /// Width of a label (the bold face).
    pub fn text_w(&self, s: &str, size: f32) -> f32 {
        self.text_w_in(Face::Bold, s, size)
    }

    /// Draws text with its baseline starting at (x, y), turned by `rot`
    /// about that point.
    #[allow(clippy::too_many_arguments)]
    fn raw_text(&self, face: Face, s: &str, x: f32, y: f32, size: f32, rot: f32, color: Color) {
        let (set, scale) = self.set(face, size);
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

    /// One line of text in any face. `anchor` is 0 for left-aligned at `x`,
    /// 0.5 centred, 1 right-aligned; `cy` is the vertical middle of the
    /// capitals. `shadow` puts a soft dark edge of the given spread behind
    /// it, for text over a photograph.
    #[allow(clippy::too_many_arguments)]
    pub fn write(&self, face: Face, s: &str, x: f32, cy: f32, size: f32, anchor: f32, color: Color, shadow: Option<(Color, f32)>, rot: f32) {
        if s.is_empty() || color.a <= 0.003 || size < 1.0 {
            return;
        }
        let w = self.text_w_in(face, s, size);
        // from the anchor point to the text origin (baseline, left edge)
        let cap = if face == Face::Head { 0.36 } else { 0.33 };
        let (ox, oy) = (-w * anchor, size * cap);
        let (sin, cos) = rot.sin_cos();
        let (bx, by) = (x + ox * cos - oy * sin, cy + ox * sin + oy * cos);
        if let Some((oc, th)) = shadow {
            let oc = alpha(oc, color.a * 0.6);
            for (dx, dy) in [(-0.7, -0.5), (0.7, -0.5), (-0.7, 0.9), (0.7, 0.9), (0.0, 1.3)] {
                self.raw_text(face, s, bx + dx * th, by + dy * th, size, rot, oc);
            }
        }
        self.raw_text(face, s, bx, by, size, rot, color);
    }

    /// A label or a number: the bold face.
    #[allow(clippy::too_many_arguments)]
    pub fn label(&self, s: &str, x: f32, cy: f32, size: f32, anchor: f32, color: Color, shadow: Option<(Color, f32)>, rot: f32) {
        self.write(Face::Bold, s, x, cy, size, anchor, color, shadow, rot);
    }

    /// Plain centred label.
    pub fn text_c(&self, s: &str, cx: f32, cy: f32, size: f32, color: Color) {
        self.label(s, cx, cy, size, 0.5, color, None, 0.0);
    }

    /// A centred heading in capitals, with a dark edge behind it.
    pub fn title(&self, s: &str, cx: f32, cy: f32, size: f32, color: Color, edge: Color) {
        self.write(Face::Head, s, cx, cy, size, 0.5, color, Some((edge, (size * 0.05).max(1.0))), 0.0);
    }

    /// A heading in capitals, plain.
    pub fn head(&self, s: &str, x: f32, cy: f32, size: f32, anchor: f32, color: Color) {
        self.write(Face::Head, s, x, cy, size, anchor, color, None, 0.0);
    }

    /// A centred heading with its letters set apart: `gap` is the extra
    /// space between them, as a fraction of the size.
    #[allow(clippy::too_many_arguments)]
    pub fn spaced(&self, s: &str, cx: f32, cy: f32, size: f32, gap: f32, color: Color, shadow: Option<(Color, f32)>) {
        let letters: Vec<String> = s.chars().map(|c| c.to_string()).collect();
        let widths: Vec<f32> = letters.iter().map(|l| self.text_w_in(Face::Head, l, size)).collect();
        let total = widths.iter().sum::<f32>() + gap * size * letters.len().saturating_sub(1) as f32;
        let mut x = cx - total / 2.0;
        for (l, w) in letters.iter().zip(&widths) {
            self.write(Face::Head, l, x, cy, size, 0.0, color, shadow, 0.0);
            x += w + gap * size;
        }
    }

    /// Breaks running text into lines no wider than `max_w`.
    pub fn wrap(&self, s: &str, size: f32, max_w: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split_whitespace() {
                let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && self.text_w_in(Face::Text, &trial, size) > max_w {
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

    /// Wrapped, centred paragraph of running text. Returns the height used.
    pub fn para(&self, s: &str, cx: f32, top: f32, size: f32, max_w: f32, color: Color) -> f32 {
        self.para_at(s, cx, top, size, max_w, 0.5, color)
    }

    /// Wrapped paragraph, aligned by `anchor` about `x`.
    #[allow(clippy::too_many_arguments)]
    pub fn para_at(&self, s: &str, x: f32, top: f32, size: f32, max_w: f32, anchor: f32, color: Color) -> f32 {
        let lh = size * 1.24;
        let lines = self.wrap(s, size, max_w);
        for (i, line) in lines.iter().enumerate() {
            self.write(Face::Text, line, x, top + lh * (i as f32 + 0.5), size, anchor, color, None, 0.0);
        }
        lh * lines.len() as f32
    }

    pub fn para_height(&self, s: &str, size: f32, max_w: f32) -> f32 {
        size * 1.24 * self.wrap(s, size, max_w).len() as f32
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

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
