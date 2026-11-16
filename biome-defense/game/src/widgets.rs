//! Small pieces of interface shared between screens.

use crate::audio::Sfx;
use crate::gfx::*;
use crate::ui::*;
use crate::App;
use macroquad::prelude::{draw_arc, draw_rectangle, Color, Rect};

pub const CREAM: Color = rgb(0xfff6e0);
pub const BARK: Color = rgb(0x4a2c12);
pub const SOIL: Color = rgb(0x2a1a0c);

/// A cream card with a brown edge.
pub fn panel(g: &Gfx, r: Rect, u: f32, color: Color) {
    g.nine(spr::PANEL, r, 32.0, 22.0 * u, color);
}

/// A dark translucent backing for numbers over the playfield.
pub fn chip(g: &Gfx, r: Rect, a: f32) {
    g.nine(spr::CHIP, r, 24.0, (r.h * 0.5).min(r.w * 0.5), alpha(WHITE, a));
}

/// A rounded bar, filled from the left.
pub fn bar(g: &Gfx, r: Rect, frac: f32, color: Color, back: Color) {
    g.nine(spr::PILL, r, 31.0, r.h * 0.5, back);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.02 {
        let inner = Rect::new(r.x + r.h * 0.12, r.y + r.h * 0.12, ((r.w - r.h * 0.24) * f).max(r.h * 0.76), r.h * 0.76);
        g.nine(spr::PILL, inner, 31.0, inner.h * 0.5, color);
    }
}

/// A plain little health bar for things on the field.
pub fn tiny_bar(x: f32, y: f32, w: f32, h: f32, frac: f32, color: Color) {
    draw_rectangle(x - w / 2.0 - h * 0.25, y - h * 0.25, w + h * 0.5, h * 1.5, alpha(SOIL, 0.75));
    draw_rectangle(x - w / 2.0, y, w * frac.clamp(0.0, 1.0), h, color);
}

/// Up to three stars in a row, `got` of them gold.
pub fn stars(g: &Gfx, cx: f32, cy: f32, size: f32, got: u8) {
    for i in 0..3 {
        let x = cx + (i as f32 - 1.0) * size * 1.05;
        let lift = if i == 1 { -size * 0.12 } else { 0.0 };
        g.spr_fit(if i < got { spr::IC_STAR } else { spr::IC_STAR_GRAY }, x, cy + lift, size, (i as f32 - 1.0) * 0.18, WHITE);
    }
}

/// A seed icon and a number, centred on `cx`. Red when it cannot be afforded.
pub fn price(g: &Gfx, cx: f32, cy: f32, size: f32, n: u32, ok: bool) {
    let text = n.to_string();
    let w = g.text_w(&text, size) + size * 1.1;
    g.spr_fit(spr::IC_SEED, cx - w / 2.0 + size * 0.45, cy, size * 1.05, 0.0, WHITE);
    g.label(&text, cx - w / 2.0 + size * 1.05, cy, size, 0.0, if ok { WHITE } else { rgb(0xff8a7a) }, Some((alpha(SOIL, 0.8), size * 0.09)), 0.0);
}

/// A ring that empties as something recharges. `frac` is how much is left
/// to wait.
pub fn cooldown_ring(cx: f32, cy: f32, r: f32, thick: f32, frac: f32, color: Color) {
    if frac > 0.0 {
        draw_arc(cx, cy, 40, r, -90.0, thick, 360.0 * (1.0 - frac.clamp(0.0, 1.0)), color);
    }
}

/// The sound and music switches, side by side. `id` and `id + 1` are used
/// for the two buttons.
pub fn sound_toggles(app: &mut App, cx: f32, y: f32, id: u32) {
    let u = app.u;
    let s = 46.0 * u;
    let sfx = Rect::new(cx - s - 5.0 * u, y, s, s);
    let mus = Rect::new(cx + 5.0 * u, y, s, s);
    let (sfx_on, music_on) = (app.save.sfx, app.save.music);
    if app.ui.button(&app.g, &mut app.au, id, sfx, if sfx_on { BLUE } else { GREY }, "", Some(if sfx_on { spr::IC_SOUND } else { spr::IC_MUTE }), true) {
        app.save.sfx = !sfx_on;
        app.au.sfx_on = !sfx_on;
        app.save.store();
        app.au.play(Sfx::Click, 0.8);
    }
    if app.ui.button(&app.g, &mut app.au, id + 1, mus, if music_on { BLUE } else { GREY }, "", Some(if music_on { spr::IC_MUSIC } else { spr::IC_NOMUSIC }), true) {
        app.save.music = !music_on;
        app.au.music_on = !music_on;
        app.save.store();
    }
}

/// Darkens everything behind a card. Returns true if the dark part was
/// tapped (which usually means "close").
pub fn dim(app: &App, a: f32, keep: Rect) -> bool {
    app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(rgb(0x0c1408), a));
    app.ui.ptr.released && !keep.contains(app.ui.ptr.pos) && !keep.contains(app.ui.ptr.press_pos) && app.ui.ptr.travel < 14.0 * app.u
}
