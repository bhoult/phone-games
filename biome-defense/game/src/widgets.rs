//! The look of the interface, and small pieces of it shared between screens.
//!
//! Everything sits on dark plates with pale text, so the photographs
//! underneath carry the colour and the interface stays out of their way.

use crate::audio::Sfx;
use crate::gfx::*;
use crate::ui::*;
use crate::App;
use macroquad::prelude::{draw_arc, draw_rectangle, Color, Rect};

/// Pale text on dark.
pub const INK: Color = rgb(0xe9e2d0);
/// Quieter text: captions, things that are not the point.
pub const DIM: Color = rgb(0xa39d8c);
/// The dark that plates and text shadows are made of.
pub const SHADE: Color = rgb(0x0b0d0a);
/// What can be spent.
pub const GOLD: Color = rgb(0xd9b25a);
/// Harm, loss, refusal.
pub const BLOOD: Color = rgb(0xc4553f);
/// Health, growth, yes.
pub const MOSS: Color = rgb(0x8fae5c);
/// Cold, slowed.
pub const FROST: Color = rgb(0x9cc4d8);

/// A dark plate with a thin pale edge.
pub fn panel(g: &Gfx, r: Rect, u: f32, color: Color) {
    g.nine(spr::PANEL, r, 32.0, 14.0 * u, color);
}

/// A dark translucent backing for numbers over the playfield.
pub fn chip(g: &Gfx, r: Rect, a: f32) {
    g.nine(spr::CHIP, r, 24.0, (r.h * 0.3).min(r.w * 0.3), alpha(WHITE, a));
}

/// A flat bar, filled from the left.
pub fn bar(_g: &Gfx, r: Rect, frac: f32, color: Color, back: Color) {
    draw_rectangle(r.x, r.y, r.w, r.h, back);
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        draw_rectangle(r.x, r.y, r.w * f, r.h, color);
    }
}

/// A plain little health bar for things on the field.
pub fn tiny_bar(x: f32, y: f32, w: f32, h: f32, frac: f32, color: Color) {
    draw_rectangle(x - w / 2.0 - 1.0, y - 1.0, w + 2.0, h + 2.0, alpha(SHADE, 0.8));
    draw_rectangle(x - w / 2.0, y, w * frac.clamp(0.0, 1.0), h, color);
}

/// Up to three marks in a row, `got` of them lit.
pub fn stars(g: &Gfx, cx: f32, cy: f32, size: f32, got: u8) {
    for i in 0..3 {
        let x = cx + (i as f32 - 1.0) * size * 1.15;
        g.spr_fit(spr::IC_STAR, x, cy, size, 0.0, if i < got { GOLD } else { alpha(INK, 0.16) });
    }
}

/// A forage mark and a number, centred on `cx`. Red when it cannot be afforded.
pub fn price(g: &Gfx, cx: f32, cy: f32, size: f32, n: u32, ok: bool) {
    let text = n.to_string();
    let w = g.text_w(&text, size) + size * 0.95;
    let col = if ok { GOLD } else { BLOOD };
    g.spr_fit(spr::IC_SEED, cx - w / 2.0 + size * 0.38, cy, size * 0.82, 0.0, col);
    g.label(&text, cx - w / 2.0 + size * 0.9, cy, size, 0.0, col, None, 0.0);
}

/// A ring that empties as something recharges. `frac` is how much is left
/// to wait.
pub fn cooldown_ring(cx: f32, cy: f32, r: f32, thick: f32, frac: f32, color: Color) {
    if frac > 0.0 {
        draw_arc(cx, cy, 40, r, -90.0, thick, 360.0 * (1.0 - frac.clamp(0.0, 1.0)), color);
    }
}

/// A round portrait with a thin ring round it.
pub fn portrait(g: &Gfx, face: Spr, cx: f32, cy: f32, size: f32, ring: Color, tint: Color) {
    g.spr(face, cx, cy, size, size, 0.0, tint);
    g.spr(spr::RIM, cx, cy, size * 1.06, size * 1.06, 0.0, ring);
}

/// The sound and music switches, side by side. `id` and `id + 1` are used
/// for the two buttons.
pub fn sound_toggles(app: &mut App, cx: f32, y: f32, id: u32) {
    let u = app.u;
    let s = 44.0 * u;
    let sfx = Rect::new(cx - s - 5.0 * u, y, s, s);
    let mus = Rect::new(cx + 5.0 * u, y, s, s);
    let (sfx_on, music_on) = (app.save.sfx, app.save.music);
    if app.ui.button(&app.g, &mut app.au, id, sfx, if sfx_on { STEEL } else { GREY }, "", Some(if sfx_on { spr::IC_SOUND } else { spr::IC_MUTE }), true) {
        app.save.sfx = !sfx_on;
        app.au.sfx_on = !sfx_on;
        app.save.store();
        app.au.play(Sfx::Click, 0.8);
    }
    if app.ui.button(&app.g, &mut app.au, id + 1, mus, if music_on { STEEL } else { GREY }, "", Some(if music_on { spr::IC_MUSIC } else { spr::IC_NOMUSIC }), true) {
        app.save.music = !music_on;
        app.au.music_on = !music_on;
        app.save.store();
    }
}

/// Darkens everything behind a card. Returns true if the dark part was
/// tapped (which usually means "close").
pub fn dim(app: &App, a: f32, keep: Rect) -> bool {
    app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(SHADE, a));
    app.ui.ptr.released && !keep.contains(app.ui.ptr.pos) && !keep.contains(app.ui.ptr.press_pos) && app.ui.ptr.travel < 14.0 * app.u
}
