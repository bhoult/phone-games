//! Rasterises the SVG art at build time so the game ships as one self-contained
//! wasm file with no runtime asset requests.
//!
//!   assets/svg/atlas/**.svg  -> one packed texture atlas + `spr::NAME` rects
//!   assets/svg/scenes/*.svg  -> one PNG each, `scene::NAME`
//!   assets/svg/icon.svg      -> PWA icons written to ../web/icons/
//!   assets/font/*.ttf        -> one texture of every glyph at a few sizes
//!
//! Each SVG is rendered at its own width/height attributes.

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

fn write_png(path: &Path, w: u32, h: u32, px: &[u8]) {
    let file = fs::File::create(path).unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
    let mut enc = png::Encoder::new(BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Best);
    let mut writer = enc.write_header().unwrap();
    writer.write_image_data(px).unwrap();
}

fn svgs_in(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            svgs_in(&p, out);
        } else if p.extension().is_some_and(|e| e == "svg") {
            out.push(p);
        }
    }
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let svg = root.join("assets/svg");
    println!("cargo:rerun-if-changed=assets/svg");
    println!("cargo:rerun-if-changed=build.rs");

    let mut code = String::new();

    // ---- atlas ---------------------------------------------------------
    let mut files = Vec::new();
    svgs_in(&svg.join("atlas"), &mut files);
    let mut imgs: Vec<Img> = files.iter().map(|p| render(p, None)).collect();
    imgs.sort_by(|a, b| b.h.cmp(&a.h).then(a.name.cmp(&b.name)));

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
    let atlas_h = (y + row_h + PAD).next_power_of_two().max(64);
    assert!(atlas_h <= 4096, "atlas grew past 4096px tall; split it");
    let mut atlas = vec![0u8; (ATLAS_W * atlas_h * 4) as usize];
    for (im, &(px, py)) in imgs.iter().zip(&placed) {
        for row in 0..im.h {
            let src = (row * im.w * 4) as usize;
            let dst = (((py + row) * ATLAS_W + px) * 4) as usize;
            atlas[dst..dst + (im.w * 4) as usize]
                .copy_from_slice(&im.px[src..src + (im.w * 4) as usize]);
        }
    }
    bleed(&mut atlas, ATLAS_W as usize, atlas_h as usize, 2);
    write_png(&out.join("atlas.png"), ATLAS_W, atlas_h, &atlas);

    code.push_str(&format!(
        "pub const ATLAS_PNG: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/atlas.png\"));\n\
         #[allow(dead_code)]\npub mod spr {{\n    use super::Spr;\n"
    ));
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

    // ---- scenes --------------------------------------------------------
    // `scene::ALL` holds the PNGs; `scene::NAME` is each one's index into it.
    let mut files = Vec::new();
    svgs_in(&svg.join("scenes"), &mut files);
    code.push_str("#[allow(dead_code)]\npub mod scene {\n");
    let mut all = String::new();
    for (i, p) in files.iter().enumerate() {
        let mut im = render(p, None);
        bleed(&mut im.px, im.w as usize, im.h as usize, 1);
        let file = format!("scene_{}.png", im.name.to_lowercase());
        write_png(&out.join(&file), im.w, im.h, &im.px);
        code.push_str(&format!("    pub const {}: usize = {};\n", im.name, i));
        all.push_str(&format!("        include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{}\")),\n", file));
    }
    code.push_str(&format!("    pub const ALL: &[&[u8]] = &[\n{}    ];\n}}\n", all));

    // ---- font --------------------------------------------------------
    // Every printable ASCII glyph at a handful of sizes, packed into one
    // texture. The game draws text from this and never asks the engine to
    // rasterise a glyph: the engine's own glyph cache grows to fit whatever
    // is asked of it, and on some screens' pixel ratios it grew past what a
    // phone's GPU will take. A fixed texture is the same on every device.
    println!("cargo:rerun-if-changed=assets/font");
    const FONT_SIZES: [u32; 6] = [18, 28, 44, 68, 104, 160];
    const FONT_W: u32 = 2048;
    let ttf = fs::read(root.join("assets/font/LilitaOne-Regular.ttf")).expect("font");
    let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).expect("parse font");
    let mut glyphs: Vec<Vec<(fontdue::Metrics, Vec<u8>, (u32, u32))>> = Vec::new();
    let (mut x, mut y, mut row_h) = (PAD, PAD, 0u32);
    // biggest first, a size at a time: glyphs of one size are about one height, so rows pack tight
    let mut order: Vec<usize> = (0..FONT_SIZES.len()).collect();
    order.reverse();
    glyphs.resize_with(FONT_SIZES.len(), Vec::new);
    for si in order {
        for ch in 32u8..127 {
            let (m, bitmap) = font.rasterize(ch as char, FONT_SIZES[si] as f32);
            let (w, h) = (m.width as u32, m.height as u32);
            if x + w + PAD > FONT_W {
                x = PAD;
                y += row_h + PAD;
                row_h = 0;
            }
            glyphs[si].push((m, bitmap, (x, y)));
            x += w + PAD;
            row_h = row_h.max(h);
        }
    }
    let font_h = (y + row_h + PAD).next_power_of_two();
    assert!(font_h <= 2048, "the font texture grew past 2048px tall; drop a size");
    // white everywhere, the glyph in the alpha: no dark fringes however it is filtered
    let mut px = vec![255u8; (FONT_W * font_h * 4) as usize];
    for p in px.chunks_exact_mut(4) {
        p[3] = 0;
    }
    for set in &glyphs {
        for (m, bitmap, (gx, gy)) in set {
            for row in 0..m.height as u32 {
                for col in 0..m.width as u32 {
                    px[(((gy + row) * FONT_W + gx + col) * 4 + 3) as usize] = bitmap[(row * m.width as u32 + col) as usize];
                }
            }
        }
    }
    write_png(&out.join("font.png"), FONT_W, font_h, &px);
    code.push_str("pub const FONT_PNG: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/font.png\"));\n");
    code.push_str(&format!("pub const FONT_SIZES: [f32; {}] = {:?};\n", FONT_SIZES.len(), FONT_SIZES.map(|s| s as f32)));
    code.push_str(
        "/// One glyph in the font texture: its rect there, where its bitmap sits\n\
         /// against the pen (left edge, and bottom edge above the baseline), and how\n\
         /// far the pen moves on.\n\
         #[derive(Clone, Copy)]\npub struct Glyph { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub ox: f32, pub oy: f32, pub adv: f32 }\n",
    );
    code.push_str(&format!("pub static GLYPHS: [[Glyph; 95]; {}] = [\n", FONT_SIZES.len()));
    for set in &glyphs {
        code.push_str("    [\n");
        for (m, _, (gx, gy)) in set {
            code.push_str(&format!(
                "        Glyph {{ x: {}.0, y: {}.0, w: {}.0, h: {}.0, ox: {}.0, oy: {}.0, adv: {:?} }},\n",
                gx, gy, m.width, m.height, m.xmin, m.ymin, m.advance_width
            ));
        }
        code.push_str("    ],\n");
    }
    code.push_str("];\n");

    fs::write(out.join("atlas.rs"), code).unwrap();

    // ---- PWA icons (written next to the web shell) ---------------------
    let icon = svg.join("icon.svg");
    if icon.exists() {
        let dir = root.join("../web/icons");
        fs::create_dir_all(&dir).unwrap();
        for size in [180u32, 192, 512] {
            let im = render(&icon, Some(size));
            let path = dir.join(format!("icon-{size}.png"));
            // Skip the write when nothing changed so file watchers stay quiet.
            let tmp = out.join(format!("icon-{size}.png"));
            write_png(&tmp, im.w, im.h, &im.px);
            let new = fs::read(&tmp).unwrap();
            if fs::read(&path).map(|old| old != new).unwrap_or(true) {
                fs::write(&path, new).unwrap();
            }
        }
    }
}
