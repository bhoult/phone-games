//! Pointer input (one finger or the mouse, whichever is in use) and buttons.

use crate::audio::{Audio, Sfx};
use crate::gfx::*;
use macroquad::prelude::{
    get_time, is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released, mouse_position, screen_dpi_scale, touches,
    vec2, Color, MouseButton, Rect, Touch, TouchPhase, Vec2,
};

#[derive(Default)]
pub struct Pointer {
    pub pos: Vec2,
    pub down: bool,
    /// Went down this frame.
    pub pressed: bool,
    /// Came up this frame.
    pub released: bool,
    pub press_pos: Vec2,
    /// Furthest it has strayed from where it went down: tells taps from drags.
    pub travel: f32,
    touch: Option<u64>,
    quiet_until: f64,
}

impl Pointer {
    /// Reads this frame's input. Touches are tracked directly rather than
    /// through macroquad's mouse emulation, so a second finger or a cancelled
    /// touch can't leave the pointer stuck down.
    pub fn update(&mut self) {
        let dpi = screen_dpi_scale().max(0.5);
        let now = get_time();
        self.pressed = false;
        self.released = false;
        let ts = touches();
        let ended = |t: &Touch| matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);

        if let Some(id) = self.touch {
            self.quiet_until = now + 0.6;
            match ts.iter().find(|t| t.id == id) {
                Some(t) => {
                    self.pos = t.position / dpi;
                    if ended(t) {
                        self.lift();
                    }
                }
                None => self.lift(),
            }
        } else if let Some(t) = ts.first() {
            self.quiet_until = now + 0.6;
            self.pos = t.position / dpi;
            self.press();
            if ended(t) {
                // down and up inside one frame: a very quick tap
                self.lift();
            } else {
                self.touch = Some(t.id);
            }
        } else if now >= self.quiet_until {
            let (mx, my) = mouse_position();
            self.pos = vec2(mx, my);
            if is_mouse_button_pressed(MouseButton::Left) {
                self.press();
            }
            if self.down && (is_mouse_button_released(MouseButton::Left) || !is_mouse_button_down(MouseButton::Left)) {
                self.lift();
            }
        }
        if self.down {
            self.travel = self.travel.max(self.pos.distance(self.press_pos));
        }
    }

    fn press(&mut self) {
        self.pressed = true;
        self.down = true;
        self.press_pos = self.pos;
        self.travel = 0.0;
    }

    fn lift(&mut self) {
        if self.down {
            self.released = true;
        }
        self.down = false;
        self.touch = None;
    }
}

pub struct Ui {
    pub ptr: Pointer,
    /// Button the pointer went down on.
    active: Option<u32>,
    /// Reference unit: 1/390 of the play column's width.
    pub u: f32,
}

pub const GREEN: Color = rgb(0x58c93a);
pub const ORANGE: Color = rgb(0xff9a2e);
pub const BLUE: Color = rgb(0x3f9be8);
pub const GREY: Color = rgb(0x9aa3b5);
pub const PURPLE: Color = rgb(0xa86fe8);

impl Ui {
    pub fn new() -> Ui {
        Ui { ptr: Pointer::default(), active: None, u: 1.0 }
    }

    pub fn begin(&mut self) {
        self.ptr.update();
        if !self.ptr.down && !self.ptr.released {
            self.active = None;
        }
    }

    /// Swallow this frame's press/release (after a popup closes, say).
    pub fn eat(&mut self) {
        self.ptr.pressed = false;
        self.ptr.released = false;
        self.active = None;
    }

    /// A tap that started and ended inside `r` without wandering off.
    pub fn tapped(&self, r: Rect) -> bool {
        self.ptr.released && r.contains(self.ptr.pos) && r.contains(self.ptr.press_pos) && self.ptr.travel < 14.0 * self.u
    }

    /// Button logic only. Returns (held down right now, clicked this frame).
    pub fn press(&mut self, id: u32, r: Rect, enabled: bool) -> (bool, bool) {
        if !enabled {
            return (false, false);
        }
        let grown = Rect::new(r.x - 4.0 * self.u, r.y - 4.0 * self.u, r.w + 8.0 * self.u, r.h + 8.0 * self.u);
        if self.ptr.pressed && grown.contains(self.ptr.pos) {
            self.active = Some(id);
        }
        let mine = self.active == Some(id);
        let over = grown.contains(self.ptr.pos);
        let clicked = mine && self.ptr.released && over;
        if clicked {
            self.active = None;
        }
        (mine && over && self.ptr.down, clicked)
    }

    /// A chunky pill button with a label and/or icon. Returns true on click.
    #[allow(clippy::too_many_arguments)]
    pub fn button(&mut self, g: &Gfx, au: &mut Audio, id: u32, r: Rect, color: Color, label: &str, icon: Option<Spr>, enabled: bool) -> bool {
        let (held, clicked) = self.press(id, r, enabled);
        let dy = if held { r.h * 0.045 } else { 0.0 };
        let tint = if held { mix(color, rgb(0x000000), 0.14) } else { color };
        let body = Rect::new(r.x, r.y + dy, r.w, r.h);
        g.nine(spr::BTN, body, 32.0, (r.h * 0.46).min(r.w * 0.5), tint);
        let cy = body.y + body.h * 0.455;
        let size = (r.h * 0.44).min(30.0 * self.u);
        match (icon, label.is_empty()) {
            (Some(ic), true) => g.spr_fit(ic, body.x + body.w / 2.0, cy, r.h * 0.56, 0.0, WHITE),
            (Some(ic), false) => {
                let tw = g.text_w(label, size);
                let iw = r.h * 0.5;
                let total = tw + iw + 6.0 * self.u;
                let x0 = body.x + (body.w - total) / 2.0;
                g.spr_fit(ic, x0 + iw / 2.0, cy, iw, 0.0, WHITE);
                g.label(label, x0 + iw + 6.0 * self.u, cy, size, 0.0, WHITE, Some((alpha(NAVY, 0.55), size * 0.07)), 0.0);
            }
            (None, _) => g.label(label, body.x + body.w / 2.0, cy, size, 0.5, WHITE, Some((alpha(NAVY, 0.55), size * 0.07)), 0.0),
        }
        if clicked {
            au.play(Sfx::Click, 0.8);
        }
        clicked
    }
}
