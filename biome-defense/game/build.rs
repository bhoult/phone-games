//! Packs the art at build time so the game ships as one self-contained wasm
//! file with no runtime asset requests.
//!
//!   assets/svg/atlas/**.svg  \  one packed texture atlas + `spr::NAME` rects.
//!   assets/img/atlas/**.png  /  SVGs (interface, effects) are rendered at
//!                               their own width/height; PNGs (the photo
//!                               cut-outs made by tools/cut.py) go in as is.
//!   assets/img/tex/*.jpg     -> `tex::NAME`, ground and trail surfaces
//!   assets/img/scenes/*.jpg  -> `scene::NAME`, whole pictures
//!   assets/img/icon-*.png    -> PWA icons copied to ../web/icons/
//!   assets/font/*.ttf        -> one texture of every glyph at a few sizes
//!
//! The atlas is stored as a JPEG of its colour and a PNG of its coverage:
//! photographs make a huge PNG, and the split is a fifth of the size.

use resvg::{tiny_skia, usvg};
use std::{
    env, fs,
    io::BufWriter,
    path::{Path, PathBuf},
};

const ATLAS_W: u32 = 2048;
const PAD: u32 = 3;

struct Img {
    name: String,
    w: u32,
    h: u32,
    /// The SVG's viewBox: where this sprite sits in its drawing's own
    /// coordinate space. Character parts share one space, so this is what
    /// lets the game put a character back together.
    view: [f32; 4],
    /// Straight (non-premultiplied) RGBA.
    px: Vec<u8>,
}

fn render(path: &Path, size: Option<u32>) -> Img {
    let data = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&data, &opt)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let sz = tree.size();
    let (w, h, sx, sy) = match size {
        Some(s) => (s, s, s as f32 / sz.width(), s as f32 / sz.height()),
        None => (sz.width().ceil() as u32, sz.height().ceil() as u32, 1.0, 1.0),
    };
    let mut pixmap = tiny_skia::Pixmap::new(w, h).expect("pixmap");
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    // tiny-skia is premultiplied; the game blends with straight alpha.
    let mut px = pixmap.take();
    for p in px.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut p[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    let text = String::from_utf8_lossy(&data);
    let view = text
        .split_once("viewBox=\"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(vb, _)| vb.split_whitespace().filter_map(|n| n.parse::<f32>().ok()).collect::<Vec<_>>())
        .filter(|v| v.len() == 4)
        .map(|v| [v[0], v[1], v[2], v[3]])
        .unwrap_or([0.0, 0.0, sz.width(), sz.height()]);
    let name = path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_uppercase()
        .replace(['-', ' '], "_");
    Img { name, w, h, view, px }
}

/// Fully transparent pixels still contribute their RGB when the GPU filters a
/// sprite edge. Push neighbouring colour outwards so edges don't go dark.
fn bleed(px: &mut [u8], w: usize, h: usize, passes: usize) {
    let mut known: Vec<bool> = px.chunks_exact(4).map(|p| p[3] > 0).collect();
    for _ in 0..passes {
        let mut fills: Vec<(usize, [u8; 3])> = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if known[i] {
                    continue;
                }
                let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    let j = ny as usize * w + nx as usize;
                    if known[j] {
                        r += px[j * 4] as u32;
                        g += px[j * 4 + 1] as u32;
                        b += px[j * 4 + 2] as u32;
                        n += 1;
                    }
                }
                if n > 0 {
                    fills.push((i, [(r / n) as u8, (g / n) as u8, (b / n) as u8]));
                }
            }
        }
        if fills.is_empty() {
            break;
        }
        for (i, c) in fills {
            px[i * 4..i * 4 + 3].copy_from_slice(&c);
            known[i] = true;
        }
    }
}

fn files_in(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files_in(&p, ext, out);
        } else if p.extension().is_some_and(|e| e == ext) {
            out.push(p);
        }
    }
}

fn const_name(path: &Path) -> String {
    path.file_stem().unwrap().to_string_lossy().to_uppercase().replace(['-', ' '], "_")
}

/// Reads a PNG as straight RGBA.
fn load_png(path: &Path) -> Img {
    let file = fs::File::open(path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let mut dec = png::Decoder::new(file);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let data = &buf[..info.buffer_size()];
    let n = (info.width * info.height) as usize;
    let mut px = Vec::with_capacity(n * 4);
    match info.color_type {
        png::ColorType::Rgba => px.extend_from_slice(data),
        png::ColorType::Rgb => data.chunks_exact(3).for_each(|p| px.extend_from_slice(&[p[0], p[1], p[2], 255])),
        png::ColorType::GrayscaleAlpha => data.chunks_exact(2).for_each(|p| px.extend_from_slice(&[p[0], p[0], p[0], p[1]])),
        png::ColorType::Grayscale => data.iter().for_each(|&v| px.extend_from_slice(&[v, v, v, 255])),
        other => panic!("{}: unexpected PNG colour type {other:?}", path.display()),
    }
    Img { name: const_name(path), w: info.width, h: info.height, view: [0.0, 0.0, info.width as f32, info.height as f32], px }
}

fn write_gray(path: &Path, w: u32, h: u32, px: &[u8]) {
    let file = fs::File::create(path).unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    let mut enc = png::Encoder::new(BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Best);
    let mut writer = enc.write_header().unwrap();
    writer.write_image_data(px).unwrap();
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let svg = root.join("assets/svg");
    let img = root.join("assets/img");
    println!("cargo:rerun-if-changed=assets/svg");
    println!("cargo:rerun-if-changed=assets/img");
    println!("cargo:rerun-if-changed=build.rs");

    let mut code = String::new();

    // ---- atlas ---------------------------------------------------------
    let mut files = Vec::new();
    files_in(&svg.join("atlas"), "svg", &mut files);
    let mut imgs: Vec<Img> = files.iter().map(|p| render(p, None)).collect();
    let mut files = Vec::new();
    files_in(&img.join("atlas"), "png", &mut files);
    imgs.extend(files.iter().map(|p| load_png(p)));
    imgs.sort_by(|a, b| b.h.cmp(&a.h).then(a.name.cmp(&b.name)));
    for pair in imgs.windows(2) {
        assert!(pair[0].name != pair[1].name || pair[0].h != pair[1].h, "two sprites are both called {}", pair[0].name);
    }

    let mut placed: Vec<(u32, u32)> = Vec::new();
    let (mut x, mut y, mut row_h) = (PAD, PAD, 0u32);
    for im in &imgs {
        assert!(im.w + 2 * PAD <= ATLAS_W, "{} is wider than the atlas", im.name);
        if x + im.w + PAD > ATLAS_W {
            x = PAD;
            y += row_h + PAD;
            row_h = 0;
        }
        placed.push((x, y));
        x += im.w + PAD;
        row_h = row_h.max(im.h);
    }
    // a multiple of 8 so the JPEG has no part-blocks; no need for a power of two
    let atlas_h = ((y + row_h + PAD).div_ceil(8) * 8).max(64);
    assert!(atlas_h <= 4096, "atlas grew past 4096px tall ({atlas_h}); split it or shrink some art");
    let mut atlas = vec![0u8; (ATLAS_W * atlas_h * 4) as usize];
    for (im, &(px, py)) in imgs.iter().zip(&placed) {
        for row in 0..im.h {
            let src = (row * im.w * 4) as usize;
            let dst = (((py + row) * ATLAS_W + px) * 4) as usize;
            atlas[dst..dst + (im.w * 4) as usize]
                .copy_from_slice(&im.px[src..src + (im.w * 4) as usize]);
        }
    }
    // Colour is pushed well out past every edge: the JPEG smears a little
    // across them, and what it smears in should be the sprite's own colour.
    bleed(&mut atlas, ATLAS_W as usize, atlas_h as usize, 6);
    let mut rgb = Vec::with_capacity((ATLAS_W * atlas_h * 3) as usize);
    let mut cover = Vec::with_capacity((ATLAS_W * atlas_h) as usize);
    for p in atlas.chunks_exact(4) {
        // what is left untouched is far from any sprite: mid grey compresses to nothing
        let far = p[3] == 0 && p[0] == 0 && p[1] == 0 && p[2] == 0;
        rgb.extend_from_slice(if far { &[96, 96, 96] } else { &p[..3] });
        cover.push(p[3]);
    }
    let mut enc = jpeg_encoder::Encoder::new_file(out.join("atlas.jpg"), 90).expect("atlas.jpg");
    enc.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_4_4);
    enc.encode(&rgb, ATLAS_W as u16, atlas_h as u16, jpeg_encoder::ColorType::Rgb).expect("encode atlas");
    write_gray(&out.join("atlas_a.png"), ATLAS_W, atlas_h, &cover);

    code.push_str(
        "pub const ATLAS_JPG: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/atlas.jpg\"));\n\
         pub const ATLAS_COVER_PNG: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/atlas_a.png\"));\n\
         #[allow(dead_code)]\npub mod spr {\n    use super::Spr;\n",
    );
    let mut named: Vec<(&Img, &(u32, u32))> = imgs.iter().zip(&placed).collect();
    named.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    for (im, (px, py)) in named {
        let [vx, vy, vw, vh] = im.view;
        code.push_str(&format!(
            "    pub const {}: Spr = Spr {{ x: {}.0, y: {}.0, w: {}.0, h: {}.0, vx: {:?}, vy: {:?}, vw: {:?}, vh: {:?} }};\n",
            im.name, px, py, im.w, im.h, vx, vy, vw, vh
        ));
    }
    code.push_str("}\n");

    // ---- surfaces and whole pictures -------------------------------------
    // JPEGs, embedded as they are and decoded the first time each is shown.
    // `tex::ALL` / `scene::ALL` hold the files; `NAME` is each one's index.
    for (module, dir) in [("tex", "tex"), ("scene", "scenes")] {
        let mut files = Vec::new();
        files_in(&img.join(dir), "jpg", &mut files);
        code.push_str(&format!("#[allow(dead_code)]\npub mod {module} {{\n"));
        let mut all = String::new();
        for (i, p) in files.iter().enumerate() {
            code.push_str(&format!("    pub const {}: usize = {};\n", const_name(p), i));
            all.push_str(&format!("        include_bytes!({:?}),\n", p.display().to_string()));
        }
        code.push_str(&format!("    pub const ALL: &[&[u8]] = &[\n{}    ];\n}}\n", all));
    }

    // ---- fonts ---------------------------------------------------------
    // Every printable ASCII glyph of each face at a handful of sizes, packed
    // into one texture. The game draws text from this and never asks the
    // engine to rasterise a glyph: the engine's own glyph cache grows to
    // fit whatever is asked of it, and on some screens' pixel ratios it
    // grew past what a phone's GPU will take. A fixed texture is the same
    // on every device.
    println!("cargo:rerun-if-changed=assets/font");
    // (file, sizes): headings, running text, and a heavier text for numbers
    const FACES: [(&str, &[u32]); 3] = [
        ("Cinzel-Bold.ttf", &[24, 38, 60, 96, 150]),
        ("AlegreyaSans-Medium.ttf", &[18, 28, 44, 68]),
        ("AlegreyaSans-Bold.ttf", &[18, 28, 44, 68, 104]),
    ];
    const FONT_W: u32 = 2048;
    type Set = Vec<(fontdue::Metrics, Vec<u8>, (u32, u32))>;
    let mut faces: Vec<Vec<Set>> = Vec::new();
    let (mut x, mut y, mut row_h) = (PAD, PAD, 0u32);
    // every (face, size), biggest first: glyphs of one size are about one
    // height, so rows pack tight
    let mut order: Vec<(usize, usize)> = Vec::new();
    let mut fonts = Vec::new();
    for (fi, (file, sizes)) in FACES.iter().enumerate() {
        let ttf = fs::read(root.join("assets/font").join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        fonts.push(fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).expect("parse font"));
        faces.push(vec![Vec::new(); sizes.len()]);
        for si in 0..sizes.len() {
            order.push((fi, si));
        }
    }
    order.sort_by_key(|&(fi, si)| std::cmp::Reverse(FACES[fi].1[si]));
    for (fi, si) in order {
        for ch in 32u8..127 {
            let (m, bitmap) = fonts[fi].rasterize(ch as char, FACES[fi].1[si] as f32);
            let (w, h) = (m.width as u32, m.height as u32);
            if x + w + PAD > FONT_W {
                x = PAD;
                y += row_h + PAD;
                row_h = 0;
            }
            faces[fi][si].push((m, bitmap, (x, y)));
            x += w + PAD;
            row_h = row_h.max(h);
        }
    }
    let font_h = (y + row_h + PAD).next_power_of_two();
    assert!(font_h <= 2048, "the font texture grew past 2048px tall; drop a size");
    let mut cover = vec![0u8; (FONT_W * font_h) as usize];
    for face in &faces {
        for set in face {
            for (m, bitmap, (gx, gy)) in set {
                for row in 0..m.height as u32 {
                    for col in 0..m.width as u32 {
                        cover[((gy + row) * FONT_W + gx + col) as usize] = bitmap[(row * m.width as u32 + col) as usize];
                    }
                }
            }
        }
    }
    // only the coverage is stored; the game makes it white-with-alpha when it loads
    write_gray(&out.join("font.png"), FONT_W, font_h, &cover);
    code.push_str("pub const FONT_PNG: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/font.png\"));\n");
    code.push_str(
        "/// One glyph in the font texture: its rect there, where its bitmap sits\n\
         /// against the pen (left edge, and bottom edge above the baseline), and how\n\
         /// far the pen moves on.\n\
         #[derive(Clone, Copy)]\npub struct Glyph { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub ox: f32, pub oy: f32, pub adv: f32 }\n\
         /// One face at one size.\n\
         pub struct GlyphSet { pub size: f32, pub glyphs: [Glyph; 95] }\n",
    );
    // (glyph measurements are whatever they are; a width of 2.718 is not a mistyped e)
    code.push_str(&format!("/// Each face's sizes, smallest first.\n#[allow(clippy::approx_constant)]\npub static FACES: [&[GlyphSet]; {}] = [\n", FACES.len()));
    for (fi, face) in faces.iter().enumerate() {
        code.push_str("    &[\n");
        for (si, set) in face.iter().enumerate() {
            code.push_str(&format!("        GlyphSet {{ size: {:?}, glyphs: [\n", FACES[fi].1[si] as f32));
            for (m, _, (gx, gy)) in set {
                code.push_str(&format!(
                    "            Glyph {{ x: {}.0, y: {}.0, w: {}.0, h: {}.0, ox: {}.0, oy: {}.0, adv: {:?} }},\n",
                    gx, gy, m.width, m.height, m.xmin, m.ymin, m.advance_width
                ));
            }
            code.push_str("        ] },\n");
        }
        code.push_str("    ],\n");
    }
    code.push_str("];\n");

    fs::write(out.join("atlas.rs"), code).unwrap();

    // ---- PWA icons (copied next to the web shell) -----------------------
    let dir = root.join("../web/icons");
    fs::create_dir_all(&dir).unwrap();
    for size in [180u32, 192, 512] {
        let src = img.join(format!("icon-{size}.png"));
        let Ok(new) = fs::read(&src) else { continue };
        let path = dir.join(format!("icon-{size}.png"));
        // Skip the write when nothing changed so file watchers stay quiet.
        if fs::read(&path).map(|old| old != new).unwrap_or(true) {
            fs::write(&path, new).unwrap();
        }
    }
}
