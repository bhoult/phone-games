//! Rufus Crush: a match-3 about the very busy life of one black lab.
//!
//! The rules live in the `rufus-core` crate. This crate is everything you can
//! see and hear: it draws with macroquad (WebGL in the browser) and compiles
//! to a single self-contained wasm file.

mod ambient;
mod audio;
mod content;
mod fx;
mod gfx;
mod map;
mod platform;
mod play;
mod rufus;
mod save;
mod story;
mod synth;
mod title;
mod ui;

use audio::Audio;
use gfx::*;
use macroquad::prelude::{get_frame_time, next_frame, screen_height, screen_width, Conf, Rect};
use platform::Query;
use rufus_core::{Rng, LEVELS};
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
    frame: u32,
}

impl App {
    /// In debug mode, tells the test driver where a button is so it can be
    /// tapped by name with a real touch.
    pub fn mark(&self, name: &str, r: Rect) {
        if self.debug && self.frame % 15 == 0 {
            macroquad::logging::info!("RUFUS BTN {} {:.0} {:.0}", name, r.x + r.w / 2.0, r.y + r.h / 2.0);
        }
    }
}

/// A request to change screens.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Nav {
    Title,
    /// The map, scrolled to this level if given.
    Map(Option<usize>),
    Play(usize),
    /// Rufus's scrapbook, and then on to wherever was next.
    Story(story::Then),
}

enum Screen {
    Title(title::Title),
    Map(map::MapScreen),
    Play(Box<play::Play>),
    Story(Box<story::Story>),
}

fn open(app: &mut App, nav: Nav) -> Screen {
    match nav {
        Nav::Title => Screen::Title(title::Title::new(app)),
        Nav::Map(focus) => Screen::Map(map::MapScreen::new(app, focus)),
        Nav::Play(level) => Screen::Play(Box::new(play::Play::new(app, level.min(LEVELS.len() - 1)))),
        Nav::Story(then) => Screen::Story(Box::new(story::Story::new(app, then))),
    }
}

fn conf() -> Conf {
    Conf {
        window_title: "Rufus Crush".to_string(),
        window_width: 430,
        window_height: 900,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let q = Query::read();
    let mut save = Save::load();
    // Test switches change this session only. A session that used one never
    // writes to storage, so faked progress can't overwrite the real thing.
    if q.flag("unlock") {
        save.paws = vec![q.num("unlock").map(|n| n as u8).unwrap_or(2).clamp(1, 3); LEVELS.len()];
        save.ephemeral = true;
    }
    if q.flag("mute") {
        save.sfx = false;
        save.music = false;
        save.ephemeral = true;
    }
    if let Some(d) = q.num("difficulty") {
        save.difficulty = rufus_core::Difficulty::from_index(d as usize);
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
        frame: 0,
        q,
    };
    let first = match (app.q.num("level"), app.q.get("screen")) {
        (Some(n), _) => Nav::Play((n as usize).saturating_sub(1)),
        (None, Some("map")) => Nav::Map(None),
        (None, Some("story")) => Nav::Story(story::Then::Title),
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
            Screen::Map(s) => s.frame(&mut app),
            Screen::Play(s) => {
                let go = s.frame(&mut app);
                // Finishing a level can open a page of Rufus's story. It is
                // shown on the way to wherever the player was going next.
                let new_pages = story::unseen(&app.save) > 0;
                go.map(|go| match (go, new_pages) {
                    (play::Go::Retry, _) => Nav::Play(s.level),
                    (play::Go::Map, false) => Nav::Map(Some(s.level)),
                    (play::Go::Next, false) => Nav::Play(s.level + 1),
                    (play::Go::Map, true) => Nav::Story(story::Then::Map(Some(s.level))),
                    (play::Go::Next, true) => Nav::Story(story::Then::Play(s.level + 1)),
                })
            }
            Screen::Story(s) => s.frame(&mut app),
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
            app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(rgb(0x101630), cover));
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
