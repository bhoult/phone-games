//! Biome Defense: a tower defense about wild creatures holding on to
//! their homes.
//!
//! The rules live in the `biome-core` crate. This crate is everything you
//! can see and hear: it draws with macroquad (WebGL in the browser) and
//! compiles to a single self-contained wasm file.

mod art;
mod audio;
mod content;
mod field;
mod fx;
mod gfx;
mod map;
mod pack;
mod platform;
mod play;
mod save;
mod story;
mod synth;
mod title;
mod ui;
mod widgets;

use audio::Audio;
use biome_core::bot::expected_profile;
use biome_core::creature::Cr;
use biome_core::level::Difficulty;
use biome_core::{Rng, LEVEL_COUNT};
use content::Beat;
use gfx::*;
use macroquad::prelude::{get_frame_time, next_frame, screen_height, screen_width, Conf, Rect};
use platform::Query;
use save::Save;
use ui::Ui;

/// Everything a screen needs each frame.
pub struct App {
    pub g: Gfx,
    pub au: Audio,
    pub save: Save,
    pub ui: Ui,
    pub q: Query,
    pub rng: Rng,
    /// Seconds since launch, and this frame's length.
    pub t: f32,
    pub dt: f32,
    pub w: f32,
    pub h: f32,
    /// The portrait column everything is laid out in (the whole screen on a
    /// phone; centred on anything wider).
    pub x0: f32,
    pub pw: f32,
    /// One layout unit: 1/390 of the column width.
    pub u: f32,
    /// Safe-area insets (notch, home bar).
    pub top: f32,
    pub bot: f32,
    /// `?debug=1`: narrate to the console for the test driver.
    pub debug: bool,
    /// The stage just won, so the map can walk the pack on from it.
    pub walk_from: Option<usize>,
    frame: u32,
}

impl App {
    /// In debug mode, tells the test driver where a button is so it can be
    /// tapped by name with a real touch.
    pub fn mark(&self, name: &str, r: Rect) {
        if self.debug && self.frame % 15 == 0 {
            macroquad::logging::info!("BIOME BTN {} {:.0} {:.0}", name, r.x + r.w / 2.0, r.y + r.h / 2.0);
        }
    }

    /// In debug mode, a line for the test driver to wait on.
    pub fn log(&self, what: &str) {
        if self.debug {
            macroquad::logging::info!("BIOME {}", what);
        }
    }

    /// True once every `n` frames.
    pub fn tick(&self, n: u32) -> bool {
        self.frame % n == 0
    }
}

/// A request to change screens.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Nav {
    Title,
    /// Choosing a creature for a new journey.
    Choose,
    Story(Beat),
    Map,
    Pack,
    Play(usize),
}

enum Screen {
    Title(title::Title),
    Choose(title::Choose),
    Story(story::Story),
    Map(map::MapScreen),
    Pack(pack::Pack),
    Play(Box<play::Play>),
}

fn open(app: &mut App, nav: Nav) -> Screen {
    // Nothing but the title and the choosing makes sense without a journey,
    // and the map waits for any story that is due.
    let nav = match nav {
        Nav::Title | Nav::Choose => nav,
        _ if app.save.profile.is_none() => Nav::Choose,
        Nav::Map => match content::next_beat(app.save.p()) {
            Some(beat) => Nav::Story(beat),
            None => Nav::Map,
        },
        other => other,
    };
    match nav {
        Nav::Title => Screen::Title(title::Title::new(app)),
        Nav::Choose => Screen::Choose(title::Choose::new(app)),
        Nav::Story(beat) => Screen::Story(story::Story::new(app, beat)),
        Nav::Map => Screen::Map(map::MapScreen::new(app)),
        Nav::Pack => Screen::Pack(pack::Pack::new(app)),
        Nav::Play(level) => Screen::Play(Box::new(play::Play::new(app, level.min(LEVEL_COUNT - 1)))),
    }
}

fn conf() -> Conf {
    Conf { window_title: "Biome Defense".to_string(), window_width: 430, window_height: 900, high_dpi: true, ..Default::default() }
}

#[macroquad::main(conf)]
async fn main() {
    let q = Query::read();
    let mut save = Save::load();
    // Test switches change this session only. A session that used one never
    // writes to storage, so a faked journey can't overwrite the real one.
    let hero = match q.get("hero") {
        Some("badger") => Some(Cr::Badger),
        Some("owl") => Some(Cr::Owl),
        Some("fox") => Some(Cr::Fox),
        _ => None,
    };
    if q.flag("fresh") {
        save.profile = None;
        save.ephemeral = true;
    }
    if let Some(n) = q.num("unlock") {
        // as if the first n stages were behind us
        let n = (n as usize).min(LEVEL_COUNT);
        let mut p = expected_profile(n, hero.unwrap_or(Cr::Fox));
        if q.flag("story") {
            // the story picks up where a player who had got this far would be
            p.seen = Beat::Intro.bit();
            for b in 0..=n / biome_core::level::PER_BIOME {
                if b < biome_core::BIOMES.len() && b * biome_core::level::PER_BIOME < n {
                    p.seen |= Beat::Arrive(b).bit();
                }
                if (b + 1) * biome_core::level::PER_BIOME <= n {
                    p.seen |= Beat::Held(b).bit();
                }
            }
        } else {
            p.seen = !0;
        }
        save.profile = Some(p);
        save.ephemeral = true;
    } else if let (Some(hero), None) = (hero, save.profile.as_ref()) {
        save.profile = Some(biome_core::Profile::new(hero));
        save.ephemeral = true;
    }
    if q.flag("mute") {
        save.sfx = false;
        save.music = false;
        save.ephemeral = true;
    }
    if let (Some(d), Some(p)) = (q.num("difficulty"), save.profile.as_mut()) {
        p.difficulty = Difficulty::from_index(d as usize);
        save.ephemeral = true;
    }
    let speed = q.num("speed").unwrap_or(1.0).clamp(0.1, 8.0);
    let seed = q.num("seed").map(|s| s as u64).unwrap_or(platform::now_ms() as u64);

    let mut app = App {
        g: Gfx::new(),
        au: Audio::new(save.sfx, save.music),
        save,
        ui: Ui::new(),
        rng: Rng::new(seed),
        t: 0.0,
        dt: 0.0,
        w: 1.0,
        h: 1.0,
        x0: 0.0,
        pw: 1.0,
        u: 1.0,
        top: 0.0,
        bot: 0.0,
        debug: q.flag("debug"),
        walk_from: None,
        frame: 0,
        q,
    };
    let first = match (app.q.num("level"), app.q.get("screen")) {
        (Some(n), _) => Nav::Play((n as usize).saturating_sub(1)),
        (None, Some("map")) => Nav::Map,
        (None, Some("pack")) => Nav::Pack,
        (None, Some("choose")) => Nav::Choose,
        _ => Nav::Title,
    };
    measure(&mut app);
    let mut screen = open(&mut app, first);
    let mut leaving: Option<(Nav, f32)> = None;
    let mut arriving = 0.3f32;
    let mut frames = 0u32;
    platform::ready();

    loop {
        app.dt = get_frame_time().min(0.05) * speed;
        app.t += app.dt;
        app.g.begin_frame();
        if frames % 30 == 0 || (screen_width() - app.w).abs() > 0.5 || (screen_height() - app.h).abs() > 0.5 {
            measure(&mut app);
        }
        frames += 1;
        app.frame = frames;
        app.ui.u = app.u;
        app.ui.begin();
        if leaving.is_some() {
            app.ui.eat();
        }
        app.au.update();

        let nav = match &mut screen {
            Screen::Title(s) => s.frame(&mut app),
            Screen::Choose(s) => s.frame(&mut app),
            Screen::Story(s) => s.frame(&mut app),
            Screen::Map(s) => s.frame(&mut app),
            Screen::Pack(s) => s.frame(&mut app),
            Screen::Play(s) => s.frame(&mut app),
        };
        if leaving.is_none() {
            if let Some(n) = nav {
                leaving = Some((n, 0.0));
            }
        }

        // quick dip to dark between screens
        const FADE: f32 = 0.18;
        let mut cover = 0.0;
        if let Some((n, t)) = &mut leaving {
            *t += get_frame_time().min(0.05);
            cover = (*t / FADE).min(1.0);
            if *t >= FADE {
                let n = *n;
                screen = open(&mut app, n);
                leaving = None;
                arriving = FADE;
            }
        } else if arriving > 0.0 {
            arriving -= get_frame_time().min(0.05);
            cover = (arriving / FADE).clamp(0.0, 1.0);
        }
        if cover > 0.0 {
            app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(rgb(0x0b0d0a), cover));
        }
        next_frame().await
    }
}

/// Works out the play column and safe areas for the current window.
fn measure(app: &mut App) {
    app.w = screen_width().max(1.0);
    app.h = screen_height().max(1.0);
    app.pw = app.w.min(app.h * 0.62);
    app.x0 = (app.w - app.pw) / 2.0;
    app.u = app.pw / 390.0;
    app.top = platform::inset(0);
    app.bot = platform::inset(2);
}
