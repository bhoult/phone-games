//! The level screen: board, HUD, Rufus on his stage, and the choreography
//! that plays back what the rules engine reports.
//!
//! The rules (`rufus_core::Board`) resolve a whole step at once and describe
//! it; this file turns each description into timed animation, then asks for
//! the next step. Nothing here decides what happens on the board.

use crate::audio::Sfx;
use crate::content::*;
use crate::fx::Fx;
use crate::gfx::*;
use crate::platform;
use crate::rufus::{Mood, Rufus};
use crate::ui::*;
use crate::App;
use macroquad::prelude::{vec2, Color, Rect, Vec2};
use rufus_core::bot::{self, BotCfg};
use rufus_core::level::*;
use rufus_core::{
    BlastKind, Cause, ClearReport, Difficulty, FallReport, Item, Move, Pos, Rng, Session, Shuffled, Special, Step, COLS, KINDS, N, ROWS,
    RUN_LEAD, RUN_STEP, ZOOMIES_FULL,
};

const SWAP_T: f32 = 0.15;
/// The pitiful look: when the extra moves are handed over, and when it ends.
const BEG_GIVE: f32 = 1.9;
const BEG_END: f32 = 3.5;
const GRAVITY: f32 = 50.0;
const GOLD: Color = rgb(0xffd23a);
const CARD: Color = rgb(0xd9a566);
const MUD: Color = rgb(0x8a5a2b);

/// Where to go when this screen is finished with.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Go {
    Map,
    Next,
    Retry,
}

#[derive(Clone, Copy, PartialEq)]
enum Popup {
    None,
    Intro,
    Pause,
    Won,
    Lost,
}

enum Phase {
    Idle,
    Swap { ida: u32, idb: u32, a: Pos, b: Pos, t: f32, ok: bool },
    Clear,
    Fall,
    Bones { t: f32 },
    Shuffle { t: f32 },
    /// End-of-level bonus: leftover moves become power pieces, one at a time.
    BonusPlant { wait: f32, planted: u32 },
    BonusFire { wait: f32 },
    /// A beat of silence before the "out of moves" card.
    Losing { t: f32 },
    /// Out of moves but nearly there: Rufus gives you the look, in close-up,
    /// until you hand over a few more.
    Begging { t: f32, given: bool },
    Done,
}

struct PopAnim {
    t: f32,
    into: Option<Vec2>,
}

struct VPiece {
    id: u32,
    item: Item,
    /// Position in cell units.
    x: f32,
    y: f32,
    falling: bool,
    ty: f32,
    vy: f32,
    delay: f32,
    pop: Option<PopAnim>,
    /// Seconds since it appeared as a new power piece (pop-in).
    born: f32,
    /// Seconds since it landed (squash and settle).
    land: f32,
    shake: f32,
    flash: f32,
    wiggle: f32,
    /// Shuffle animation: where it started.
    from: Option<Vec2>,
}

impl VPiece {
    fn at(id: u32, item: Item, x: f32, y: f32) -> VPiece {
        VPiece {
            id,
            item,
            x,
            y,
            falling: false,
            ty: y,
            vy: 0.0,
            delay: 0.0,
            pop: None,
            born: 9.0,
            land: 9.0,
            shake: 9.0,
            flash: 0.0,
            wiggle: 9.0,
            from: None,
        }
    }
}

#[derive(Clone, Copy)]
enum Act {
    Pop(usize),
    Zap(usize),
    Made(usize),
    Morph(usize),
    BoxHit(usize),
    Mud(usize),
    Blast(usize),
    Points(Vec2, u32, Color),
}

/// Rufus mid-zoomie, crossing a row or column.
struct Runner {
    horizontal: bool,
    line: usize,
    dir: i8,
    t: f32,
    puff: f32,
}

/// Golden Rufus fetching: a paw print flying to its target.
struct Zap {
    from: Vec2,
    to: Vec2,
    t: f32,
    dur: f32,
}

/// Something flying up to its goal counter in the HUD.
struct Flyer {
    spr: Spr,
    from: Vec2,
    to: Vec2,
    t: f32,
    dur: f32,
    size: f32,
    goal: usize,
}

/// Where everything goes on screen this frame.
#[derive(Clone, Copy, Default)]
struct L {
    u: f32,
    stage: Rect,
    panel: Rect,
    zoom: Rect,
    cell: f32,
    bx: f32,
    by: f32,
    rufus_x: f32,
    rufus_ground: f32,
    rufus_h: f32,
}

impl L {
    fn cell(&self, x: f32, y: f32) -> Vec2 {
        vec2(self.bx + (x + 0.5) * self.cell, self.by + (y + 0.5) * self.cell)
    }
}

pub struct Play {
    pub level: usize,
    s: Session,
    theme: Theme,
    phase: Phase,
    popup: Popup,
    popup_t: f32,
    bonus: bool,
    pieces: Vec<VPiece>,
    mud: [u8; N],
    rep: ClearReport,
    acts: Vec<(f32, Act)>,
    act_i: usize,
    clear_t: f32,
    clear_dur: f32,
    last_pop_t: f32,
    note: u8,
    runners: Vec<Runner>,
    zaps: Vec<Zap>,
    flyers: Vec<Flyer>,
    fx: Fx,
    rufus: Rufus,
    cameo: Option<(u8, f32)>,
    cameo_cd: f32,
    picked: Option<Pos>,
    drag: Option<Pos>,
    idle_t: f32,
    hint: Option<Move>,
    moved_once: bool,
    t: f32,
    l: L,
    rows: (usize, usize),
    // What the HUD shows. These trail the real tallies so the counters tick
    // as pieces visibly pop, not the instant the rules resolve a step.
    vis_kinds: [u32; KINDS],
    vis_mud: u32,
    vis_boxes: u32,
    vis_bones: u32,
    vis_score: f32,
    score_target: u32,
    goal_pulse: [f32; 4],
    goal_pos: [Vec2; 4],
    turn_pops: [u32; KINDS],
    turn_total: u32,
    warned_low: bool,
    said_ready: bool,
    quip_cd: f32,
    auto: u8,
    auto_t: f32,
    debug: bool,
    rng: Rng,
    result_line: &'static str,
    result_paws: u8,
    new_best: bool,
    paws_shown: u8,
    wiggle_in: f32,
    land_cd: f32,
    banner: Option<(String, f32)>,
    woof: Option<(Vec2, f32, bool)>,
    /// At the lake Rufus swims his zoomies, and shakes off when he's done.
    lake: bool,
    shook: bool,
    beg_line: &'static str,
    /// A tap during the treat frenzy fast-forwards the rest of it.
    hurry: bool,
    changed_difficulty: bool,
}

impl Play {
    pub fn new(app: &mut App, level: usize) -> Play {
        let seed = app.q.num("seed").map(|s| s as u64).unwrap_or_else(|| app.rng.next_u64());
        let mut s = Session::with(level, seed, app.save.difficulty);
        if let Some(m) = app.q.num("moves") {
            s.moves_left = m.max(1.0) as u32;
        }
        let mut rng = Rng::new(seed ^ 0x5eed);
        let r0 = (0..ROWS).find(|&y| (0..COLS).any(|x| s.board.get(x, y).exists)).unwrap_or(0);
        let r1 = (0..ROWS).rev().find(|&y| (0..COLS).any(|x| s.board.get(x, y).exists)).unwrap_or(ROWS - 1);

        // Everything drops in from above while the level card is up.
        let mut pieces = Vec::new();
        for (pos, pc) in s.board.pieces() {
            let top = s.board.column_top(pos.x as usize).unwrap_or(0) as f32;
            let mut p = VPiece::at(pc.id, pc.item, pos.x as f32, top - 1.0 - (r1 as f32 - pos.y as f32) * 0.2);
            p.falling = true;
            p.ty = pos.y as f32;
            p.delay = 0.25 + (r1 as f32 - pos.y as f32) * 0.05 + pos.x as f32 * 0.035 + rng.range(0.0, 0.03);
            pieces.push(p);
        }
        let mut mud = [0u8; N];
        for (i, m) in mud.iter_mut().enumerate() {
            *m = s.board.cell(Pos::from_i(i)).mud;
        }
        let cameo = app.q.num("cameo").map(|k| (k as u8 % KINDS as u8, -1.2));
        if app.debug {
            macroquad::logging::info!("RUFUS LEVEL {}", level + 1);
        }
        let mut rufus = Rufus::new(seed ^ 0xD06);
        rufus.rest(Mood::Idle);
        rufus.react(Mood::Happy, 1.5);
        Play {
            level,
            s,
            theme: theme(level / CHAPTER_LEN),
            phase: Phase::Idle,
            popup: Popup::Intro,
            popup_t: 0.0,
            bonus: false,
            pieces,
            mud,
            rep: ClearReport::default(),
            acts: Vec::new(),
            act_i: 0,
            clear_t: 0.0,
            clear_dur: 0.0,
            last_pop_t: -1.0,
            note: 0,
            runners: Vec::new(),
            zaps: Vec::new(),
            flyers: Vec::new(),
            fx: Fx::new(seed ^ 0xF),
            rufus,
            cameo,
            cameo_cd: 3.0,
            picked: None,
            drag: None,
            idle_t: 0.0,
            hint: None,
            moved_once: false,
            t: 0.0,
            l: L::default(),
            rows: (r0, r1),
            vis_kinds: [0; KINDS],
            vis_mud: 0,
            vis_boxes: 0,
            vis_bones: 0,
            vis_score: 0.0,
            score_target: 0,
            goal_pulse: [0.0; 4],
            goal_pos: [Vec2::ZERO; 4],
            turn_pops: [0; KINDS],
            turn_total: 0,
            warned_low: false,
            said_ready: false,
            quip_cd: 6.0,
            auto: app.q.num("auto").unwrap_or(0.0) as u8,
            auto_t: 0.0,
            debug: app.debug,
            rng,
            result_line: "",
            result_paws: 0,
            new_best: false,
            paws_shown: 0,
            wiggle_in: 2.0,
            land_cd: 0.0,
            banner: None,
            woof: None,
            lake: is_lake(level),
            shook: false,
            beg_line: "",
            hurry: false,
            changed_difficulty: false,
        }
    }

    fn layout(&self, app: &App) -> L {
        let u = app.u;
        let bar_h = 46.0 * u;
        let zoom = Rect::new(app.x0 + 14.0 * u, app.h - app.bot - bar_h - 10.0 * u, app.pw - 28.0 * u, bar_h);
        let used = (self.rows.1 - self.rows.0 + 1) as f32;
        let bottom = zoom.y - 8.0 * u;
        let mut stage_h = app.top + (app.h * 0.24).clamp(150.0 * u, 215.0 * u);
        let fit = |stage_h: f32| ((app.pw - 12.0 * u) / COLS as f32).min((bottom - stage_h - 8.0 * u) / used).min(64.0 * u).max(8.0);
        // A short board leaves room to spare: give half of it to the scene.
        let spare = (bottom - stage_h - 8.0 * u) - used * fit(stage_h);
        stage_h += (spare * 0.5).clamp(0.0, 80.0 * u);
        let stage = Rect::new(app.x0, 0.0, app.pw, stage_h);
        let top = stage_h + 8.0 * u;
        let cell = fit(stage_h);
        L {
            u,
            stage,
            panel: Rect::new(app.x0 + app.pw - 198.0 * u, app.top + 8.0 * u, 190.0 * u, 94.0 * u),
            zoom,
            cell,
            bx: app.x0 + (app.pw - cell * COLS as f32) / 2.0,
            by: top + ((bottom - top) - used * cell) / 2.0 - self.rows.0 as f32 * cell,
            rufus_x: app.x0 + 84.0 * u,
            rufus_ground: stage_h - 16.0 * u,
            rufus_h: ((stage_h - app.top) * 0.62).min(150.0 * u),
        }
    }

    fn cell_at(&self, p: Vec2) -> Option<Pos> {
        let (x, y) = ((p.x - self.l.bx) / self.l.cell, (p.y - self.l.by) / self.l.cell);
        if x < 0.0 || y < 0.0 || x >= COLS as f32 || y >= ROWS as f32 {
            return None;
        }
        let pos = Pos::new(x as usize, y as usize);
        self.s.board.cell(pos).exists.then_some(pos)
    }

    /// With `?debug=1`, narrates state changes to the browser console so a
    /// test driver can follow along.
    fn log(&self, what: &str) {
        if self.debug {
            macroquad::logging::info!("RUFUS {} moves={} score={} charge={}", what, self.s.moves_left, self.s.board.score, self.s.board.charge);
        }
    }

    fn piece(&mut self, id: u32) -> Option<&mut VPiece> {
        self.pieces.iter_mut().find(|p| p.id == id)
    }

    fn goal_index(&self, icon: GoalIcon) -> Option<usize> {
        self.s.goals().iter().position(|g| g.icon == icon)
    }

    /// Goal progress as the HUD should currently show it.
    fn shown_goals(&self) -> Vec<GoalItem> {
        self.s
            .goals()
            .into_iter()
            .map(|mut g| {
                g.have = match g.icon {
                    GoalIcon::Score => self.vis_score.round() as u32,
                    GoalIcon::Kind(k) => self.vis_kinds[k as usize],
                    GoalIcon::Mud => self.vis_mud,
                    GoalIcon::Bone => self.vis_bones,
                    GoalIcon::Box => self.vis_boxes,
                }
                .min(g.have);
                g
            })
            .collect()
    }

    fn sync_vis(&mut self) {
        let t = &self.s.board.tally;
        self.vis_kinds = t.kinds;
        self.vis_mud = t.mud;
        self.vis_boxes = t.boxes;
        self.vis_bones = t.bones;
        self.score_target = self.s.board.score;
    }

    // ---- turn flow -------------------------------------------------------------

    fn try_swap(&mut self, app: &mut App, a: Pos, b: Pos) {
        let (Some(pa), Some(pb)) = (self.s.board.cell(a).piece, self.s.board.cell(b).piece) else { return };
        if self.s.moves_left == 0 {
            // only reachable in the "one last zoomies" moment
            self.picked = None;
            self.drag = None;
            self.rufus.say("No moves left. Tap for zoomies!", 2.0);
            app.au.play(Sfx::Bonk, 0.6);
            return;
        }
        let ok = self.s.swap(a, b);
        self.phase = Phase::Swap { ida: pa.id, idb: pb.id, a, b, t: 0.0, ok };
        self.picked = None;
        self.drag = None;
        self.hint = None;
        self.idle_t = 0.0;
        self.log(if ok { "SWAP ok" } else { "SWAP no" });
        if ok {
            self.moved_once = true;
            self.turn_pops = [0; KINDS];
            self.turn_total = 0;
            app.au.play(Sfx::Swap, 0.7);
            if self.rufus.mood() == Mood::Asleep {
                self.rufus.rest(Mood::Idle);
            }
        } else {
            app.au.play(Sfx::Bonk, 0.7);
            platform::vibrate(25);
            self.rufus.react(Mood::Confused, 0.9);
            if self.rng.chance(0.4) {
                let line = self.rng.pick(&BAD_SWAP);
                self.rufus.say(line, 1.2);
            }
        }
    }

    fn zoomies(&mut self, app: &mut App) {
        if !self.s.board.zoomies() {
            return;
        }
        self.log("ZOOMIES");
        self.turn_pops = [0; KINDS];
        self.turn_total = 0;
        self.picked = None;
        self.hint = None;
        self.idle_t = 0.0;
        self.said_ready = false;
        self.rufus.rest(Mood::Idle);
        self.rufus.react(Mood::Excited, 2.4);
        self.rufus.bark(2);
        self.rufus.quiet();
        app.au.play(Sfx::Bark2, 0.9);
        platform::vibrate(40);
        let c = vec2(self.l.bx + self.l.cell * COLS as f32 / 2.0, self.l.by + self.l.cell * (self.rows.0 + self.rows.1 + 1) as f32 / 2.0);
        let f = self.fx.text("ZOOMIES!!!", c, 46.0 * self.l.u, GOLD, rgb(0x7a2a00), 1.5);
        f.rot = -0.08;
        self.step(app);
    }

    /// Asks the board for its next piece of business and starts animating it.
    fn step(&mut self, app: &mut App) {
        match self.s.board.advance() {
            Step::Clear(rep) => self.begin_clear(app, rep),
            Step::Fall(rep) => self.begin_fall(&rep),
            Step::Bones(list) => self.begin_bones(app, &list),
            Step::Shuffle(list) => self.begin_shuffle(app, &list),
            Step::Idle => self.settled(app),
        }
    }

    fn settled(&mut self, app: &mut App) {
        self.sync_vis();
        if self.bonus {
            if self.s.board.fire_powers() {
                self.step(app);
            } else {
                self.finish_win(app);
            }
            return;
        }
        if self.s.won() {
            self.start_bonus(app);
            return;
        }
        if self.s.lost() {
            if self.s.board.zoomies_ready() {
                // Out of moves, but zoomies are free: one last chance.
                self.phase = Phase::Idle;
                self.idle_t = 0.0;
                self.auto_t = 0.0;
                self.hint = None;
                self.banner = Some(("LAST CHANCE: ZOOMIES!".into(), 0.0));
                self.rufus.rest(Mood::Idle);
                self.rufus.react(Mood::Excited, 2.0);
                self.rufus.say("I can fix this!", 2.4);
                app.au.play(Sfx::Yip, 0.8);
                self.log("LASTCHANCE");
                return;
            }
            if self.s.can_beg() {
                // So nearly there. Time for the look.
                self.phase = Phase::Begging { t: 0.0, given: false };
                self.beg_line = self.rng.pick(&BEG_LINES);
                self.hint = None;
                self.picked = None;
                self.banner = None;
                self.rufus.rest(Mood::Pitiful);
                self.rufus.react(Mood::Pitiful, BEG_GIVE);
                let line = self.rng.pick(&BEG_SAYS);
                self.rufus.say(line, BEG_GIVE - 0.2);
                app.au.play(Sfx::Whine, 0.6);
                self.log("BEG");
                return;
            }
            self.phase = Phase::Losing { t: 0.0 };
            // If the look has been tried and failed he is just sad. Otherwise
            // he is still giving it everything.
            self.rufus.rest(if self.s.begged() { Mood::Sad } else { Mood::Pitiful });
            self.rufus.quiet();
            self.log("LOST");
            return;
        }
        self.phase = Phase::Idle;
        self.idle_t = 0.0;
        self.auto_t = 0.0;
        self.hint = None;
        self.log("IDLE");

        // A big turn brings someone by to visit.
        if self.turn_total >= 9 && self.cameo.is_none() && self.cameo_cd <= 0.0 {
            let kind = (0..KINDS).max_by_key(|&k| self.turn_pops[k]).unwrap() as u8;
            if self.turn_pops[kind as usize] >= 5 {
                self.start_cameo(app, kind);
            }
        }
        self.turn_total = 0;

        if self.s.moves_left <= 3 && !self.warned_low {
            self.warned_low = true;
            self.rufus.rest(Mood::Worried);
            if !self.rufus.speaking() {
                let line = self.rng.pick(&LOW_MOVES);
                self.rufus.say(line, 1.8);
            }
        }
        if self.s.board.zoomies_ready() {
            self.log("ZOOMREADY");
        }
        if self.s.board.zoomies_ready() && !self.said_ready {
            self.said_ready = true;
            app.au.play(Sfx::Yip, 0.8);
            self.rufus.react(Mood::Excited, 1.4);
            let line = self.rng.pick(&ZOOM_READY);
            self.rufus.say(line, 2.2);
        }
    }

    fn begin_fall(&mut self, rep: &FallReport) {
        // bottom pieces of each column go first
        let mut order: Vec<usize> = (0..rep.falls.len()).collect();
        order.sort_by_key(|&i| (rep.falls[i].to.x, std::cmp::Reverse(rep.falls[i].to.y)));
        let mut col = usize::MAX;
        let mut rank = 0;
        for i in order {
            let f = rep.falls[i];
            if f.to.x as usize != col {
                col = f.to.x as usize;
                rank = 0;
            }
            let delay = rank as f32 * 0.03;
            rank += 1;
            if f.spawned {
                let mut p = VPiece::at(f.id, f.item, f.to.x as f32, f.from_y as f32);
                p.falling = true;
                p.ty = f.to.y as f32;
                p.delay = delay;
                self.pieces.push(p);
            } else if let Some(p) = self.piece(f.id) {
                p.falling = true;
                p.ty = f.to.y as f32;
                p.delay = delay;
                p.vy = 2.0;
            }
        }
        self.phase = Phase::Fall;
    }

    fn begin_bones(&mut self, app: &mut App, list: &[(u32, Pos)]) {
        let goal = self.goal_index(GoalIcon::Bone);
        for &(id, pos) in list {
            self.pieces.retain(|p| p.id != id);
            let from = self.l.cell(pos.x as f32, pos.y as f32);
            self.fx.sparkle(from, self.l.cell * 0.4, 8, 16.0 * self.l.u, GOLD);
            self.fx.text("+200", from, 20.0 * self.l.u, GOLD, rgb(0x7a4a00), 0.9);
            if let Some(gi) = goal {
                self.flyers.push(Flyer { spr: spr::BONE, from, to: self.goal_pos[gi], t: 0.0, dur: 0.55, size: self.l.cell * 0.8, goal: gi });
            }
        }
        self.vis_bones += list.len() as u32;
        app.au.play(Sfx::Bone, 0.9);
        platform::vibrate(20);
        self.rufus.react(Mood::Excited, 1.3);
        self.rufus.say("BONE!", 1.3);
        self.phase = Phase::Bones { t: 0.0 };
    }

    fn begin_shuffle(&mut self, app: &mut App, list: &[Shuffled]) {
        for s in list {
            if let Some(p) = self.piece(s.id) {
                p.from = Some(vec2(s.from.x as f32, s.from.y as f32));
                p.x = s.to.x as f32;
                p.y = s.to.y as f32;
                p.ty = p.y;
                p.item = s.item;
            }
        }
        let c = vec2(self.l.bx + self.l.cell * COLS as f32 / 2.0, self.l.by + self.l.cell * (self.rows.0 + self.rows.1 + 1) as f32 / 2.0);
        self.fx.text("No moves! SHAKE IT OFF!", c, 24.0 * self.l.u, WHITE, NAVY, 1.6);
        app.au.play(Sfx::Zoom, 0.6);
        self.rufus.react(Mood::Happy, 1.5);
        let line = self.rng.pick(&SHUFFLE_QUIPS);
        self.rufus.say(line, 1.8);
        self.phase = Phase::Shuffle { t: 0.0 };
    }

    fn begin_clear(&mut self, app: &mut App, rep: ClearReport) {
        let u = self.l.u;
        let ms = |t: u32| t as f32 / 1000.0;
        let mut acts: Vec<(f32, Act)> = Vec::new();
        for (i, p) in rep.pops.iter().enumerate() {
            acts.push((ms(p.t), Act::Pop(i)));
            if p.cause == Cause::Fetch {
                if let Some(b) = p.blast {
                    acts.push(((ms(p.t) - 0.24).max(ms(rep.blasts[b as usize].t)), Act::Zap(i)));
                }
            }
            if let Item::Normal { kind, .. } = p.item {
                self.turn_pops[kind as usize] += 1;
            }
        }
        self.turn_total += rep.pops.len() as u32;
        for (i, m) in rep.made.iter().enumerate() {
            acts.push((ms(m.t) + 0.1, Act::Made(i)));
        }
        for (i, m) in rep.morphs.iter().enumerate() {
            acts.push((ms(m.t), Act::Morph(i)));
        }
        for (i, b) in rep.box_hits.iter().enumerate() {
            acts.push((ms(b.t), Act::BoxHit(i)));
        }
        for (i, m) in rep.mud_hits.iter().enumerate() {
            acts.push((ms(m.t), Act::Mud(i)));
        }
        for (i, b) in rep.blasts.iter().enumerate() {
            acts.push((ms(b.t), Act::Blast(i)));
            // one total per blast, shown as it finishes
            let hits: Vec<_> = rep.pops.iter().filter(|p| p.blast == Some(i as u16)).collect();
            if !hits.is_empty() {
                let total: u32 = hits.iter().map(|p| p.points).sum();
                let n = hits.len() as f32;
                let c = vec2(hits.iter().map(|p| p.pos.x as f32).sum::<f32>() / n, hits.iter().map(|p| p.pos.y as f32).sum::<f32>() / n);
                let when = hits.iter().map(|p| p.t).max().unwrap();
                acts.push((ms(when) * 0.6, Act::Points(c, total, GOLD)));
            }
        }
        for g in &rep.groups {
            acts.push((0.04, Act::Points(vec2(g.cx, g.cy), g.points, kind_color(g.kind))));
        }
        acts.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Chain reactions get shouted about, louder each time.
        if let Some(text) = cascade_callout(rep.cascade) {
            let c = vec2(self.l.bx + self.l.cell * COLS as f32 / 2.0, self.l.by + self.l.cell * (self.rows.0 as f32 + 1.2));
            let size = (20.0 + rep.cascade as f32 * 3.5).min(40.0) * u;
            let f = self.fx.text(text, c, size, WHITE, rgb(0xd2401f), 1.3);
            f.rot = if rep.cascade % 2 == 0 { -0.06 } else { 0.06 };
            match rep.cascade {
                2 => app.au.play(Sfx::Yip, 0.7),
                3 => {
                    app.au.play(Sfx::Bark, 0.8);
                    self.rufus.bark(1);
                }
                _ => {
                    app.au.play(Sfx::Bark2, 0.9);
                    self.rufus.bark(2);
                    self.fx.shake(3.0 * u);
                }
            }
            self.rufus.react(if rep.cascade >= 3 { Mood::Excited } else { Mood::Happy }, 1.6);
        } else if self.rufus.mood() == Mood::Idle {
            self.rufus.react(Mood::Happy, 0.9);
        }

        self.note = rep.cascade.saturating_sub(1).min(4) as u8;
        self.last_pop_t = -1.0;
        self.clear_dur = ms(rep.duration);
        self.clear_t = 0.0;
        self.acts = acts;
        self.act_i = 0;
        self.rep = rep;
        self.phase = Phase::Clear;
        platform::vibrate(8);
    }

    fn run_act(&mut self, app: &mut App, act: Act) {
        let l = self.l;
        let u = l.u;
        match act {
            Act::Pop(i) => {
                let pop = self.rep.pops[i];
                let c = l.cell(pop.pos.x as f32, pop.pos.y as f32);
                let into = pop.into.map(|q| vec2(q.x as f32, q.y as f32));
                if let Some(p) = self.piece(pop.id) {
                    p.pop = Some(PopAnim { t: 0.0, into });
                }
                let color = match pop.item {
                    Item::Normal { kind, .. } => kind_color(kind),
                    _ => GOLD,
                };
                if into.is_some() {
                    self.fx.burst(c, color, 3, 90.0 * u, 5.0 * u, 0.0);
                } else {
                    self.fx.burst(c, color, 7, 150.0 * u, 7.0 * u, 0.05);
                }
                if self.clear_t - self.last_pop_t > 0.03 {
                    if self.last_pop_t >= 0.0 {
                        self.note = (self.note + 1).min(7);
                    }
                    self.last_pop_t = self.clear_t;
                    app.au.play(Sfx::Pop(self.note), 0.7);
                }
                self.score_target += pop.points;
                if let Item::Normal { kind, .. } = pop.item {
                    if kind == TOY && pop.cause == Cause::Match {
                        app.au.play(Sfx::Squeak, 0.45);
                    }
                    self.vis_kinds[kind as usize] += 1;
                    if let Some(gi) = self.goal_index(GoalIcon::Kind(kind)) {
                        if self.flyers.len() < 40 {
                            let dur = 0.45 + self.rng.range(0.0, 0.15);
                            self.flyers.push(Flyer { spr: kind_spr(kind), from: c, to: self.goal_pos[gi], t: 0.0, dur, size: l.cell * 0.6, goal: gi });
                        }
                    }
                }
            }
            Act::Zap(i) => {
                let pop = self.rep.pops[i];
                let Some(b) = pop.blast else { return };
                let at = self.rep.blasts[b as usize].at;
                let dur = (pop.t as f32 / 1000.0 - self.clear_t).max(0.05);
                self.zaps.push(Zap { from: vec2(at.x as f32, at.y as f32), to: vec2(pop.pos.x as f32, pop.pos.y as f32), t: 0.0, dur });
            }
            Act::Made(i) => {
                let m = self.rep.made[i];
                let mut p = VPiece::at(m.id, m.item, m.pos.x as f32, m.pos.y as f32);
                p.born = 0.0;
                self.pieces.push(p);
                let c = l.cell(m.pos.x as f32, m.pos.y as f32);
                self.fx.sparkle(c, l.cell * 0.5, 9, 18.0 * u, WHITE);
                self.fx.ring(c, l.cell * 0.3, l.cell * 1.1, 0.4, WHITE, 0.0);
                self.fx.text(made_callout(m.item), c - vec2(0.0, l.cell * 0.55), 17.0 * u, GOLD, rgb(0x7a2a00), 1.0);
                app.au.play(Sfx::Chime, 0.8);
                platform::vibrate(18);
            }
            Act::Morph(i) => {
                let m = self.rep.morphs[i];
                self.morph(app, m.id, m.item, m.pos);
            }
            Act::BoxHit(i) => {
                let b = self.rep.box_hits[i];
                let c = l.cell(b.pos.x as f32, b.pos.y as f32);
                if b.hp_left == 0 {
                    self.pieces.retain(|p| p.id != b.id);
                    self.fx.chunks(spr::P_BIT, c, CARD, 10, 190.0 * u, 13.0 * u, 520.0 * u);
                    self.fx.puff(c, l.cell * 0.6, Vec2::ZERO, alpha(rgb(0xf3e2b8), 0.8));
                    app.au.play(Sfx::Rip, 0.9);
                    self.vis_boxes += 1;
                    if let Some(gi) = self.goal_index(GoalIcon::Box) {
                        self.goal_pulse[gi] = 1.0;
                    }
                } else {
                    if let Some(p) = self.piece(b.id) {
                        p.item = Item::Box { hp: b.hp_left };
                        p.shake = 0.0;
                    }
                    self.fx.chunks(spr::P_BIT, c, rgb(0xff5a3c), 5, 150.0 * u, 10.0 * u, 520.0 * u);
                    app.au.play(Sfx::Rip, 0.55);
                }
            }
            Act::Mud(i) => {
                let m = self.rep.mud_hits[i];
                self.mud[m.pos.i()] = m.left;
                let c = l.cell(m.pos.x as f32, m.pos.y as f32);
                self.fx.chunks(spr::P_DROP, c, MUD, 6, 150.0 * u, 9.0 * u, 600.0 * u);
                app.au.play(Sfx::Splat, 0.6);
                self.vis_mud += 1;
                if let Some(gi) = self.goal_index(GoalIcon::Mud) {
                    self.goal_pulse[gi] = 1.0;
                }
            }
            Act::Blast(i) => {
                let b = self.rep.blasts[i];
                let c = l.cell(b.at.x as f32, b.at.y as f32);
                match b.kind {
                    BlastKind::Row { dir } => {
                        self.log("RUN");
                        self.runners.push(Runner { horizontal: true, line: b.at.y as usize, dir, t: 0.0, puff: 0.0 });
                        app.au.play(if self.lake { Sfx::Splash } else { Sfx::Zoom }, 0.8);
                        self.rufus.react(Mood::Excited, 1.2);
                    }
                    BlastKind::Col { dir } => {
                        self.log("RUN");
                        self.runners.push(Runner { horizontal: false, line: b.at.x as usize, dir, t: 0.0, puff: 0.0 });
                        app.au.play(if self.lake { Sfx::Splash } else { Sfx::Zoom }, 0.8);
                        self.rufus.react(Mood::Excited, 1.2);
                    }
                    BlastKind::Bomb { big } => {
                        let reach = if big { 3.6 } else { 2.5 };
                        self.fx.ring(c, l.cell * 0.4, l.cell * reach, 0.45, WHITE, 0.08);
                        self.fx.ring(c, l.cell * 0.2, l.cell * reach * 0.8, 0.5, GOLD, 0.14);
                        self.fx.shake(if big { 13.0 } else { 8.0 } * u);
                        self.woof = Some((c, 0.0, big));
                        app.au.play(if big { Sfx::Bark2 } else { Sfx::Bark }, 1.0);
                        app.au.play(Sfx::Boom, if big { 1.0 } else { 0.7 });
                        platform::vibrate(if big { 80 } else { 45 });
                        self.rufus.bark(if big { 2 } else { 1 });
                        self.rufus.react(Mood::Excited, 1.2);
                    }
                    BlastKind::Fetch { kind } => {
                        let centre = vec2(l.bx + l.cell * COLS as f32 / 2.0, c.y.clamp(l.by + l.cell * (self.rows.0 as f32 + 1.0), l.by + l.cell * (self.rows.1 as f32)));
                        let f = self.fx.text(fetch_callout(kind), centre, 30.0 * u, kind_color(kind), NAVY, 1.7);
                        f.rot = -0.05;
                        self.fx.ring(c, l.cell * 0.3, l.cell * 1.6, 0.5, GOLD, 0.0);
                        app.au.play(Sfx::Sparkle, 0.9);
                        app.au.play(Sfx::Bark, 0.8);
                        self.rufus.bark(1);
                        self.rufus.react(if kind == SQUIRREL || kind == CAT { Mood::Alert } else { Mood::Excited }, 2.0);
                        if self.cameo.is_none() {
                            self.start_cameo(app, kind);
                        }
                    }
                    BlastKind::Everything => {
                        self.fx.flash = 0.9;
                        self.fx.shake(16.0 * u);
                        for k in 0..4 {
                            self.fx.ring(c, l.cell * 0.4, l.cell * (3.0 + k as f32 * 1.6), 0.7, if k % 2 == 0 { WHITE } else { GOLD }, k as f32 * 0.09);
                        }
                        let centre = vec2(l.bx + l.cell * COLS as f32 / 2.0, l.by + l.cell * (self.rows.0 + self.rows.1 + 1) as f32 / 2.0);
                        self.fx.text("BEST. BOY. EVER.", centre, 40.0 * u, GOLD, rgb(0x7a2a00), 2.2);
                        app.au.play(Sfx::Howl, 1.0);
                        app.au.play(Sfx::Boom, 1.0);
                        platform::vibrate(120);
                        self.rufus.bark(3);
                        self.rufus.react(Mood::Excited, 3.0);
                    }
                }
            }
            Act::Points(at, pts, color) => {
                if pts > 0 {
                    let size = (15.0 + (pts as f32 / 45.0).min(13.0)) * u;
                    self.fx.text(&format!("+{pts}"), l.cell(at.x, at.y), size, color, NAVY, 0.85);
                }
            }
        }
    }

    fn morph(&mut self, app: &mut App, id: u32, item: Item, pos: Pos) {
        let c = self.l.cell(pos.x as f32, pos.y as f32);
        if let Some(p) = self.piece(id) {
            p.item = item;
            p.flash = 0.35;
            p.born = 0.0;
        }
        self.fx.sparkle(c, self.l.cell * 0.45, 6, 15.0 * self.l.u, WHITE);
        app.au.play(Sfx::Tick, 0.8);
    }

    fn start_bonus(&mut self, app: &mut App) {
        self.bonus = true;
        self.picked = None;
        self.hint = None;
        self.banner = Some(("GOOD BOY!".into(), 0.0));
        app.au.play(Sfx::Bone, 0.9);
        app.au.play(Sfx::Bark2, 0.8);
        self.rufus.rest(Mood::Happy);
        self.rufus.react(Mood::Excited, 2.5);
        self.rufus.bark(2);
        self.rufus.say("I DID IT!", 1.6);
        platform::vibrate(60);
        self.fx.confetti(self.l.stage.x, self.l.stage.x + self.l.stage.w, self.l.stage.y + self.l.stage.h, 40, self.l.u);
        self.phase = Phase::BonusPlant { wait: 0.9, planted: 0 };
    }

    fn finish_win(&mut self, app: &mut App) {
        self.sync_vis();
        self.result_paws = self.s.paws();
        self.new_best = app.save.record(self.level, self.result_paws);
        self.result_line = if self.level + 1 == LEVELS.len() {
            FINALE
        } else if (self.level + 1) % CHAPTER_LEN == 0 {
            chapter_done(self.level / CHAPTER_LEN)
        } else {
            WIN_LINES[self.rng.below(WIN_LINES.len() as u32) as usize]
        };
        self.popup = Popup::Won;
        self.popup_t = 0.0;
        self.paws_shown = 0;
        self.log("WON");
        self.phase = Phase::Done;
        self.banner = None;
        app.au.play(Sfx::Win, 0.9);
        self.rufus.quiet();
        self.rufus.rest(Mood::Excited);
        // he's holding whatever the level was about
        let prize = match LEVELS[self.level].goals[0] {
            Goal::Kind(k, _) => kind_spr(k),
            Goal::Bones(_) => spr::BONE,
            _ => spr::BALL,
        };
        self.rufus.hold(prize, 999.0);
        self.shook = false;
    }

    fn start_cameo(&mut self, app: &mut App, kind: u8) {
        self.cameo = Some((kind, 0.0));
        self.cameo_cd = 7.0;
        let line = quip(kind, &mut self.rng);
        self.rufus.say(line, 2.4);
        match kind {
            SQUIRREL => {
                self.rufus.react(Mood::Alert, 2.2);
                self.rufus.bark(2);
                app.au.play(Sfx::Bark2, 0.8);
            }
            CAT => self.rufus.react(Mood::Worried, 2.6),
            SOCK => {
                self.rufus.hold(spr::SOCK, 2.6);
                self.rufus.react(Mood::Proud, 2.6);
            }
            POODLE => {
                self.rufus.react(Mood::Excited, 2.4);
                app.au.play(Sfx::Yip, 0.7);
                self.fx.hearts(self.rufus.head, 5, self.l.u);
            }
            BALL => self.rufus.react(Mood::Excited, 2.2),
            STICK => {
                // the stick itself is drawn by the cameo: it is far too big to hold
                self.rufus.react(Mood::Proud, 2.8);
                app.au.play(Sfx::Yip, 0.7);
            }
            _ => {
                self.rufus.hold(spr::TOY, 2.4);
                self.rufus.react(Mood::Happy, 2.4);
                app.au.play(Sfx::Squeak, 0.7);
            }
        }
    }

    // ---- per-frame update --------------------------------------------------------

    fn update(&mut self, app: &mut App) {
        let dt = app.dt;
        let u = self.l.u;
        self.t += dt;
        self.cameo_cd -= dt;
        self.quip_cd -= dt;
        self.land_cd -= dt;
        for p in &mut self.goal_pulse {
            *p = (*p - dt * 3.5).max(0.0);
        }
        self.vis_score += (self.score_target as f32 - self.vis_score) * (1.0 - (-dt * 9.0).exp());
        if (self.score_target as f32 - self.vis_score).abs() < 1.0 {
            self.vis_score = self.score_target as f32;
        }

        // pieces
        let mut landed = false;
        for p in &mut self.pieces {
            p.born += dt;
            p.land += dt;
            p.shake += dt;
            p.wiggle += dt;
            p.flash = (p.flash - dt).max(0.0);
            if p.falling {
                if p.delay > 0.0 {
                    p.delay -= dt;
                } else {
                    p.vy = (p.vy + GRAVITY * dt).min(27.0);
                    p.y += p.vy * dt;
                    if p.y >= p.ty {
                        p.y = p.ty;
                        p.falling = false;
                        p.vy = 0.0;
                        p.land = 0.0;
                        landed = true;
                    }
                }
            }
            if let Some(pop) = &mut p.pop {
                pop.t += dt;
            }
        }
        self.pieces.retain(|p| p.pop.as_ref().is_none_or(|pop| pop.t < 0.24));
        if landed && self.land_cd <= 0.0 {
            self.land_cd = 0.07;
            app.au.play(Sfx::Land, 0.35);
        }
        // now and then somebody on the board fidgets
        self.wiggle_in -= dt;
        if self.wiggle_in <= 0.0 && !self.pieces.is_empty() {
            self.wiggle_in = self.rng.range(1.2, 3.0);
            let i = self.rng.below(self.pieces.len() as u32) as usize;
            if matches!(self.pieces[i].item, Item::Normal { .. }) {
                self.pieces[i].wiggle = 0.0;
            }
        }

        // runners kick up dust as they go (or, at the lake, water)
        for r in &mut self.runners {
            r.t += dt;
            r.puff -= dt;
        }
        let l = self.l;
        for i in 0..self.runners.len() {
            if self.runners[i].puff <= 0.0 {
                self.runners[i].puff = 0.028;
                let (p, back) = runner_pos(&l, &self.runners[i]);
                if self.lake {
                    self.fx.spray(p + back * l.cell * 0.3, 1, 130.0 * u, l.cell * 0.2);
                } else {
                    self.fx.puff(p - back * l.cell * 0.7, l.cell * 0.42, -back * 20.0 * u, alpha(rgb(0xfff4dc), 0.85));
                }
            }
        }
        self.runners.retain(|r| (r.t * 1000.0 - RUN_LEAD as f32) / (RUN_STEP as f32) < (if r.horizontal { COLS } else { ROWS }) as f32 + 2.5);
        for z in &mut self.zaps {
            z.t += dt;
        }
        for i in 0..self.zaps.len() {
            let z = &self.zaps[i];
            if z.t < z.dur {
                let p = zap_pos(&l, z);
                self.fx.sparkle(p, l.cell * 0.1, 1, 11.0 * u, GOLD);
            }
        }
        self.zaps.retain(|z| z.t < z.dur);
        for f in &mut self.flyers {
            f.t += dt;
        }
        for i in 0..self.flyers.len() {
            if self.flyers[i].t >= self.flyers[i].dur {
                self.goal_pulse[self.flyers[i].goal.min(3)] = 1.0;
            }
        }
        self.flyers.retain(|f| f.t < f.dur);
        if let Some((_, t)) = &mut self.cameo {
            *t += dt;
            if *t > 3.0 {
                self.cameo = None;
            }
        }
        if let Some((_, t)) = &mut self.banner {
            *t += dt;
            if *t > 2.2 {
                self.banner = None;
            }
        }
        if let Some((_, t, _)) = &mut self.woof {
            *t += dt;
            if *t > 0.7 {
                self.woof = None;
            }
        }
        self.fx.update(dt);
        self.rufus.update(dt, &mut self.fx);

        // phases
        let mut phase = std::mem::replace(&mut self.phase, Phase::Done);
        let mut turn_into: Option<Phase> = None;
        let mut next: Option<fn(&mut Play, &mut App)> = None;
        match &mut phase {
            Phase::Idle | Phase::Done => {}
            Phase::Swap { ida, idb, a, b, t, ok } => {
                *t += dt;
                let ok = *ok;
                let done = *t >= if ok { SWAP_T } else { SWAP_T * 2.0 };
                // there, and (for a swap that does nothing) back again
                let k = if done {
                    if ok { 1.0 } else { 0.0 }
                } else if *t < SWAP_T {
                    ease_io(*t / SWAP_T)
                } else {
                    ease_io(1.0 - (*t - SWAP_T) / SWAP_T)
                };
                let (va, vb) = (vec2(a.x as f32, a.y as f32), vec2(b.x as f32, b.y as f32));
                let (ia, ib) = (*ida, *idb);
                if let Some(p) = self.piece(ia) {
                    let v = va.lerp(vb, k);
                    (p.x, p.y, p.ty) = (v.x, v.y, v.y);
                }
                if let Some(p) = self.piece(ib) {
                    let v = vb.lerp(va, k);
                    (p.x, p.y, p.ty) = (v.x, v.y, v.y);
                }
                if done {
                    if ok {
                        next = Some(|p, app| p.step(app));
                    } else {
                        turn_into = Some(Phase::Idle);
                    }
                }
            }
            Phase::Clear => {
                self.clear_t += dt;
                while self.act_i < self.acts.len() && self.acts[self.act_i].0 <= self.clear_t {
                    let act = self.acts[self.act_i].1;
                    self.act_i += 1;
                    self.run_act(app, act);
                }
                if self.clear_t >= self.clear_dur {
                    next = Some(|p, app| {
                        p.sync_vis();
                        p.step(app)
                    });
                }
            }
            Phase::Fall => {
                if !self.pieces.iter().any(|p| p.falling) {
                    next = Some(|p, app| p.step(app));
                }
            }
            Phase::Bones { t } => {
                *t += dt;
                if *t > 0.5 {
                    next = Some(|p, app| p.step(app));
                }
            }
            Phase::Shuffle { t } => {
                *t += dt;
                if *t > 0.8 {
                    for p in &mut self.pieces {
                        p.from = None;
                    }
                    next = Some(|p, app| p.step(app));
                }
            }
            Phase::BonusPlant { wait, planted } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    match self.s.plant_bonus() {
                        Some(m) => {
                            if *planted == 0 {
                                self.banner = Some(("TREAT FRENZY!".into(), 0.0));
                            }
                            *planted += 1;
                            *wait = 0.11;
                            self.morph(app, m.id, m.item, m.pos);
                            self.score_target = self.s.board.score;
                        }
                        None => {
                            // anything still unspent is paid out as points
                            let pts = self.s.cash_in_moves();
                            if pts > 0 {
                                let c = vec2(self.l.panel.x + self.l.panel.w * 0.5, self.l.panel.y + self.l.panel.h + 22.0 * u);
                                self.fx.text(&format!("+{pts} spare moves!"), c, 17.0 * u, GOLD, rgb(0x7a2a00), 1.6);
                                app.au.play(Sfx::Chime, 0.8);
                            }
                            self.score_target = self.s.board.score;
                            turn_into = Some(Phase::BonusFire { wait: 0.45 });
                        }
                    }
                }
            }
            Phase::BonusFire { wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    next = Some(|p, app| {
                        if p.s.board.fire_powers() {
                            p.step(app)
                        } else {
                            p.finish_win(app)
                        }
                    });
                }
            }
            Phase::Begging { t, given } => {
                *t += dt;
                if !*given && *t > BEG_GIVE {
                    *given = true;
                    next = Some(|p, app| {
                        p.s.beg();
                        p.rufus.rest(Mood::Idle);
                        p.rufus.react(Mood::Excited, 1.3);
                        let line = p.rng.pick(&BEG_THANKS);
                        p.rufus.say(line, 1.4);
                        p.fx.hearts(p.rufus.head, 7, p.l.u);
                        app.au.play(Sfx::Chime, 0.8);
                        app.au.play(Sfx::Yip, 0.7);
                        platform::vibrate(25);
                        p.log("BEGGED");
                    });
                } else if *t > BEG_END {
                    turn_into = Some(Phase::Idle);
                    next = Some(|p, _| {
                        p.idle_t = 0.0;
                        p.auto_t = 0.0;
                        p.hint = None;
                        p.log("IDLE");
                    });
                }
            }
            Phase::Losing { t } => {
                *t += dt;
                if *t > 0.8 {
                    next = Some(|p, app| {
                        p.result_line = LOSE_LINES[p.rng.below(LOSE_LINES.len() as u32) as usize];
                        p.popup = Popup::Lost;
                        p.popup_t = 0.0;
                        p.phase = Phase::Done;
                        p.log("LOSTCARD");
                        app.au.play(Sfx::Lose, 0.8);
                        app.au.play(Sfx::Whine, 0.5);
                    });
                }
            }
        }
        self.phase = turn_into.unwrap_or(phase);
        if let Some(f) = next {
            f(self, app);
        }
    }

    fn idle_input(&mut self, app: &mut App) {
        let u = self.l.u;
        self.idle_t += app.dt;
        let ptr_pos = app.ui.ptr.pos;

        // self-playing mode, for testing builds
        if self.auto > 0 {
            self.auto_t += app.dt;
            if self.auto_t > 0.25 {
                self.auto_t = 0.0;
                if self.s.board.zoomies_ready() {
                    self.zoomies(app);
                } else if let Some(m) = bot::choose(&self.s, &mut self.rng, &BotCfg::CASUAL) {
                    self.try_swap(app, m.a, m.b);
                }
            }
            return;
        }

        if app.ui.ptr.pressed {
            self.idle_t = 0.0;
            if self.rufus.mood() == Mood::Asleep {
                self.rufus.rest(Mood::Idle);
                self.rufus.react(Mood::Confused, 0.8);
            }
            self.drag = self.cell_at(ptr_pos).filter(|&p| self.s.board.cell(p).piece.is_some_and(|pc| !matches!(pc.item, Item::Box { .. })));
            if self.drag.is_none() && self.cell_at(ptr_pos).is_none() {
                self.picked = None;
            }
        }
        if let Some(a) = self.drag {
            let d = ptr_pos - app.ui.ptr.press_pos;
            if app.ui.ptr.down && d.length() > self.l.cell * 0.36 {
                let (dx, dy) = if d.x.abs() > d.y.abs() { (d.x.signum() as i32, 0) } else { (0, d.y.signum() as i32) };
                self.drag = None;
                if let Some(b) = a.offset(dx, dy).filter(|b| self.s.board.cell(*b).exists) {
                    self.try_swap(app, a, b);
                    return;
                }
            } else if app.ui.ptr.released {
                // a tap: first tap picks, second tap on a neighbour swaps
                self.drag = None;
                match self.picked {
                    Some(p) if p.adjacent(a) => {
                        self.try_swap(app, p, a);
                        return;
                    }
                    Some(p) if p == a => self.picked = None,
                    _ => {
                        self.picked = Some(a);
                        app.au.play(Sfx::Click, 0.5);
                    }
                }
            }
        }

        // zoomies: tap the meter, or the dog
        let on_rufus = app.ui.ptr.released && app.ui.ptr.travel < 14.0 * u && self.rufus.hit(ptr_pos) && self.rufus.hit(app.ui.ptr.press_pos);
        if app.ui.tapped(self.l.zoom) || on_rufus {
            if self.s.board.zoomies_ready() {
                self.zoomies(app);
                return;
            } else if on_rufus {
                app.au.play(Sfx::Bark, 0.7);
                self.rufus.bark(1);
                self.rufus.react(Mood::Happy, 1.0);
                let line = self.rng.pick(&BOOP_QUIPS);
                self.rufus.say(line, 1.4);
            } else {
                self.rufus.say("Not yet! Keep matching.", 1.6);
            }
        }

        // hints, quips, naps
        let first_time = self.level == 0 && !self.moved_once;
        let hint_after = if self.debug { 0.3 } else if first_time { 0.9 } else { 5.0 };
        if self.hint.is_none() && self.idle_t > hint_after && self.s.moves_left > 0 {
            self.hint = bot::choose(&self.s, &mut self.rng, &BotCfg::CAREFUL);
            if let (true, Some(m)) = (self.debug, self.hint) {
                let (a, b) = (self.l.cell(m.a.x as f32, m.a.y as f32), self.l.cell(m.b.x as f32, m.b.y as f32));
                macroquad::logging::info!("RUFUS HINT {:.0} {:.0} {:.0} {:.0}", a.x, a.y, b.x, b.y);
            }
        }
        if self.idle_t > 10.0 && self.quip_cd <= 0.0 && !self.rufus.speaking() && self.rufus.mood() != Mood::Asleep {
            self.quip_cd = 16.0;
            let line = self.rng.pick(&IDLE_QUIPS);
            self.rufus.say(line, 2.4);
        }
        if self.idle_t > 26.0 && self.rufus.mood() != Mood::Asleep {
            self.rufus.rest(Mood::Asleep);
            self.rufus.quiet();
        }
    }

    // ---- drawing -------------------------------------------------------------------

    /// Runs one frame: update, draw, and handle whatever was tapped.
    pub fn frame(&mut self, app: &mut App) -> Option<Go> {
        self.l = self.layout(app);
        // cards animate even while the game underneath is frozen
        self.popup_t += app.dt;
        let paused = self.popup == Popup::Pause;
        if !paused {
            if self.popup == Popup::None && matches!(self.phase, Phase::Idle) {
                self.idle_input(app);
            }
            self.update(app);
            // The treat frenzy is a reward, not a wait: it runs at double
            // speed, and a tap fast-forwards whatever is left of it.
            if self.bonus && self.popup == Popup::None {
                if app.ui.ptr.pressed {
                    self.hurry = true;
                }
                for _ in 0..if self.hurry { 5 } else { 1 } {
                    if self.bonus && self.popup == Popup::None {
                        self.update(app);
                    }
                }
            }
        }
        let l = self.l;
        let u = l.u;
        let g = &app.g;
        let shake = self.fx.offset();

        // backdrop
        g.vgrad(Rect::new(0.0, 0.0, app.w, app.h), self.theme.top, self.theme.bottom);
        for i in 0..16 {
            let fx = (i as f32 * 0.618_034).fract();
            let fy = (i as f32 * 0.372 + 0.11).fract();
            let y = l.stage.h + fy * (app.h - l.stage.h);
            g.spr(spr::P_PAW, app.x0 + fx * app.pw, y, 30.0 * u, 30.0 * u, i as f32 * 1.3, alpha(WHITE, 0.055));
        }
        crate::ambient::pools(g, Rect::new(app.x0, l.stage.h, app.pw, app.h - l.stage.h), self.t, u, mix(self.theme.top, WHITE, 0.45));
        g.vgrad(Rect::new(app.x0, app.h - 170.0 * u, app.pw, 170.0 * u), alpha(rgb(0x000000), 0.0), alpha(rgb(0x000000), 0.24));
        g.scene(self.theme.scene, l.stage, self.theme.light);
        crate::ambient::draw(g, self.theme.scene, l.stage, self.t, u);
        // soft edge where the scene meets the play area
        g.vgrad(Rect::new(l.stage.x, l.stage.y + l.stage.h - 1.0, l.stage.w, 14.0 * u), alpha(rgb(0x000000), 0.22), alpha(rgb(0x000000), 0.0));
        if app.x0 > 1.0 {
            g.rect(Rect::new(0.0, 0.0, app.x0, app.h), alpha(rgb(0x000000), 0.28));
            g.rect(Rect::new(app.x0 + app.pw, 0.0, app.w - app.x0 - app.pw, app.h), alpha(rgb(0x000000), 0.28));
        }

        let begging = match self.phase {
            Phase::Begging { t, given } => Some((t, given)),
            _ => None,
        };
        let in_card = begging.is_some() || matches!(self.popup, Popup::Won | Popup::Lost);
        self.draw_cameo(g, false);
        if !in_card {
            self.rufus.draw(g, l.rufus_x, l.rufus_ground, l.rufus_h);
        }
        self.draw_cameo(g, true);

        self.draw_board(g, shake);

        // zoomie runners, fetch paws, particles
        for r in &self.runners {
            let (p, back) = runner_pos(&l, r);
            let frame = match (self.lake, ((r.t * if self.lake { 9.0 } else { 15.0 }) as i32) % 2 == 0) {
                (false, true) => spr::RUFUS_RUN1,
                (false, false) => spr::RUFUS_RUN2,
                (true, true) => spr::RUFUS_SWIM1,
                (true, false) => spr::RUFUS_SWIM2,
            };
            let w = l.cell * 2.1;
            let h = w * frame.h / frame.w;
            let p = p + shake - back * l.cell * 0.15;
            if !self.lake {
                g.lights(|| {
                    let ang = back.y.atan2(back.x) + std::f32::consts::PI;
                    g.spr(spr::STREAK, p.x - back.x * l.cell * 1.7, p.y - back.y * l.cell * 1.7, l.cell * 3.0, l.cell * 0.62, ang, alpha(rgb(0xffe9a8), 0.6));
                    g.spr(spr::GLOW, p.x, p.y, l.cell * 2.4, l.cell * 1.7, 0.0, alpha(rgb(0xffd27a), 0.35));
                });
            }
            if r.horizontal {
                g.spr_ex(frame, p.x, p.y - l.cell * 0.08, w, h, 0.0, WHITE, r.dir < 0);
            } else {
                g.spr(frame, p.x, p.y, w, h, std::f32::consts::FRAC_PI_2 * r.dir as f32, WHITE);
            }
        }
        for z in &self.zaps {
            let p = zap_pos(&l, z) + shake;
            g.spr(spr::GLOW, p.x, p.y, l.cell * 0.9, l.cell * 0.9, 0.0, alpha(GOLD, 0.8));
            g.spr(spr::P_PAW, p.x, p.y, l.cell * 0.42, l.cell * 0.42, z.t * 9.0, WHITE);
        }
        self.fx.draw(g);
        if let Some((c, t, big)) = self.woof {
            let k = ease_back((t / 0.16).min(1.0));
            let a = 1.0 - ((t - 0.4) / 0.3).clamp(0.0, 1.0);
            let size = l.cell * if big { 4.6 } else { 3.3 } * k;
            let c = c + shake;
            g.spr(spr::FX_BURST, c.x, c.y, size, size, t * 1.5, alpha(WHITE, a));
            let text = if big { "WOOF WOOF!" } else { "WOOF!" };
            g.label(text, c.x, c.y, size * if big { 0.2 } else { 0.27 }, 0.5, alpha(WHITE, a), Some((rgb(0xc2260a), size * 0.022)), -0.12);
        }
        for f in &self.flyers {
            let k = ease_io(f.t / f.dur);
            let mid = (f.from + f.to) * 0.5 + vec2((f.to.y - f.from.y) * 0.18, -40.0 * u);
            let a = f.from.lerp(mid, k);
            let b = mid.lerp(f.to, k);
            let p = a.lerp(b, k);
            let size = f.size * (1.0 - 0.45 * k);
            g.spr_fit(f.spr, p.x, p.y, size, k * 2.0, alpha(WHITE, 0.95));
        }

        self.draw_hud(app);
        let g = &app.g;
        self.draw_zoom_bar(g, app.t);
        self.fx.draw_text(g);
        if let Some((text, t)) = &self.banner {
            let k = ease_back((t / 0.3).min(1.0));
            let a = 1.0 - ((t - 1.7) / 0.5).clamp(0.0, 1.0);
            let y = l.by + l.cell * (self.rows.0 as f32 + 0.6);
            let w = (app.pw * 0.86) * k;
            g.spr(spr::RIBBON, app.x0 + app.pw / 2.0, y + 6.0 * u, w, w * 0.28, 0.0, alpha(WHITE, a));
            g.label(text, app.x0 + app.pw / 2.0, y - 4.0 * u, 26.0 * u * k, 0.5, alpha(WHITE, a), Some((rgb(0x7d0f1b), 2.0 * u)), 0.0);
        }
        if !in_card {
            self.rufus.draw_bubble(g, u, app.x0, l.panel.x - 2.0 * u);
        }
        if self.fx.flash > 0.0 {
            g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(WHITE, self.fx.flash.min(1.0) * 0.8));
        }
        if let Some((t, given)) = begging {
            self.draw_beg(app, t, given);
        }
        if self.bonus && self.popup == Popup::None && !self.hurry {
            let g = &app.g;
            g.label("tap to skip", app.x0 + app.pw / 2.0, l.zoom.y - 12.0 * u, 12.0 * u, 0.5, alpha(WHITE, 0.8), Some((alpha(NAVY, 0.6), 1.2 * u)), 0.0);
        }

        // pause button
        let mut go = None;
        let pause_r = Rect::new(app.x0 + 8.0 * u, app.top + 8.0 * u, 38.0 * u, 38.0 * u);
if self.popup == Popup::None {
            app.mark("pause", pause_r);
            app.mark("zoom", l.zoom);
        }
                if app.ui.button(&app.g, &mut app.au, 1, pause_r, BLUE, "", Some(spr::IC_PAUSE), self.popup == Popup::None) && !matches!(self.phase, Phase::Done) {
            self.popup = Popup::Pause;
            self.popup_t = 0.0;
        }
        app.g.label(&format!("LEVEL {}", self.level + 1), pause_r.x + pause_r.w + 8.0 * u, pause_r.y + pause_r.h * 0.5, 17.0 * u, 0.0, WHITE, Some((alpha(NAVY, 0.8), 1.6 * u)), 0.0);

        match self.popup {
            Popup::None => {}
            Popup::Intro => {
                if self.draw_intro(app) {
                    self.popup = Popup::None;
                    app.ui.eat();
                }
            }
            Popup::Pause => go = self.draw_pause(app),
            Popup::Won | Popup::Lost => go = self.draw_result(app),
        }
        go
    }

    fn draw_cameo(&self, g: &Gfx, front: bool) {
        let Some((kind, t)) = self.cameo.filter(|c| c.1 >= 0.0) else { return };
        let l = &self.l;
        let u = l.u;
        let st = l.stage;
        let fence = st.y + st.h * 0.56;
        let ground = l.rufus_ground;
        let right = l.panel.x - 6.0 * u;
        let s = 44.0 * u;
        match (kind, front) {
            // a squirrel legs it along the fence
            (SQUIRREL, false) => {
                let k = t / 2.2;
                let x = lerp(st.x + st.w + s, st.x - s, k);
                let y = fence - s * 0.3 - (t * 13.0).sin().abs() * 10.0 * u;
                g.spr_ex(spr::SQUIRREL, x, y, s, s, (t * 13.0).sin() * 0.12, WHITE, true);
            }
            // the cat arrives, judges, leaves
            (CAT, false) => {
                let k = if t < 0.6 { ease_out(t / 0.6) } else if t > 2.3 { 1.0 - ease_in((t - 2.3) / 0.6) } else { 1.0 };
                let x = lerp(st.x + st.w * 0.36, st.x + st.w * 0.5, 0.5).min(right - s * 0.6);
                g.spr(spr::CAT, x, fence + s * 0.9 - s * 1.15 * k, s * 1.1, s * 1.1, 0.0, WHITE);
            }
            // a ball bounces past
            (BALL, true) => {
                let k = t / 2.0;
                let x = lerp(st.x + st.w + s, st.x - s, k);
                let bounce = (t * 6.5).sin().abs() * 46.0 * u * (1.0 - k * 0.6);
                g.spr(spr::BALL, x, ground - s * 0.35 - bounce, s * 0.8, s * 0.8, -t * 9.0, WHITE);
            }
            // Rufus has found a stick. It is much too big. He is making it work.
            (STICK, true) => {
                let k = if t < 0.4 { ease_back(t / 0.4) } else if t > 2.5 { 1.0 - ease_in((t - 2.5) / 0.5) } else { 1.0 };
                let m = self.rufus.mouth;
                // across the mouth and below the eyes: he needs to see where he's going
                let size = l.rufus_h * 0.98 * k;
                g.spr_fit(spr::STICK, m.x + 4.0 * u, m.y + l.rufus_h * 0.09, size, 0.62 + (t * 6.0).sin() * 0.07, WHITE);
            }
            // a poodle pops up to say hello
            (POODLE, true) => {
                let k = if t < 0.5 { ease_back(t / 0.5) } else if t > 2.2 { 1.0 - ease_in((t - 2.2) / 0.6) } else { 1.0 };
                let x = (l.rufus_x + 96.0 * u).min(right - s * 0.6);
                g.spr(spr::POODLE, x, st.y + st.h + s * 0.6 - s * 1.05 * k, s * 1.2, s * 1.2, (t * 5.0).sin() * 0.08, WHITE);
            }
            _ => {}
        }
    }

    fn draw_board(&self, g: &Gfx, shake: Vec2) {
        let l = &self.l;
        let u = l.u;
        let b = &self.s.board;
        let at = |x: f32, y: f32| l.cell(x, y) + shake;
        // a glass tray the whole board sits in
        let (mut x0, mut x1, mut y0, mut y1) = (COLS, 0, ROWS, 0);
        for y in 0..ROWS {
            for x in 0..COLS {
                if b.get(x, y).exists {
                    (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
                }
            }
        }
        if x0 <= x1 {
            let (a, z) = (at(x0 as f32, y0 as f32), at(x1 as f32, y1 as f32));
            let pad = l.cell * 0.5 + 7.0 * u;
            let tray = Rect::new(a.x - pad, a.y - pad, z.x - a.x + pad * 2.0, z.y - a.y + pad * 2.0);
            let rad = 20.0 * u;
            g.nine(spr::PILL, Rect::new(tray.x, tray.y + 5.0 * u, tray.w, tray.h), 30.0, rad, alpha(rgb(0x000000), 0.22));
            g.nine(spr::PILL, tray, 30.0, rad, alpha(mix(self.theme.bottom, rgb(0x000000), 0.45), 0.5));
            g.nine(spr::PILL, Rect::new(tray.x + 2.0 * u, tray.y + 2.0 * u, tray.w - 4.0 * u, tray.h * 0.5), 30.0, rad, alpha(WHITE, 0.07));
        }
        let tile_light = mix(rgb(0xfffaec), self.theme.top, 0.07);
        let tile_dark = mix(rgb(0xf6e7c4), self.theme.top, 0.2);
        // soft outline under the whole board shape, then the tiles
        for pass in 0..2 {
            for y in 0..ROWS {
                for x in 0..COLS {
                    if !b.get(x, y).exists {
                        continue;
                    }
                    let c = at(x as f32, y as f32);
                    if pass == 0 {
                        g.spr(spr::TILE, c.x, c.y + 2.5 * u, l.cell * 1.12, l.cell * 1.12, 0.0, alpha(rgb(0x1a1206), 0.3));
                    } else {
                        let light = (x + y) % 2 == 0;
                        g.spr(spr::TILE, c.x, c.y, l.cell * 1.0, l.cell * 1.0, 0.0, if light { tile_light } else { tile_dark });
                    }
                }
            }
        }
        for y in 0..ROWS {
            for x in 0..COLS {
                let m = self.mud[y * COLS + x];
                if m > 0 {
                    let c = at(x as f32, y as f32);
                    g.spr(if m >= 2 { spr::MUD2 } else { spr::MUD1 }, c.x, c.y, l.cell * 0.98, l.cell * 0.98, 0.0, WHITE);
                }
            }
        }
        // where bones need to get to
        if self.goal_index(GoalIcon::Bone).is_some() {
            for x in 0..COLS {
                if let Some(y) = b.column_bottom(x) {
                    let c = at(x as f32, y as f32 + 0.62 + 0.06 * (self.t * 4.0 + x as f32).sin());
                    g.spr(spr::FX_ARROW, c.x, c.y, l.cell * 0.34, l.cell * 0.34, std::f32::consts::FRAC_PI_2, alpha(rgb(0xfff4d6), 0.9));
                }
            }
        }
        if let Some(p) = self.picked {
            let c = at(p.x as f32, p.y as f32);
            let s = l.cell * (1.06 + 0.05 * (self.t * 8.0).sin());
            g.spr(spr::SEL, c.x, c.y, s, s, 0.0, GOLD);
        }

        // pieces: resting ones first, anything in motion on top
        let hint = if matches!(self.phase, Phase::Idle) && self.popup == Popup::None { self.hint } else { None };
        let mut cmds: Vec<ItemDraw> = Vec::with_capacity(self.pieces.len());
        for lifted in [false, true] {
            for p in &self.pieces {
                let moving = p.falling || p.pop.is_some() || p.born < 0.4 || p.from.is_some();
                if moving != lifted {
                    continue;
                }
                let (mut x, mut y) = (p.x, p.y);
                let mut scale = 1.0;
                let mut a = 1.0;
                let mut rot = 0.0;
                // new pieces slide in from behind the top edge of their column
                if p.falling {
                    let top = b.column_top(p.x as usize).unwrap_or(0) as f32;
                    a = ((p.y - (top - 1.0)) / 0.55).clamp(0.0, 1.0);
                    if a <= 0.0 {
                        continue;
                    }
                }
                if let (Some(from), Phase::Shuffle { t }) = (p.from, &self.phase) {
                    let k = ease_io((*t / 0.6).min(1.0));
                    let v = from.lerp(vec2(p.x, p.y), k);
                    // swirl through the middle on the way
                    let mid = vec2(COLS as f32 / 2.0 - 0.5, (self.rows.0 + self.rows.1) as f32 / 2.0);
                    let pull = (k * std::f32::consts::PI).sin() * 0.45;
                    (x, y) = (lerp(v.x, mid.x, pull), lerp(v.y, mid.y, pull));
                    rot = (k * std::f32::consts::PI).sin() * 1.2 * if p.id % 2 == 0 { 1.0 } else { -1.0 };
                }
                if let Some(pop) = &p.pop {
                    let k = (pop.t / 0.22).min(1.0);
                    match pop.into {
                        Some(to) => {
                            let e = ease_in(k);
                            x = lerp(x, to.x, e);
                            y = lerp(y, to.y, e);
                            scale = 1.0 - 0.55 * e;
                            a = 1.0 - e * 0.5;
                        }
                        None => {
                            scale = if k < 0.35 { 1.0 + 0.3 * (k / 0.35) } else { 1.3 * (1.0 - (k - 0.35) / 0.65) };
                            rot = k * 0.5;
                            a = 1.0 - ease_in(k);
                        }
                    }
                }
                if p.born < 0.4 {
                    scale *= ease_back(p.born / 0.4);
                }
                if p.shake < 0.35 {
                    x += wobble(p.shake, 55.0, 9.0) * 0.07;
                }
                if p.wiggle < 0.6 {
                    rot += wobble(p.wiggle, 22.0, 6.0) * 0.16;
                }
                if let Some(m) = hint {
                    let pos = Pos::new(p.x.round() as usize, p.y.round() as usize);
                    let toward = if pos == m.a { Some(m.b) } else if pos == m.b { Some(m.a) } else { None };
                    if let Some(o) = toward {
                        let k = 0.5 + 0.5 * (self.t * 6.5).sin();
                        x += (o.x as f32 - p.x) * 0.13 * k;
                        y += (o.y as f32 - p.y) * 0.13 * k;
                        scale *= 1.0 + 0.06 * k;
                    }
                }
                // squash on landing, anchored at the feet
                let sq = if p.land < 0.5 { wobble(p.land, 26.0, 9.0) * 0.17 } else { 0.0 };
                let c = at(x, y);
                let size = l.cell * 0.95 * scale;
                cmds.push(ItemDraw {
                    item: p.item,
                    c: vec2(c.x, c.y + size * 0.5 * sq),
                    size,
                    t: self.t + p.id as f32 * 0.37,
                    sx: 1.0 + sq * 0.8,
                    sy: 1.0 - sq,
                    rot,
                    a,
                    flash: (p.flash / 0.35).max(0.0),
                });
            }
        }
        // light under everything, then the pieces (which catch the gleam as
        // it crosses the board), then markers and flashes on top
        g.lights(|| cmds.iter().for_each(|d| item_light(g, d)));
        g.shine_on(self.t);
        cmds.iter().for_each(|d| item_body(g, d));
        g.shine_off();
        cmds.iter().for_each(|d| item_top(g, d));
        g.lights(|| {
            for d in cmds.iter().filter(|d| d.flash > 0.0) {
                g.spr(spr::GLOW, d.c.x, d.c.y, d.size * 1.7, d.size * 1.7, 0.0, alpha(WHITE, d.flash));
            }
        });

        // first level: a paw shows the move
        if let Some(m) = hint {
            if self.level == 0 && !self.moved_once {
                let k = (self.t * 0.9).fract();
                let slide = ease_io(((k - 0.25) / 0.45).clamp(0.0, 1.0));
                let a = if k > 0.85 { 1.0 - (k - 0.85) / 0.15 } else { (k / 0.12).min(1.0) };
                let from = at(m.a.x as f32, m.a.y as f32);
                let to = at(m.b.x as f32, m.b.y as f32);
                let p = from.lerp(to, slide) + vec2(l.cell * 0.12, l.cell * 0.6);
                g.spr(spr::POINTER, p.x, p.y, l.cell * 1.1, l.cell * 1.1, -0.25, alpha(WHITE, a));
            }
        }
    }

    fn draw_hud(&mut self, app: &App) {
        let l = self.l;
        let u = l.u;
        let g = &app.g;
        let p = l.panel;
        g.nine(spr::PANEL, p, 30.0, 17.0 * u, WHITE);

        // moves
        let mx = p.x + p.w - 35.0 * u;
        let low = self.s.moves_left <= 3 && !self.bonus;
        g.text_c("MOVES", mx, p.y + 15.0 * u, 10.5 * u, rgb(0x9a6a3a));
        let pulse = if low { 1.0 + 0.1 * (self.t * 9.0).sin() } else { 1.0 };
        g.label(&self.s.moves_left.to_string(), mx, p.y + 39.0 * u, 33.0 * u * pulse, 0.5, if low { rgb(0xe8323c) } else { INK }, None, 0.0);
        g.rect(Rect::new(p.x + p.w - 68.0 * u, p.y + 10.0 * u, 1.5 * u, 48.0 * u), alpha(rgb(0x9a6a3a), 0.3));

        // goals
        let goals = self.shown_goals();
        let n = goals.len().max(1) as f32;
        let area_w = p.w - 78.0 * u;
        let cw = (area_w / n).min(46.0 * u);
        let x0 = p.x + 8.0 * u + (area_w - cw * n) / 2.0;
        for (i, goal) in goals.iter().enumerate() {
            let cx = x0 + cw * (i as f32 + 0.5);
            let cy = p.y + 24.0 * u;
            self.goal_pos[i.min(3)] = vec2(cx, cy);
            let bump = 1.0 + 0.35 * self.goal_pulse[i.min(3)];
            let size = (cw - 3.0 * u).min(31.0 * u) * bump;
            match goal.icon {
                GoalIcon::Score => g.spr_fit(spr::P_STAR, cx, cy, size * 0.9, 0.0, GOLD),
                GoalIcon::Kind(k) => g.spr_fit(kind_spr(k), cx, cy, size, 0.0, WHITE),
                GoalIcon::Mud => g.spr_fit(spr::MUD2, cx, cy, size * 0.86, 0.0, WHITE),
                GoalIcon::Bone => {
                    g.spr_fit(spr::P_DOT, cx, cy, size * 1.05, 0.0, rgb(0x7fb6e8));
                    g.spr_fit(spr::BONE, cx, cy, size, 0.0, WHITE);
                }
                GoalIcon::Box => g.spr_fit(spr::BOX, cx, cy, size, 0.0, WHITE),
            }
            if goal.done() {
                g.spr_fit(spr::PAW_GOLD, cx, p.y + 49.0 * u, 17.0 * u * bump, 0.0, WHITE);
            } else {
                g.text_c(&goal.left().to_string(), cx, p.y + 49.0 * u, if goal.left() > 999 { 11.0 } else { 14.0 } * u * bump.min(1.2), INK);
            }
        }

        // score bar with the paw thresholds marked on it
        let [two, three] = LEVELS[self.level].paws;
        let bar = Rect::new(p.x + 12.0 * u, p.y + 67.0 * u, p.w - 24.0 * u, 15.0 * u);
        g.nine(spr::PILL, bar, 30.0, bar.h / 2.0, alpha(rgb(0x5a3413), 0.28));
        let frac = (self.vis_score / three as f32).min(1.0);
        if frac > 0.0 {
            g.nine(spr::PILL, Rect::new(bar.x, bar.y, (bar.w * frac).max(bar.h), bar.h), 30.0, bar.h / 2.0, ORANGE);
        }
        for (i, at) in [0.07, two as f32 / three as f32, 1.0].into_iter().enumerate() {
            let lit = match i {
                0 => self.vis_score > 0.0,
                1 => self.vis_score >= two as f32,
                _ => self.vis_score >= three as f32,
            };
            let x = bar.x + bar.w * at - if i == 2 { 6.0 * u } else { 0.0 };
            g.spr_fit(if lit { spr::PAW_GOLD } else { spr::PAW_GRAY }, x, bar.y + bar.h * 0.45, 19.0 * u, 0.0, WHITE);
        }
        g.label(&format!("{}", self.vis_score.round() as u32), bar.x + bar.w * 0.5, bar.y + bar.h * 0.5, 11.5 * u, 0.5, WHITE, Some((alpha(rgb(0x5a3413), 0.9), 1.3 * u)), 0.0);
    }

    fn draw_zoom_bar(&self, g: &Gfx, time: f32) {
        let l = &self.l;
        let u = l.u;
        let r = l.zoom;
        let frac = self.s.board.charge as f32 / ZOOMIES_FULL as f32;
        let ready = self.s.board.zoomies_ready() && !self.bonus;
        let bob = if ready { 1.0 + 0.025 * (time * 9.0).sin() } else { 1.0 };
        let r = Rect::new(r.x - r.w * (bob - 1.0) / 2.0, r.y - r.h * (bob - 1.0) / 2.0, r.w * bob, r.h * bob);
        g.nine(spr::CHIP, r, 22.0, r.h * 0.42, WHITE);
        let inner = Rect::new(r.x + 5.0 * u, r.y + 5.0 * u, r.w - 10.0 * u, r.h - 10.0 * u);
        if frac > 0.0 {
            let c = if ready { mix(ORANGE, GOLD, 0.5 + 0.5 * (time * 10.0).sin()) } else { ORANGE };
            g.nine(spr::PILL, Rect::new(inner.x, inner.y, (inner.w * frac).max(inner.h), inner.h), 30.0, inner.h / 2.0, c);
            g.nine(spr::PILL, Rect::new(inner.x + 4.0 * u, inner.y + 3.0 * u, (inner.w * frac - 8.0 * u).max(inner.h * 0.5), inner.h * 0.3), 30.0, inner.h * 0.15, alpha(WHITE, 0.3));
        }
        // a little Rufus runs along the bar as it fills
        let frame = if ready && ((time * 14.0) as i32) % 2 == 0 { spr::RUFUS_RUN2 } else { spr::RUFUS_RUN1 };
        let rw = r.h * 1.45;
        let rx = (inner.x + inner.w * frac).clamp(inner.x + rw * 0.45, inner.x + inner.w - rw * 0.4);
        g.spr(frame, rx, r.y + r.h * 0.42, rw, rw * frame.h / frame.w, 0.0, WHITE);
        if ready && self.idle_t > 2.5 && self.popup == Popup::None && matches!(self.phase, Phase::Idle) {
            let bounce = (time * 5.0).sin().abs() * 9.0 * u;
            g.spr(spr::POINTER, r.x + r.w * 0.5, r.y - 26.0 * u - bounce, 50.0 * u, 50.0 * u, std::f32::consts::PI, WHITE);
        }
        let (text, size) = if ready { ("TAP FOR ZOOMIES!", 18.0) } else { ("ZOOMIES", 15.0) };
        let tx = if frac > 0.55 || ready { inner.x + inner.w * 0.36 } else { inner.x + inner.w * 0.68 };
        g.label(text, tx, r.y + r.h * 0.5, size * u * bob, 0.5, WHITE, Some((alpha(NAVY, 0.75), 1.6 * u)), 0.0);
    }

    /// The pitiful look, in close-up. There are no buttons on it: nobody has
    /// ever been given a choice about this.
    fn draw_beg(&mut self, app: &App, t: f32, given: bool) {
        let u = app.u;
        let g = &app.g;
        let cx = app.x0 + app.pw / 2.0;
        let out = ease_in(((t - (BEG_END - 0.4)) / 0.4).clamp(0.0, 1.0));
        let fade = ease_out((t / 0.3).min(1.0)) * (1.0 - out);
        g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(rgb(0x0b1026), 0.8 * fade));
        // he comes up from the bottom of the screen and goes back the same way
        let rufus_h = (app.h * 0.4).min(320.0 * u);
        let rise = ease_back((t / 0.5).min(1.0));
        let ground = app.h * 0.5 + rufus_h * 0.5 + ((1.0 - rise) + out) * app.h * 0.62;
        g.spr(spr::GLOW, cx, ground - rufus_h * 0.5, rufus_h * 1.5, rufus_h * 1.35, 0.0, alpha(rgb(0xb8d4ff), 0.42 * fade));
        self.rufus.draw(g, cx, ground, rufus_h);
        // hearts and anything else in the air, on top of the dark
        self.fx.draw(g);
        let top = app.h * 0.5 - rufus_h * 0.5;
        g.label("OUT OF MOVES...", cx, top - 52.0 * u, 25.0 * u, 0.5, alpha(WHITE, fade), Some((alpha(rgb(0x3b4a6a), fade), 1.8 * u)), 0.0);
        let line_a = ease_out(((t - 0.8) / 0.35).clamp(0.0, 1.0)) * fade;
        let below = app.h * 0.5 + rufus_h * 0.5 + 18.0 * u;
        let size = 17.0 * u;
        let lines = g.wrap(self.beg_line, size, app.pw - 56.0 * u);
        for (i, line) in lines.iter().enumerate() {
            let y = below + size * 1.25 * (i as f32 + 0.5);
            g.label(line, cx, y, size, 0.5, alpha(WHITE, line_a), Some((alpha(rgb(0x1c2a4a), line_a), 1.5 * u)), 0.0);
        }
        let line_h = lines.len() as f32 * size * 1.25;
        if given {
            let k = ease_back(((t - BEG_GIVE) / 0.3).min(1.0)) * (1.0 - out);
            let y = below + line_h + 34.0 * u;
            g.label(&format!("FINE. +{BEG_MOVES} MOVES"), cx, y, 31.0 * u * k, 0.5, GOLD, Some((rgb(0x7d0f1b), 2.2 * u)), -0.05);
        }
        self.rufus.draw_bubble(g, u, app.x0, app.x0 + app.pw);
    }

    fn dim(&self, app: &App, strength: f32) {
        let a = ease_out((self.popup_t / 0.25).min(1.0)) * strength;
        app.g.rect(Rect::new(0.0, 0.0, app.w, app.h), alpha(rgb(0x0b1026), a));
    }

    fn goal_row(&self, g: &Gfx, cx: f32, cy: f32, u: f32) {
        let goals = self.s.goals();
        let n = goals.len() as f32;
        let cw = 62.0 * u;
        for (i, goal) in goals.iter().enumerate() {
            let x = cx + (i as f32 - (n - 1.0) / 2.0) * cw;
            let size = 40.0 * u;
            match goal.icon {
                GoalIcon::Score => g.spr_fit(spr::P_STAR, x, cy, size * 0.9, 0.0, GOLD),
                GoalIcon::Kind(k) => g.spr_fit(kind_spr(k), x, cy, size, 0.0, WHITE),
                GoalIcon::Mud => g.spr_fit(spr::MUD2, x, cy, size * 0.86, 0.0, WHITE),
                GoalIcon::Bone => {
                    g.spr_fit(spr::P_DOT, x, cy, size * 1.05, 0.0, rgb(0x7fb6e8));
                    g.spr_fit(spr::BONE, x, cy, size, 0.0, WHITE);
                }
                GoalIcon::Box => g.spr_fit(spr::BOX, x, cy, size, 0.0, WHITE),
            }
            let text = match goal.icon {
                GoalIcon::Score => format!("{}", goal.need),
                GoalIcon::Mud | GoalIcon::Box => "all".to_string(),
                _ => format!("x{}", goal.need),
            };
            g.text_c(&text, x, cy + 30.0 * u, 15.0 * u, INK);
        }
    }

    fn goal_sentence(&self) -> String {
        let parts: Vec<String> = LEVELS[self.level]
            .goals
            .iter()
            .map(|g| match *g {
                Goal::Score(n) => format!("score {n}"),
                Goal::Kind(k, n) => format!("collect {n} {}", kind_name(k)),
                Goal::Mud => "clean up all the mud".to_string(),
                Goal::Bones(n) => format!("bring down {n} bone{}", if n == 1 { "" } else { "s" }),
                Goal::Boxes => "open every box".to_string(),
            })
            .collect();
        let mut s = parts.join(", ");
        if let Some(i) = s.rfind(", ") {
            s.replace_range(i..i + 2, if parts.len() > 2 { ", and " } else { " and " });
        }
        let mut c = s.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str() + ".",
            None => s,
        }
    }

    /// The card shown before a level starts. Returns true when dismissed.
    fn draw_intro(&mut self, app: &mut App) -> bool {
        let u = app.u;
        self.dim(app, 0.55);
        let g = &app.g;
        let def = &LEVELS[self.level];
        let k = ease_back((self.popup_t / 0.35).min(1.0));
        let w = 310.0 * u;
        let tip_h = g.para_height(def.tip, 15.0 * u, w - 48.0 * u);
        let goal_text = self.goal_sentence();
        let goal_h = g.para_height(&goal_text, 14.0 * u, w - 48.0 * u);
        let h = 236.0 * u + tip_h + goal_h;
        let cx = app.x0 + app.pw / 2.0;
        let top = (app.h - h) / 2.0 + (1.0 - k) * 60.0 * u;
        let r = Rect::new(cx - w / 2.0, top, w, h);
        g.nine(spr::PANEL, r, 30.0, 24.0 * u, alpha(WHITE, k.min(1.0)));
        g.spr(spr::RIBBON, cx, top + 4.0 * u, w * 0.92, w * 0.92 * 0.28, 0.0, WHITE);
        g.label(&format!("LEVEL {}", self.level + 1), cx, top - 5.0 * u, 23.0 * u, 0.5, WHITE, Some((rgb(0x7d0f1b), 1.8 * u)), 0.0);
        let mut y = top + 40.0 * u;
        g.text_c(def.title, cx, y + 12.0 * u, if g.text_w(def.title, 23.0 * u) > w - 36.0 * u { 19.0 * u } else { 23.0 * u }, INK);
        y += 36.0 * u;
        self.goal_row(g, cx, y + 22.0 * u, u);
        y += 68.0 * u;
        y += g.para(&goal_text, cx, y, 14.0 * u, w - 48.0 * u, rgb(0x6a4a2a));
        g.text_c(&format!("{} moves  ({})", self.s.moves_left, difficulty_name(app.save.difficulty).to_lowercase()), cx, y + 13.0 * u, 16.0 * u, rgb(0xd2401f));
        y += 30.0 * u;
        g.rect(Rect::new(r.x + 28.0 * u, y, w - 56.0 * u, 1.5 * u), alpha(rgb(0x9a6a3a), 0.3));
        y += 9.0 * u;
        y += g.para(def.tip, cx, y, 15.0 * u, w - 48.0 * u, rgb(0x7a4a22));
        let br = Rect::new(cx - 92.0 * u, y + 12.0 * u, 184.0 * u, 50.0 * u);
        let ready = self.popup_t > 0.3;
app.mark("go", br);
        let clicked = app.ui.button(&app.g, &mut app.au, 10, br, GREEN, "LET'S GO!", None, ready);
        clicked || (self.auto > 0 && self.popup_t > 0.7)
    }

    fn draw_pause(&mut self, app: &mut App) -> Option<Go> {
        let u = app.u;
        self.dim(app, 0.6);
        let k = ease_back((self.popup_t / 0.3).min(1.0));
        let (w, h) = (280.0 * u, 352.0 * u);
        let cx = app.x0 + app.pw / 2.0;
        let top = (app.h - h) / 2.0 + (1.0 - k) * 50.0 * u;
        let r = Rect::new(cx - w / 2.0, top, w, h);
        app.g.nine(spr::PANEL, r, 30.0, 24.0 * u, WHITE);
        app.g.text_c("PAUSED", cx, top + 30.0 * u, 26.0 * u, INK);
        app.g.text_c(&format!("Level {}: {}", self.level + 1, LEVELS[self.level].title), cx, top + 56.0 * u, 13.5 * u, rgb(0x8a6a4a));
        let bw = 200.0 * u;
        let mut go = None;
        app.mark("resume", Rect::new(cx - bw / 2.0, top + 76.0 * u, bw, 48.0 * u));
        if app.ui.button(&app.g, &mut app.au, 20, Rect::new(cx - bw / 2.0, top + 76.0 * u, bw, 48.0 * u), GREEN, "KEEP PLAYING", None, true) {
            self.popup = Popup::None;
            app.ui.eat();
        }
        app.mark("restart", Rect::new(cx - bw / 2.0, top + 132.0 * u, bw, 48.0 * u));
        if app.ui.button(&app.g, &mut app.au, 21, Rect::new(cx - bw / 2.0, top + 132.0 * u, bw, 48.0 * u), ORANGE, "START OVER", Some(spr::IC_RETRY), true) {
            go = Some(Go::Retry);
        }
        app.mark("map", Rect::new(cx - bw / 2.0, top + 188.0 * u, bw, 48.0 * u));
        if app.ui.button(&app.g, &mut app.au, 22, Rect::new(cx - bw / 2.0, top + 188.0 * u, bw, 48.0 * u), BLUE, "LEVEL MAP", Some(spr::IC_HOME), true) {
            go = Some(Go::Map);
        }
        let before = app.save.difficulty;
        sound_toggles(app, cx, top + 250.0 * u, 23);
        let note = if app.save.difficulty != before || self.changed_difficulty {
            self.changed_difficulty = true;
            "Takes effect when you start a level."
        } else {
            difficulty_blurb(app.save.difficulty)
        };
        app.g.text_c(note, cx, top + 318.0 * u, 12.5 * u, rgb(0x8a6a4a));
        go
    }

    /// The win or lose card.
    fn draw_result(&mut self, app: &mut App) -> Option<Go> {
        let u = app.u;
        let won = self.popup == Popup::Won;
        self.dim(app, if won { 0.5 } else { 0.62 });
        let t = self.popup_t;
        let k = ease_back((t / 0.4).min(1.0));
        let w = 322.0 * u;
        let line_h = app.g.para_height(self.result_line, 15.0 * u, w - 52.0 * u);
        let easier = if won { None } else { app.save.difficulty.easier() };
        let h = if won { 400.0 * u } else { 334.0 * u + if easier.is_some() { 46.0 * u } else { 0.0 } } + line_h;
        let cx = app.x0 + app.pw / 2.0;
        let top = ((app.h - h) / 2.0).max(app.top + 34.0 * u) + (1.0 - k) * 70.0 * u;
        let r = Rect::new(cx - w / 2.0, top, w, h);
        if won && t < 0.05 {
            self.fx.confetti(app.x0, app.x0 + app.pw, -10.0, 90, u);
        }
        let g = &app.g;
        g.nine(spr::PANEL, r, 30.0, 24.0 * u, WHITE);
        g.spr(spr::RIBBON, cx, top + 4.0 * u, w * 0.98, w * 0.98 * 0.28, 0.0, if won { WHITE } else { rgb(0x9fb0d0) });
        let title = if won { if self.result_paws == 3 { "BEST BOY!" } else { "GOOD BOY!" } } else { "OUT OF MOVES" };
        g.label(title, cx, top - 6.0 * u, 25.0 * u, 0.5, WHITE, Some((if won { rgb(0x7d0f1b) } else { rgb(0x3b4a6a) }, 1.8 * u)), 0.0);

        // Rufus takes his bow on the card
        let rufus_h = 150.0 * u;
        let ground = top + 40.0 * u + rufus_h;
        g.spr(spr::GLOW, cx, ground - rufus_h * 0.5, rufus_h * 2.3, rufus_h * 1.9, 0.0, alpha(if won { rgb(0xffe27a) } else { rgb(0xb8c6e6) }, 0.55));
        if won {
            // painted, not added: light would vanish against the pale card
            let rc = ground - rufus_h * 0.5;
            g.spr(spr::RAYS, cx, rc, rufus_h * 2.5, rufus_h * 2.5, t * 0.3, alpha(rgb(0xffb62e), 0.55));
            g.spr(spr::RAYS, cx, rc, rufus_h * 1.9, rufus_h * 1.9, -t * 0.2, alpha(rgb(0xfff0a8), 0.7));
        }
        self.rufus.draw(g, cx, ground, rufus_h);
        // fresh out of the lake: he waits until he is right next to you
        if won && self.lake && t > 0.55 && t < 1.55 {
            if !self.shook {
                self.shook = true;
                app.au.play(Sfx::Splash, 0.7);
                let line = self.rng.pick(&SHAKE_SAYS);
                self.rufus.say(line, 2.0);
            }
            let side = if ((t * 22.0) as i32) % 2 == 0 { 1.0 } else { -1.0 };
            self.fx.spray(self.rufus.head + vec2(side * 30.0 * u, 10.0 * u), 3, 330.0 * u, 13.0 * u);
        }
        // particles again on top of the card (confetti, paw sparkles); floating
        // text from the board stays underneath
        self.fx.draw(g);
        if won && self.lake {
            self.rufus.draw_bubble(g, u, r.x, r.x + r.w);
        }
        let mut y = ground + 10.0 * u;

        if won {
            // paws land one at a time
            for i in 0..3u8 {
                let at = 0.55 + i as f32 * 0.38;
                let x = cx + (i as f32 - 1.0) * 62.0 * u;
                let py = y + 28.0 * u - if i == 1 { 7.0 * u } else { 0.0 };
                let earned = i < self.result_paws;
                if earned && t >= at {
                    if self.paws_shown <= i {
                        self.paws_shown = i + 1;
                        app.au.play(Sfx::Paw(i), 0.9);
                        if i == 2 {
                            app.au.play(Sfx::Howl, 0.8);
                            self.rufus.bark(1);
                        }
                        platform::vibrate(20);
                        self.fx.sparkle(vec2(x, py), 22.0 * u, 8, 16.0 * u, GOLD);
                        self.fx.ring(vec2(x, py), 10.0 * u, 40.0 * u, 0.4, GOLD, 0.0);
                    }
                    let s = 56.0 * u * (1.0 + 0.5 * wobble(t - at, 16.0, 7.0).max(-0.3));
                    g.spr_fit(spr::PAW_GOLD, x, py, s, (i as f32 - 1.0) * 0.18, WHITE);
                } else {
                    g.spr_fit(spr::PAW_GRAY, x, py, 52.0 * u, (i as f32 - 1.0) * 0.18, WHITE);
                }
            }
            y += 64.0 * u;
            let shown = (self.s.board.score as f32 * ease_out((t - 0.4) / 1.2)).round() as u32;
            g.text_c(&format!("{shown}"), cx, y + 12.0 * u, 30.0 * u, INK);
            if self.new_best && t > 1.6 {
                let pop = ease_back(((t - 1.6) / 0.3).min(1.0));
                g.label("NEW BEST!", cx + 92.0 * u, y + 4.0 * u, 13.0 * u * pop, 0.5, rgb(0xe8323c), None, 0.2);
            }
            // this win opened a page of Rufus's story: it is shown on the way out
            if crate::story::unseen(&app.save) > 0 && t > 1.9 {
                let pop = ease_back(((t - 1.9) / 0.3).min(1.0));
                g.label("STORY PAGE!", cx - 94.0 * u, y + 4.0 * u, 13.0 * u * pop, 0.5, rgb(0xd2701f), None, -0.2);
            }
            y += 34.0 * u;
        } else {
            y += 4.0 * u;
            let goals = self.s.goals();
            let missing: Vec<String> = goals
                .iter()
                .filter(|g| !g.done())
                .map(|g| match g.icon {
                    GoalIcon::Score => format!("{} points", g.left()),
                    GoalIcon::Kind(k) => format!("{} {}", g.left(), kind_name(k)),
                    GoalIcon::Mud => format!("{} muddy patch{}", g.left(), if g.left() == 1 { "" } else { "es" }),
                    GoalIcon::Bone => format!("{} bone{}", g.left(), if g.left() == 1 { "" } else { "s" }),
                    GoalIcon::Box => format!("{} box{}", g.left(), if g.left() == 1 { "" } else { "es" }),
                })
                .collect();
            g.text_c(&format!("Still needed: {}.", missing.join(", ")), cx, y + 10.0 * u, if missing.len() > 1 { 12.5 } else { 15.0 } * u, rgb(0x8a6a4a));
            y += 30.0 * u;
        }

        g.rect(Rect::new(r.x + 28.0 * u, y, w - 56.0 * u, 1.5 * u), alpha(rgb(0x9a6a3a), 0.3));
        y += 9.0 * u;
        if won {
            // the very last chapter is a short one, so its last level counts too
            let chapter_end = (self.level + 1) % CHAPTER_LEN == 0 || self.level + 1 == LEVELS.len();
            let heading = if chapter_end { format!("CHAPTER COMPLETE: {}", CHAPTERS[self.level / CHAPTER_LEN].name.to_uppercase()) } else { "THE RUFUS REPORT".to_string() };
            g.text_c(&heading, cx, y + 6.0 * u, 10.5 * u, if chapter_end { rgb(0xd2401f) } else { rgb(0xb98a5a) });
            y += 16.0 * u;
        }
        y += g.para(self.result_line, cx, y, 15.0 * u, w - 52.0 * u, rgb(0x6a4a2a));
        y += 14.0 * u;

        let mut go = None;
        let ready = t > if won { 0.9 } else { 0.5 };
        let last = self.level + 1 >= LEVELS.len();
        let bh = 52.0 * u;
        let small = 52.0 * u;
        if won {
            let big = if last { 0.0 } else { 150.0 * u };
            let total = small * 2.0 + big + 10.0 * u * if last { 1.0 } else { 2.0 };
            let mut x = cx - total / 2.0;
            if ready {
                app.mark("home", Rect::new(x, y, small, bh));
                app.mark("retry", Rect::new(x + small + 10.0 * u, y, small, bh));
                if !last {
                    app.mark("next", Rect::new(x + (small + 10.0 * u) * 2.0, y, big, bh));
                }
            }
            if app.ui.button(&app.g, &mut app.au, 30, Rect::new(x, y, small, bh), BLUE, "", Some(spr::IC_HOME), ready) {
                go = Some(Go::Map);
            }
            x += small + 10.0 * u;
            if app.ui.button(&app.g, &mut app.au, 31, Rect::new(x, y, small, bh), ORANGE, "", Some(spr::IC_RETRY), ready) {
                go = Some(Go::Retry);
            }
            x += small + 10.0 * u;
            if !last && app.ui.button(&app.g, &mut app.au, 32, Rect::new(x, y, big, bh), GREEN, "NEXT", Some(spr::IC_NEXT), ready) {
                go = Some(Go::Next);
            }
            if self.auto >= 2 && t > 4.0 {
                go = Some(if last { Go::Map } else { Go::Next });
            }
        } else {
            let big = 176.0 * u;
            let total = small + big + 10.0 * u;
            let x = cx - total / 2.0;
            if ready {
                app.mark("home", Rect::new(x, y, small, bh));
                app.mark("retry", Rect::new(x + small + 10.0 * u, y, big, bh));
            }
            if app.ui.button(&app.g, &mut app.au, 30, Rect::new(x, y, small, bh), BLUE, "", Some(spr::IC_HOME), ready) {
                go = Some(Go::Map);
            }
            if app.ui.button(&app.g, &mut app.au, 31, Rect::new(x + small + 10.0 * u, y, big, bh), ORANGE, "TRY AGAIN", Some(spr::IC_RETRY), ready) {
                go = Some(Go::Retry);
            }
            if let Some(d) = easier {
                let er = Rect::new(cx - 104.0 * u, y + bh + 12.0 * u, 208.0 * u, 36.0 * u);
                if ready {
                    app.mark("easier", er);
                }
                if app.ui.button(&app.g, &mut app.au, 33, er, PURPLE, &format!("TOO HARD? TRY {}", difficulty_name(d)), None, ready) {
                    app.save.difficulty = d;
                    app.save.store();
                    go = Some(Go::Retry);
                }
            }
            if self.auto >= 2 && t > 4.0 {
                go = Some(Go::Retry);
            }
        }
        go
    }
}

/// Sound, music and difficulty, shared by the pause card and the title
/// screen. The difficulty button cycles Easy, Normal, Hard.
pub fn sound_toggles(app: &mut App, cx: f32, y: f32, id: u32) {
    let u = app.u;
    let s = 46.0 * u;
    let dw = 118.0 * u;
    let gap = 10.0 * u;
    let x0 = cx - (s * 2.0 + dw + gap * 2.0) / 2.0;
    let sfx_icon = if app.save.sfx { spr::IC_SOUND } else { spr::IC_MUTE };
    if app.ui.button(&app.g, &mut app.au, id, Rect::new(x0, y, s, s), if app.save.sfx { PURPLE } else { GREY }, "", Some(sfx_icon), true) {
        app.save.sfx = !app.save.sfx;
        app.au.sfx_on = app.save.sfx;
        app.save.store();
        app.au.play(Sfx::Yip, 0.7);
    }
    let music_icon = if app.save.music { spr::IC_MUSIC } else { spr::IC_NOMUSIC };
    if app.ui.button(&app.g, &mut app.au, id + 1, Rect::new(x0 + s + gap, y, s, s), if app.save.music { PURPLE } else { GREY }, "", Some(music_icon), true) {
        app.save.music = !app.save.music;
        app.au.music_on = app.save.music;
        app.save.store();
    }
    let d = app.save.difficulty;
    let color = match d {
        Difficulty::Easy => GREEN,
        Difficulty::Normal => BLUE,
        Difficulty::Hard => rgb(0xe8473f),
    };
    let dr = Rect::new(x0 + (s + gap) * 2.0, y, dw, s);
    app.mark("difficulty", dr);
    if app.ui.button(&app.g, &mut app.au, id + 2, dr, color, difficulty_name(d), None, true) {
        app.save.difficulty = Difficulty::from_index((d.index() + 1) % 3);
        app.save.store();
    }
}

/// Screen position of a zoomie runner's nose, and the unit vector behind him.
fn runner_pos(l: &L, r: &Runner) -> (Vec2, Vec2) {
    let rank = (r.t * 1000.0 - RUN_LEAD as f32) / RUN_STEP as f32;
    let len = if r.horizontal { COLS } else { ROWS } as f32;
    let along = if r.dir > 0 { rank } else { len - 1.0 - rank };
    let back = if r.horizontal { vec2(-(r.dir as f32), 0.0) } else { vec2(0.0, -(r.dir as f32)) };
    let p = if r.horizontal { l.cell(along, r.line as f32) } else { l.cell(r.line as f32, along) };
    (p, back)
}

fn zap_pos(l: &L, z: &Zap) -> Vec2 {
    let k = ease_io((z.t / z.dur).min(1.0));
    let (a, b) = (l.cell(z.from.x, z.from.y), l.cell(z.to.x, z.to.y));
    let d = b - a;
    let arc = vec2(-d.y, d.x) * 0.22 * (k * std::f32::consts::PI).sin();
    a.lerp(b, k) + arc
}

/// One board piece, with whatever marks it as a power piece.
#[allow(clippy::too_many_arguments)]
/// One piece, placed and ready to draw. The board collects these and draws
/// them in layers: light underneath, the pieces, then markers on top.
#[derive(Clone, Copy)]
pub struct ItemDraw {
    pub item: Item,
    pub c: Vec2,
    pub size: f32,
    pub t: f32,
    pub sx: f32,
    pub sy: f32,
    pub rot: f32,
    pub a: f32,
    pub flash: f32,
}

/// The light a piece gives off. Draw inside [`Gfx::lights`].
pub fn item_light(g: &Gfx, d: &ItemDraw) {
    let (c, size, t, a) = (d.c, d.size, d.t, d.a);
    // without shader support these are painted, not added, so go easier
    let k = if g.has_light() { 1.0 } else { 0.7 };
    match d.item {
        Item::Normal { kind, special } => match special {
            Special::LineH | Special::LineV => {
                let pulse = 0.6 + 0.3 * (t * 6.0).sin();
                g.spr(spr::GLOW, c.x, c.y, size * 1.75, size * 1.75, 0.0, alpha(kind_color(kind), a * pulse * k));
                // a streak of light along the way he is going to run
                let r0 = if special == Special::LineH { 0.0 } else { std::f32::consts::FRAC_PI_2 };
                g.spr(spr::STREAK, c.x, c.y, size * 2.1, size * 0.36, r0, alpha(WHITE, a * 0.4 * pulse * k));
            }
            Special::Bomb => {
                let pulse = 0.55 + 0.25 * (t * 5.0).sin();
                g.spr(spr::GLOW, c.x, c.y, size * 2.0, size * 2.0, 0.0, alpha(rgb(0xff8a2a), a * pulse * k));
            }
            Special::None => {}
        },
        Item::Super => {
            g.spr(spr::RAYS, c.x, c.y, size * 2.5, size * 2.5, t * 0.5, alpha(GOLD, a * 0.6 * k));
            g.spr(spr::RAYS, c.x, c.y, size * 1.9, size * 1.9, -t * 0.35, alpha(rgb(0xfff2b8), a * 0.45 * k));
            g.spr(spr::GLOW, c.x, c.y, size * 1.7, size * 1.7, 0.0, alpha(GOLD, a * 0.6 * k));
        }
        Item::Bone => g.spr(spr::GLOW, c.x, c.y, size * 1.5, size * 1.5, 0.0, alpha(rgb(0xfff2b8), a * 0.5 * k)),
        Item::Box { .. } => {}
    }
}

/// The piece itself.
pub fn item_body(g: &Gfx, d: &ItemDraw) {
    let (c, size, t) = (d.c, d.size, d.t);
    let tint = alpha(WHITE, d.a);
    let s = item_spr(d.item);
    match d.item {
        Item::Normal { special, .. } => {
            if special == Special::Bomb {
                let pulse = 1.0 + 0.07 * (t * 5.0).sin();
                g.spr(spr::FX_BURST, c.x, c.y, size * 1.22 * pulse, size * 1.22 * pulse, t * 0.6, tint);
            }
            let shrink = if special == Special::None { 1.0 } else { 0.86 };
            g.spr(s, c.x, c.y, size * d.sx * shrink, size * d.sy * shrink, d.rot, tint);
        }
        Item::Super => g.spr(s, c.x, c.y, size * d.sx, size * d.sy, d.rot + 0.07 * (t * 3.0).sin(), tint),
        Item::Bone => g.spr(s, c.x, c.y, size * d.sx, size * d.sy, d.rot + 0.06 * (t * 2.4).sin(), tint),
        Item::Box { .. } => g.spr(s, c.x, c.y, size * d.sx, size * d.sy, d.rot, tint),
    }
}

/// Markers that sit over a piece: which way the zoomies will go, and the
/// sparkle that circles Golden Rufus.
pub fn item_top(g: &Gfx, d: &ItemDraw) {
    let (c, size, t) = (d.c, d.size, d.t);
    let tint = alpha(WHITE, d.a);
    match d.item {
        Item::Normal { special, .. } if special.is_line() => {
            let out = size * (0.42 + 0.05 * (t * 7.0).sin());
            let (dx, dy, r0) = if special == Special::LineH { (out, 0.0, 0.0) } else { (0.0, out, std::f32::consts::FRAC_PI_2) };
            g.spr(spr::FX_ARROW, c.x + dx, c.y + dy, size * 0.42, size * 0.42, r0, tint);
            g.spr(spr::FX_ARROW, c.x - dx, c.y - dy, size * 0.42, size * 0.42, r0 + std::f32::consts::PI, tint);
        }
        Item::Super => {
            let sp = (t * 1.7).fract();
            let ang = (t * 1.7).floor() * 2.4;
            let k = size * 0.3 * (sp * std::f32::consts::PI).sin();
            g.spr(spr::P_SPARK, c.x + ang.cos() * size * 0.36, c.y + ang.sin() * size * 0.36, k, k, 0.0, tint);
        }
        _ => {}
    }
}
