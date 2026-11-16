//! Rufus's scrapbook: the true story of how he got here, one page at a time.
//!
//! A page opens after the first level, and then one more at the end of every
//! chapter until the book is full. New pages are shown on the way out of the
//! level that opened them; the whole book so far can be read again from the
//! title screen.
//!
//! Each page is a picture and a few lines. The pictures are not painted one
//! by one: they are staged from what the game already has (the six backdrops
//! in different light, Rufus himself in a mood) plus a handful of props.

use crate::fx::Fx;
use crate::gfx::*;
use crate::rufus::{Mood, Rufus};
use crate::save::Save;
use crate::ui::*;
use crate::{App, Nav};
use macroquad::prelude::{vec2, Rect};
use rufus_core::level::CHAPTER_LEN;
use rufus_core::{Rng, LEVELS};

/// The first page opens after this level, so the book turns up straight away.
pub const FIRST_PAGE_AFTER: usize = 1;
/// After that, a new page every this many levels: one per chapter.
pub const PAGE_EVERY: usize = CHAPTER_LEN;

/// Where to go once the book is shut.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Then {
    Title,
    Map(Option<usize>),
    Play(usize),
}

impl Then {
    fn nav(self) -> Nav {
        match self {
            Then::Title => Nav::Title,
            Then::Map(focus) => Nav::Map(focus),
            Then::Play(level) => Nav::Play(level.min(LEVELS.len() - 1)),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pose {
    /// Sitting, full size.
    Sit,
    /// Sitting, puppy size.
    Pup,
    /// Paddling about in the water.
    Swim,
    /// Not in the picture.
    Away,
}

/// Something standing in the picture. `x` is its middle and `y` where its
/// feet are, as fractions of the picture; `w` is its width the same way.
struct Prop {
    s: Spr,
    x: f32,
    y: f32,
    w: f32,
    /// In front of Rufus rather than behind him.
    front: bool,
}

const fn back(s: Spr, x: f32, y: f32, w: f32) -> Prop {
    Prop { s, x, y, w, front: false }
}

const fn front(s: Spr, x: f32, y: f32, w: f32) -> Prop {
    Prop { s, x, y, w, front: true }
}

struct Page {
    title: &'static str,
    text: &'static str,
    /// Backdrop, by `Gfx` scene number, and the light it is seen in.
    scene: usize,
    light: u32,
    pose: Pose,
    mood: Mood,
    /// Where Rufus is, across the picture.
    at: f32,
    hold: Option<Spr>,
    /// In the scarf.
    scarf: bool,
    barks: bool,
    hearts: bool,
    snow: bool,
    props: &'static [Prop],
}

const YARD: usize = 0;
const PARK: usize = 1;
const MUD: usize = 2;
const HOME: usize = 3;
const NIGHT: usize = 4;
const LAKE: usize = 5;

const PLAIN: Page = Page {
    title: "",
    text: "",
    scene: YARD,
    light: 0xffffff,
    pose: Pose::Sit,
    mood: Mood::Idle,
    at: 0.5,
    hold: None,
    scarf: false,
    barks: false,
    hearts: false,
    snow: false,
    props: &[],
};

/// The story, in order. Every fact in it came from Rufus's people; the
/// opinions are Rufus's own.
#[rustfmt::skip]
static PAGES: [Page; 23] = [
    Page {
        title: "The Storm",
        text: "Rufus's story starts in Little Rock, after the tornadoes. Somebody found a puppy wandering through the wreckage. Small, black, and all on his own.",
        scene: MUD, light: 0x8f96ad, pose: Pose::Pup, mood: Mood::Worried,
        props: &[back(spr::DEBRIS, 0.2, 0.9, 0.44), back(spr::DEBRIS, 0.84, 0.86, 0.36)],
        ..PLAIN
    },
    Page {
        title: "A Safe Place",
        text: "A young woman took the puppy in and looked after him for a while. It could not be for good, though. He needed a home, and a post went up on Facebook.",
        scene: HOME, light: 0xfff0d8, pose: Pose::Pup, mood: Mood::Sad, at: 0.42,
        props: &[front(spr::BOWL, 0.72, 0.97, 0.2)],
        ..PLAIN
    },
    Page {
        title: "The Post",
        text: "Abigail saw a Facebook post about a puppy who needed a home, and that was that. The instructions to Brandon were simple: pick him up when you get off work.",
        scene: HOME, pose: Pose::Away, hearts: true,
        props: &[front(spr::PHONE, 0.5, 0.97, 0.36)],
        ..PLAIN
    },
    Page {
        title: "The Ride Home",
        text: "Brandon picked the puppy up after work, as instructed. One small black dog, brought home.",
        scene: PARK, light: 0xffd9b0, pose: Pose::Pup, at: 0.24,
        props: &[back(spr::CAR, 0.66, 0.93, 0.58)],
        ..PLAIN
    },
    Page {
        title: "Quiet and Thin",
        text: "At first Rufus was very quiet and very thin. No barking, no zoomies, no opinions. Just a small dog working out whether this was somewhere he could stay.",
        scene: HOME, light: 0xdfe4f2, pose: Pose::Pup, mood: Mood::Sad, at: 0.55,
        props: &[front(spr::BOWL, 0.22, 0.97, 0.2)],
        ..PLAIN
    },
    Page {
        title: "Finding His Voice",
        text: "Bit by bit the quiet dog filled out. Then he found his voice, his appetite and his opinions. The Rufus you know had arrived: enthusiastic about everything.",
        scene: YARD, mood: Mood::Excited, hearts: true,
        ..PLAIN
    },
    Page {
        title: "The Look",
        text: "Rufus is enthusiastic. He is also manipulative. Somewhere along the way he learned that looking pitiful gets results, and he has been perfecting it ever since. You have seen the look. It works on you too.",
        scene: HOME, light: 0xfff0d8, mood: Mood::Pitiful,
        ..PLAIN
    },
    Page {
        title: "The Woods",
        text: "These days Rufus goes on walks and hikes in the woods around Clarkville, Arkansas. Every trail has to be smelled from one end to the other. This takes as long as it takes.",
        scene: PARK, mood: Mood::Proud, at: 0.45, hold: Some(spr::STICK),
        ..PLAIN
    },
    Page {
        title: "Water Dog",
        text: "If there is a lake or a creek, Rufus is in it. He does not check how cold it is first. He does not check anything first.",
        scene: LAKE, pose: Pose::Swim,
        ..PLAIN
    },
    Page {
        title: "Pup-Cups",
        text: "Rufus comes along to coffee shops. The coffee is for Brandon and Abigail. The pup-cup is for Rufus. He considers this a fair split.",
        scene: PARK, light: 0xffecc4, mood: Mood::Happy, at: 0.36,
        props: &[front(spr::PUPCUP, 0.72, 0.96, 0.19)],
        ..PLAIN
    },
    Page {
        title: "Home Security",
        text: "Rufus hates mailmen and UPS drivers. They come to his home and he defends it. They always leave in the end, which he takes as proof that it works.",
        scene: YARD, mood: Mood::Alert, at: 0.26, barks: true,
        props: &[back(spr::TRUCK, 0.72, 0.9, 0.52)],
        ..PLAIN
    },
    Page {
        title: "The Day Shift",
        text: "While Brandon works from home, Rufus sulks on the couch and watches the windows. He is looking for Abigail, the postman, possums and cats. Possibly in that order.",
        scene: HOME, light: 0xe9e4f6, mood: Mood::Sad, at: 0.4,
        props: &[back(spr::CAT, 0.84, 0.5, 0.13)],
        ..PLAIN
    },
    Page {
        title: "The Good Things",
        text: "A short list of what Rufus loves: chicken, pup-cups, swimming, and chewing on his toys. Chicken is first on purpose.",
        scene: HOME, light: 0xfff0d8, mood: Mood::Excited, at: 0.38,
        props: &[front(spr::CHICKEN, 0.7, 0.96, 0.21), front(spr::TOY, 0.88, 0.97, 0.15), front(spr::PUPCUP, 0.12, 0.97, 0.13)],
        ..PLAIN
    },
    Page {
        title: "The Scarf",
        text: "Abigail started dressing Rufus in a colourful scarf. He is fine with the scarf. He thinks it makes him look distinguished, and he is right.",
        scene: PARK, mood: Mood::Proud, scarf: true,
        ..PLAIN
    },
    Page {
        title: "The Shoes",
        text: "Now and then Abigail puts shoes on him as well. Rufus hates the shoes. The scarf was a deal. Nobody agreed to shoes.",
        scene: HOME, mood: Mood::Pitiful, at: 0.4, scarf: true,
        props: &[front(spr::SHOES, 0.76, 0.97, 0.27)],
        ..PLAIN
    },
    Page {
        title: "The Kennel",
        text: "When Brandon and Abigail travelled to Italy and Portugal, Rufus had to stay in a kennel. He did not like it. He would like that on the record.",
        scene: NIGHT, light: 0xd5dbf0, mood: Mood::Sad,
        props: &[back(spr::PLANE, 0.8, 0.3, 0.24), front(spr::KENNEL, 0.5, 0.985, 0.6)],
        ..PLAIN
    },
    Page {
        title: "Better Arrangements",
        text: "After the kennel, things changed. Brandon and Abigail found a girl who looks after Rufus in her home, and Brandon's brother Philip has stayed with him at the apartment. Rufus approves of both.",
        scene: HOME, light: 0xfff0d8, mood: Mood::Happy, scarf: true, hearts: true,
        ..PLAIN
    },
    Page {
        title: "Colorado",
        text: "Rufus would much rather come along. One trip was a whole week in Colorado, with Brandon and Abigail right there the entire time. More of that, please.",
        scene: LAKE, light: 0xdcecff, mood: Mood::Excited, at: 0.42, scarf: true,
        ..PLAIN
    },
    Page {
        title: "The Yellow Van",
        text: "Then came the van: a bright yellow Sprinter camper. Officially it is a camper van. In practice it is a kennel on wheels, built around one dog.",
        scene: PARK, mood: Mood::Proud, at: 0.2, scarf: true,
        props: &[back(spr::VAN, 0.64, 0.92, 0.68)],
        ..PLAIN
    },
    Page {
        title: "Test Runs",
        text: "Brandon and Abigail took the yellow van camping close to home a few times, to try it out. Rufus tried out the sleeping.",
        scene: NIGHT, mood: Mood::Asleep, at: 0.36, scarf: true,
        props: &[back(spr::VAN, 0.7, 0.9, 0.56), front(spr::CAMPFIRE, 0.12, 0.97, 0.19)],
        ..PLAIN
    },
    Page {
        title: "Six Thousand Miles",
        text: "The big trip in the yellow van: 6,000 miles in two and a half weeks. Pagosa Springs, Durango, Ouray, Moab, Salt Lake City, Glacier Park, and on up into Canada.",
        scene: YARD, light: 0xffecc4, mood: Mood::Excited, at: 0.22, scarf: true,
        props: &[back(spr::VAN, 0.58, 0.92, 0.6), front(spr::SIGNPOST, 0.88, 0.98, 0.23)],
        ..PLAIN
    },
    Page {
        title: "Banff and Jasper",
        text: "Banff and Jasper, up in the Canadian Rockies, were the far end of the big road trip. Then the yellow van turned around and drove all the way home.",
        scene: LAKE, light: 0xe2fff4, mood: Mood::Proud, scarf: true,
        ..PLAIN
    },
    Page {
        title: "To Be Continued",
        text: "There are many more adventures planned, for Brandon, Abigail and Rufus. This book has plenty of empty pages. Rufus intends to fill them.",
        scene: YARD, light: 0xffecc4, mood: Mood::Happy, at: 0.22, scarf: true, hearts: true,
        props: &[back(spr::VAN, 0.66, 0.92, 0.6)],
        ..PLAIN
    },
];

/// The level (counting from 1) that opens a page when it is finished.
pub fn opens_after(page: usize) -> usize {
    if page == 0 {
        FIRST_PAGE_AFTER
    } else {
        page * PAGE_EVERY
    }
}

/// The page that finishing this level (counting from 0) opens, if it opens one.
pub fn page_at(level: usize) -> Option<usize> {
    (0..PAGES.len()).find(|&p| opens_after(p) == level + 1)
}

/// How many pages this player has opened. It is worked out from the levels
/// they have finished, so progress made before the book existed counts too.
pub fn unlocked(save: &Save) -> usize {
    (0..PAGES.len()).take_while(|&p| save.paws.get(opens_after(p) - 1).is_some_and(|&paws| paws > 0)).count()
}

/// Pages opened but not looked at yet.
pub fn unseen(save: &Save) -> usize {
    unlocked(save).saturating_sub(save.story_seen as usize)
}

pub struct Story {
    /// The pages on show are `first..end`; `page` is the open one.
    first: usize,
    end: usize,
    page: usize,
    then: Then,
    /// Reading the whole book from the title screen, not being shown new pages.
    browsing: bool,
    t: f32,
    page_t: f32,
    rufus: Rufus,
    fx: Fx,
    rng: Rng,
    auto: bool,
    bark_in: f32,
    heart_in: f32,
}

impl Story {
    /// The new pages, if there are any; otherwise the whole book so far.
    pub fn new(app: &mut App, then: Then) -> Story {
        if app.debug {
            macroquad::logging::info!("RUFUS SCREEN story");
        }
        let end = unlocked(&app.save);
        let seen = (app.save.story_seen as usize).min(end);
        let browsing = seen == end;
        let first = if browsing { 0 } else { seen };
        let mut story = Story {
            first,
            end,
            page: first,
            then,
            browsing,
            t: 0.0,
            page_t: 0.0,
            rufus: Rufus::new(app.rng.next_u64()),
            fx: Fx::new(app.rng.next_u64()),
            rng: Rng::new(app.rng.next_u64()),
            auto: app.q.num("auto").unwrap_or(0.0) > 0.0,
            bark_in: 1.2,
            heart_in: 0.5,
        };
        story.open(app);
        story
    }

    /// Turns to `self.page`: sets the scene and remembers it has been seen.
    fn open(&mut self, app: &mut App) {
        self.page_t = 0.0;
        self.bark_in = 1.2;
        self.heart_in = 0.5;
        let Some(page) = PAGES.get(self.page).filter(|_| self.page < self.end) else { return };
        self.rufus = Rufus::new(self.rng.next_u64());
        self.rufus.rest(page.mood);
        self.rufus.wear = page.scarf.then_some(spr::SCARF);
        if let Some(s) = page.hold {
            self.rufus.hold(s, 9999.0);
        }
        if app.debug {
            macroquad::logging::info!("RUFUS STORY page {} of {}", self.page + 1, PAGES.len());
        }
        if self.page as u32 + 1 > app.save.story_seen {
            app.save.story_seen = self.page as u32 + 1;
            app.save.store();
        }
    }

    pub fn frame(&mut self, app: &mut App) -> Option<Nav> {
        let dt = app.dt;
        let u = app.u;
        self.t += dt;
        self.page_t += dt;
        let cx = app.x0 + app.pw / 2.0;
        let g = &app.g;
        let ink = rgb(0x5a3a1a);

        // the page: warm paper, a few faded paw prints
        g.vgrad(Rect::new(0.0, 0.0, app.w, app.h), rgb(0xfdf1d6), rgb(0xf0d7a6));
        for k in 0..11 {
            let fx = (k as f32 * 0.618_034 + 0.21).fract();
            let fy = (k as f32 * 0.29 + 0.07).fract();
            g.spr(spr::P_PAW, app.x0 + fx * app.pw, fy * app.h, 34.0 * u, 34.0 * u, k as f32 * 1.7, alpha(rgb(0xb98a5a), 0.1));
        }
        if app.x0 > 1.0 {
            g.rect(Rect::new(0.0, 0.0, app.x0, app.h), alpha(rgb(0x000000), 0.28));
            g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), alpha(rgb(0x000000), 0.28));
        }

        // heading
        let head_y = app.top + 34.0 * u;
        g.label("RUFUS'S STORY", cx, head_y, 21.0 * u, 0.5, rgb(0xd2401f), None, 0.0);
        let count = if self.end == 0 { String::new() } else { format!("page {} of {}", self.page + 1, PAGES.len()) };
        g.label(&count, cx, head_y + 22.0 * u, 12.5 * u, 0.5, rgb(0xb98a5a), None, 0.0);

        // the photograph: a white border with the picture inside
        let border = 9.0 * u;
        let photo_w = app.pw - 40.0 * u;
        // a little shorter on a short screen, so the words always fit
        let photo_h = (photo_w * 0.94).min(app.h * 0.43);
        let photo = Rect::new(cx - photo_w / 2.0, head_y + 42.0 * u, photo_w, photo_h);
        let pic = Rect::new(photo.x + border, photo.y + border, photo.w - border * 2.0, photo.h - border * 2.0);
        g.rect(Rect::new(photo.x + 3.0 * u, photo.y + 5.0 * u, photo.w, photo.h), alpha(rgb(0x5a3a1a), 0.22));
        g.rect(photo, WHITE);

        let mut title = "Not Yet";
        let mut text = "Rufus's story starts once the first level is done. Go on. He's waiting.";
        if let Some(page) = PAGES.get(self.page).filter(|_| self.page < self.end) {
            title = page.title;
            text = page.text;
            self.picture(app, page, pic);
        } else {
            app.g.rect(pic, rgb(0xe9dcc0));
            app.g.spr_fit(spr::IC_LOCK, pic.x + pic.w / 2.0, pic.y + pic.h / 2.0, 70.0 * u, 0.0, alpha(rgb(0xb98a5a), 0.8));
        }
        let g = &app.g;
        // a new page arrives with a sticker on it
        if !self.browsing && self.end > 0 {
            let pop = (self.page_t * 5.0).min(1.0);
            let s = 15.0 * u * (0.6 + 0.4 * pop) * (1.0 + 0.04 * (self.t * 5.0).sin());
            g.label("NEW PAGE!", photo.x + photo.w - 14.0 * u, photo.y + 6.0 * u, s, 1.0, rgb(0xff5a3c), Some((WHITE, 2.4 * u)), -0.08);
        }

        // the words
        let mut y = photo.y + photo.h + 32.0 * u;
        g.label(title, cx, y, 26.0 * u, 0.5, ink, None, 0.0);
        y += 26.0 * u;
        let room = app.h - app.bot - 86.0 * u - y;
        let wide = app.pw - 52.0 * u;
        // shrink the words rather than let them run under the buttons
        let mut size = 17.0 * u;
        while size > 11.0 * u && g.para_height(text, size, wide) > room {
            size -= 0.5 * u;
        }
        y += g.para(text, cx, y, size, wide, ink);

        // what opens the next page, when this is as far as the book goes
        if self.browsing && self.page + 1 >= self.end && self.end < PAGES.len() && self.end > 0 {
            let note = format!("The next page opens after level {}.", opens_after(self.end));
            g.label(&note, cx, (y + 16.0 * u).min(app.h - app.bot - 92.0 * u), 12.5 * u, 0.5, rgb(0xb98a5a), None, 0.0);
        }

        // buttons: shut the book, back a page, on a page
        let by = app.h - app.bot - 72.0 * u;
        let bh = 52.0 * u;
        let last = self.page + 1 >= self.end;
        let next_label = match (last, self.browsing, self.then) {
            (false, _, _) => "NEXT",
            (true, true, _) => "DONE",
            (true, false, Then::Play(_)) => "PLAY ON",
            (true, false, _) => "DONE",
        };
        let ready = self.page_t > 0.35;
        let next = Rect::new(cx - 38.0 * u, by, 190.0 * u, bh);
        let prev = Rect::new(cx - 152.0 * u, by, 104.0 * u, bh);
        let close = Rect::new(app.x0 + 8.0 * u, app.top + 9.0 * u, 40.0 * u, 40.0 * u);
        app.mark("story_next", next);
        app.mark("story_close", close);
        let mut nav = None;
        let mut turn: Option<usize> = None;
        if app.ui.button(&app.g, &mut app.au, 40, next, GREEN, next_label, Some(spr::IC_NEXT), ready) || (self.auto && self.page_t > 1.2) {
            if last {
                nav = Some(self.then.nav());
            } else {
                turn = Some(self.page + 1);
            }
        }
        if self.page > self.first {
            app.mark("story_back", prev);
            if app.ui.button(&app.g, &mut app.au, 41, prev, BLUE, "BACK", None, ready) {
                turn = Some(self.page - 1);
            }
        }
        if app.ui.button(&app.g, &mut app.au, 42, close, BLUE, "", Some(spr::IC_X), true) {
            nav = Some(self.then.nav());
        }
        if let Some(p) = turn {
            self.page = p;
            self.open(app);
        }
        // each page fades up from the paper
        let fade = 1.0 - (self.page_t / 0.3).min(1.0);
        app.g.rect(pic, alpha(rgb(0xfdf1d6), fade));
        nav
    }

    /// Stages one page's picture inside `pic`.
    fn picture(&mut self, app: &mut App, page: &Page, pic: Rect) {
        let (dt, u, t) = (app.dt, app.u, self.t);
        let g = &app.g;
        let light = rgb(page.light);
        g.scene(page.scene, pic, light);
        crate::ambient::draw(g, page.scene, pic, t, u);
        let prop = |p: &Prop| {
            let w = p.w * pic.w;
            let h = w * p.s.h / p.s.w;
            g.spr(p.s, pic.x + p.x * pic.w, pic.y + p.y * pic.h - h / 2.0, w, h, 0.0, light);
        };
        for p in page.props.iter().filter(|p| !p.front) {
            prop(p);
        }
        let ground = pic.y + pic.h * 0.94;
        let x = pic.x + page.at * pic.w;
        self.fx.update(dt);
        match page.pose {
            Pose::Away => {}
            Pose::Swim => {
                // out in the water, paddling in a slow loop, a wake behind him
                let sx = pic.x + pic.w * (0.5 + 0.2 * (t * 0.5).sin());
                let sy = pic.y + pic.h * 0.6 + 3.0 * u * (t * 3.0).sin();
                let facing_left = (t * 0.5).cos() < 0.0;
                let frame = if (t * 5.0) as u32 % 2 == 0 { spr::RUFUS_SWIM1 } else { spr::RUFUS_SWIM2 };
                let w = pic.w * 0.42;
                g.spr(spr::P_DOT, sx, sy + w * 0.2, w * 1.1, w * 0.16, 0.0, alpha(WHITE, 0.35));
                g.spr_ex(frame, sx, sy, w, w * frame.h / frame.w, 0.0, WHITE, facing_left);
            }
            Pose::Sit | Pose::Pup => {
                let tall = pic.h * if page.pose == Pose::Pup { 0.36 } else { 0.62 };
                self.rufus.update(dt, &mut self.fx);
                self.rufus.draw(g, x, ground, tall);
                if page.barks {
                    self.bark_in -= dt;
                    if self.bark_in <= 0.0 {
                        self.bark_in = 2.2;
                        self.rufus.bark(2);
                    }
                }
            }
        }
        for p in page.props.iter().filter(|p| p.front) {
            prop(p);
        }
        if page.hearts {
            self.heart_in -= dt;
            if self.heart_in <= 0.0 {
                self.heart_in = 1.1;
                let from = if page.pose == Pose::Away { vec2(pic.x + pic.w * 0.5, pic.y + pic.h * 0.4) } else { self.rufus.head };
                self.fx.hearts(from, 2, u);
            }
        }
        if page.snow {
            for i in 0..26 {
                let h = |k: u32| ((i * 7919 + k * 104_729) as f32 * 0.618_034).fract();
                let fall = (h(1) + t * (0.05 + 0.05 * h(2))).fract();
                let sway = 0.02 * (t * (0.8 + h(3)) + h(4) * 6.28).sin();
                let s = (3.0 + 4.0 * h(5)) * u;
                g.spr(spr::P_DOT, pic.x + (h(6) + sway).clamp(0.02, 0.98) * pic.w, pic.y + fall * pic.h, s, s, 0.0, alpha(WHITE, 0.9 * (1.0 - fall).min(1.0).min(fall * 8.0)));
            }
        }
        self.fx.draw(g);
    }
}
