//! Rule tests. The fuzzers at the bottom matter most: they play thousands of
//! random games and check, after every single step, both that the board is
//! legal and that the reports it emitted are enough to reconstruct it. That
//! second check is what the animation layer depends on.

use rufus_core::bot::{self, BotCfg};
use rufus_core::*;
use std::collections::HashMap;

fn p(x: usize, y: usize) -> Pos {
    Pos::new(x, y)
}

/// A second copy of the board, kept up to date only from reports, the same
/// way the renderer does it.
struct Shadow {
    pieces: HashMap<u32, (Pos, Item)>,
}

impl Shadow {
    fn of(b: &Board) -> Shadow {
        Shadow { pieces: b.pieces().map(|(pos, pc)| (pc.id, (pos, pc.item))).collect() }
    }

    fn swap(&mut self, a: Pos, b: Pos) {
        for (pos, _) in self.pieces.values_mut() {
            if *pos == a {
                *pos = b;
            } else if *pos == b {
                *pos = a;
            }
        }
    }

    fn apply(&mut self, step: &Step) {
        match step {
            Step::Clear(rep) => {
                for m in &rep.morphs {
                    let e = self.pieces.get_mut(&m.id).expect("morph of unknown piece");
                    assert_eq!(e.0, m.pos);
                    e.1 = m.item;
                }
                let mut events: Vec<(u32, u8, usize)> = Vec::new();
                events.extend(rep.pops.iter().enumerate().map(|(i, x)| (x.t, 0, i)));
                events.extend(rep.box_hits.iter().enumerate().map(|(i, x)| (x.t, 1, i)));
                events.sort();
                for (_, kind, i) in events {
                    if kind == 0 {
                        let pop = &rep.pops[i];
                        let (pos, _) = self.pieces.remove(&pop.id).expect("pop of unknown piece");
                        assert_eq!(pos, pop.pos, "pop at wrong position");
                    } else {
                        let hit = &rep.box_hits[i];
                        let e = self.pieces.get_mut(&hit.id).expect("hit on unknown box");
                        assert_eq!(e.0, hit.pos);
                        if hit.hp_left == 0 {
                            self.pieces.remove(&hit.id);
                        } else {
                            e.1 = Item::Box { hp: hit.hp_left };
                        }
                    }
                }
                for m in &rep.made {
                    assert!(self.pieces.insert(m.id, (m.pos, m.item)).is_none(), "made a duplicate id");
                }
                let last = rep.pops.iter().map(|x| x.t).max().unwrap_or(0);
                assert!(rep.duration >= last, "report ends before its last pop");
                for pop in &rep.pops {
                    if let Some(b) = pop.blast {
                        assert!((b as usize) < rep.blasts.len());
                        assert!(pop.t >= rep.blasts[b as usize].t, "popped before its blast started");
                    }
                }
            }
            Step::Fall(rep) => {
                for f in &rep.falls {
                    if f.spawned {
                        assert!(self.pieces.insert(f.id, (f.to, f.item)).is_none(), "spawned a duplicate id");
                        assert!(f.from_y < f.to.y as i32);
                    } else {
                        let e = self.pieces.get_mut(&f.id).expect("fall of unknown piece");
                        assert_eq!((e.0.x, e.0.y as i32), (f.to.x, f.from_y), "fall from wrong cell");
                        assert!(f.to.y as i32 > f.from_y, "pieces only fall down");
                        e.0 = f.to;
                    }
                }
            }
            Step::Bones(list) => {
                for (id, pos) in list {
                    let (at, item) = self.pieces.remove(id).expect("unknown bone");
                    assert_eq!((at, item), (*pos, Item::Bone));
                }
            }
            Step::Shuffle(list) => {
                for s in list {
                    let e = self.pieces.get_mut(&s.id).expect("shuffle of unknown piece");
                    assert_eq!(e.0, s.from);
                    *e = (s.to, s.item);
                }
            }
            Step::Idle => {}
        }
    }

    fn assert_matches(&self, b: &Board) {
        let real: HashMap<u32, (Pos, Item)> = b.pieces().map(|(pos, pc)| (pc.id, (pos, pc.item))).collect();
        assert_eq!(real.len(), self.pieces.len(), "piece count drifted");
        for (id, v) in &real {
            assert_eq!(self.pieces.get(id), Some(v), "piece {id} drifted");
        }
    }
}

fn has_run(b: &Board) -> bool {
    let k = |x: usize, y: usize| b.get(x, y).piece.and_then(|pc| pc.item.kind()).filter(|_| b.get(x, y).exists);
    for y in 0..ROWS {
        for x in 0..COLS {
            let Some(kind) = k(x, y) else { continue };
            if x + 2 < COLS && k(x + 1, y) == Some(kind) && k(x + 2, y) == Some(kind) {
                return true;
            }
            if y + 2 < ROWS && k(x, y + 1) == Some(kind) && k(x, y + 2) == Some(kind) {
                return true;
            }
        }
    }
    false
}

fn assert_settled(b: &Board) {
    for y in 0..ROWS {
        for x in 0..COLS {
            let c = b.get(x, y);
            if c.exists {
                assert!(c.piece.is_some(), "hole left at {x},{y}");
            } else {
                assert!(c.piece.is_none() && c.mud == 0, "piece off the board at {x},{y}");
            }
        }
    }
    assert!(!has_run(b), "unresolved match left on a settled board");
    assert!(!b.find_moves().is_empty(), "settled with no legal move");
    assert_eq!(b.cascade, 0);
}

fn settle(b: &mut Board, shadow: &mut Shadow) -> Vec<Step> {
    let mut steps = Vec::new();
    loop {
        let step = b.advance();
        shadow.apply(&step);
        if matches!(step, Step::Idle) {
            break;
        }
        steps.push(step);
        assert!(steps.len() < 2000, "never settled");
    }
    shadow.assert_matches(b);
    assert_settled(b);
    steps
}

fn clears(steps: &[Step]) -> Vec<&ClearReport> {
    steps.iter().filter_map(|s| if let Step::Clear(r) = s { Some(r) } else { None }).collect()
}

const ALL: &[u8] = &[0, 1, 2, 3, 4, 5];
const FULL: [&str; 9] = [".......", ".......", ".......", ".......", ".......", ".......", ".......", ".......", "......."];

// ---------------------------------------------------------------------------

#[test]
fn new_boards_start_clean() {
    for seed in 0..300 {
        for kinds in 3..=6 {
            let b = Board::from_rows(&FULL, &ALL[..kinds], seed);
            assert_settled(&b);
        }
    }
}

#[test]
fn every_level_builds_and_starts_playable() {
    for (i, def) in LEVELS.iter().enumerate() {
        for row in def.rows {
            assert_eq!(row.chars().count(), COLS, "level {} has a row of the wrong width", i + 1);
        }
        assert!(def.paws[0] < def.paws[1], "level {} paw thresholds out of order", i + 1);
        for g in def.goals {
            if let Goal::Kind(k, _) = g {
                assert!(def.kinds.contains(k), "level {} asks for a kind that isn't in play", i + 1);
            }
        }
        for seed in 0..40 {
            let s = Session::new(i, seed);
            assert_settled(&s.board);
            assert!(!s.won(), "level {} is won before the first move", i + 1);
            for g in s.goals() {
                assert!(g.need > 0, "level {} has an empty goal", i + 1);
            }
        }
    }
}

#[test]
fn illegal_swaps_change_nothing() {
    let rows = ["0123450", "1234501", "2345012", "3450123", "4501234", "5012345", "0123450", "1234501", "2345012"];
    let mut b = Board::from_rows(&rows, ALL, 1);
    assert!(b.find_moves().is_empty());
    let before: Vec<_> = b.pieces().collect();
    assert!(!b.swap(p(0, 0), p(1, 0)));
    assert!(!b.swap(p(0, 0), p(2, 0)), "non-neighbours can never swap");
    assert!(!b.swap(p(3, 3), p(3, 3)));
    assert_eq!(before, b.pieces().collect::<Vec<_>>());
}

#[test]
fn a_session_only_charges_for_real_moves() {
    let mut s = Session::new(0, 3);
    let m = s.board.find_moves()[0];
    let all: Vec<(Pos, Pos)> = (0..COLS)
        .flat_map(|x| (0..ROWS).map(move |y| (p(x, y), p((x + 1) % COLS, y))))
        .collect();
    let bad = all.into_iter().find(|&(a, b)| !s.board.can_swap(a, b)).unwrap();
    let moves = s.moves_left;
    assert!(!s.swap(bad.0, bad.1));
    assert_eq!(s.moves_left, moves);
    assert!(s.swap(m.a, m.b));
    assert_eq!(s.moves_left, moves - 1);
}

#[test]
fn match_three_clears_three() {
    let rows = ["0010234", "1234512", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    let mut sh = Shadow::of(&b);
    assert!(b.swap(p(2, 0), p(3, 0)));
    sh.swap(p(2, 0), p(3, 0));
    let steps = settle(&mut b, &mut sh);
    let first = clears(&steps)[0];
    assert_eq!(first.cascade, 1);
    assert_eq!(first.groups.len(), 1);
    assert_eq!(first.groups[0].size, 3);
    assert!(first.made.is_empty());
    assert!(first.pops.len() >= 3);
    assert!(b.tally.kinds[0] >= 3);
    assert!(b.score >= 30);
}

#[test]
fn match_four_makes_zoomies_in_the_swipe_direction() {
    // vertical swipe completes a horizontal four
    let rows = ["0001234", "3450512", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    let mut b2 = b.clone();
    let mut sh = Shadow::of(&b);
    assert!(b.swap(p(3, 1), p(3, 0)));
    sh.swap(p(3, 1), p(3, 0));
    let steps = settle(&mut b, &mut sh);
    let first = clears(&steps)[0];
    assert_eq!(first.groups[0].size, 4);
    assert_eq!(first.made.len(), 1);
    assert_eq!(first.made[0].item, Item::Normal { kind: 0, special: Special::LineV });
    assert_eq!(first.made[0].pos, p(3, 0), "special appears where the player put the piece");
    assert!(first.pops.iter().all(|x| x.into == Some(p(3, 0))));

    // same match from the other side is still a vertical swipe
    assert!(b2.swap(p(3, 0), p(3, 1)));
    let first = match b2.advance() {
        Step::Clear(r) => r,
        other => panic!("{other:?}"),
    };
    assert_eq!(first.made[0].pos, p(3, 0));
}

#[test]
fn five_in_a_line_makes_golden_rufus_and_an_l_makes_a_woof() {
    // five in a row
    let rows = ["0010034", "3401512", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    assert!(b.swap(p(2, 1), p(2, 0)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.made.len(), 1);
    assert_eq!(r.made[0].item, Item::Super);

    // four in a column, completed by a sideways swipe: the runner goes sideways
    let rows = ["0012345", "0234512", "3045123", "0451234", "4512345", "5123451", "1234512", "2345123", "3451234"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    assert!(b.swap(p(1, 2), p(0, 2)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.made[0].item.special(), Special::LineH, "horizontal swipe, so a row runner");

    // an L: three across and three down sharing a corner
    let rows = ["1002345", "0234512", "0345123", "4551234", "4512345", "5123451", "1234512", "2345123", "3451234"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    b.place(p(0, 0), Item::plain(0));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.groups.len(), 1);
    assert_eq!(r.groups[0].size, 5);
    assert_eq!(r.made[0].item, Item::Normal { kind: 0, special: Special::Bomb });
    assert_eq!(r.made[0].pos, p(0, 0), "woof appears at the corner of the L");
}

#[test]
fn zoomies_piece_clears_its_whole_row() {
    let rows = ["-012345", "0134512", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    // (0,1)=0 ... make a match including the special at (0,0): column 0 = -(0),0,2 -> need another 0
    b.place(p(0, 2), Item::plain(0));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.blasts.len(), 1);
    assert!(matches!(r.blasts[0].kind, BlastKind::Row { .. }));
    let row0: Vec<_> = r.pops.iter().filter(|x| x.pos.y == 0).collect();
    assert_eq!(row0.len(), COLS, "all of row 0 cleared");
    // run order: pops along the row happen in sequence
    let mut ts: Vec<u32> = row0.iter().filter(|x| x.cause == Cause::Row).map(|x| x.t).collect();
    ts.sort();
    ts.dedup();
    assert_eq!(ts.len(), COLS - 1, "one cell per beat as Rufus runs past");
}

#[test]
fn big_woof_clears_a_diamond_and_spares_bones() {
    let rows = ["1234512", "2345123", "3451234", "451*345", "5123451", "1234512", "2345123", "3451234", "4512345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    b.place(p(3, 2), Item::Bone);
    b.place(p(2, 3), Item::Box { hp: 2 });
    // set it off with a blast: fire the bomb directly
    assert!(b.fire_powers());
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.blasts.len(), 1);
    // diamond of 13 cells, minus the bone and the box
    assert_eq!(r.pops.len(), 11);
    assert_eq!(r.box_hits.len(), 1);
    assert_eq!(r.box_hits[0].hp_left, 1);
    assert_eq!(b.cell(p(3, 2)).piece.unwrap().item, Item::Bone);
    assert_eq!(b.cell(p(2, 3)).piece.unwrap().item, Item::Box { hp: 1 });
}

#[test]
fn golden_rufus_fetches_every_piece_of_the_kind_he_is_swapped_with() {
    let rows = ["@123451", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234", "4512345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    let ones = b.pieces().filter(|(_, pc)| pc.item.kind() == Some(1)).count();
    assert!(b.swap(p(0, 0), p(1, 0)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert!(matches!(r.blasts[0].kind, BlastKind::Fetch { kind: 1 }));
    assert_eq!(r.pops.iter().filter(|x| x.item.kind() == Some(1)).count(), ones);
    assert_eq!(r.pops.iter().filter(|x| x.item == Item::Super).count(), 1);
    assert_eq!(b.tally.kinds[1] as usize, ones);
}

#[test]
fn power_piece_combos() {
    // zoomies + zoomies: a full cross
    let rows = ["1234512", "2345123", "34-|234", "4512345", "5123451", "1234512", "2345123", "3451234", "4512345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    assert!(b.swap(p(2, 2), p(3, 2)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.blasts.len(), 2);
    assert_eq!(r.pops.iter().filter(|x| x.pos.y == 2).count(), COLS);
    assert_eq!(r.pops.iter().filter(|x| x.pos.x == 3).count(), ROWS);

    // woof + woof: the big one
    let rows = ["1234512", "2345123", "3451234", "451**45", "5123451", "1234512", "2345123", "3451234", "4512345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    assert!(b.swap(p(3, 3), p(4, 3)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert!(matches!(r.blasts[0].kind, BlastKind::Bomb { big: true }));
    assert!(r.pops.len() >= 25);

    // golden + golden: the whole board
    let rows = ["1234512", "2345123", "3451234", "451@@45", "5123451", "1234512", "2345123", "3451234", "4512345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    assert!(b.swap(p(3, 3), p(4, 3)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.pops.len(), N);

    // golden + zoomies: every piece of that kind becomes zoomies, then goes
    let rows = ["1234512", "2345123", "3451234", "451@-45", "5123451", "1234512", "2345123", "3451234", "4512345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    b.place(p(0, 0), Item::plain(0));
    b.place(p(6, 8), Item::plain(0));
    assert!(b.swap(p(3, 3), p(4, 3)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.morphs.len(), 2);
    assert!(r.morphs.iter().all(|m| m.item.special().is_line()));
    assert!(r.blasts.len() >= 4, "fetch + three line blasts");
}

#[test]
fn mud_boxes_and_bones() {
    // mud comes off when a match happens on it
    let rows = ["0010234", "1234512", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    let mut muddy = Board::from_rows(
        &["mmMm234", "1234512", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234"],
        ALL,
        7,
    );
    assert_eq!(muddy.mud_left(), 5);
    for (pos, pc) in b.pieces() {
        if pos.y == 0 && pos.x < 4 {
            muddy.place(pos, pc.item);
        }
    }
    assert!(muddy.swap(p(2, 0), p(3, 0)));
    let Step::Clear(r) = muddy.advance() else { panic!() };
    assert_eq!(r.mud_hits.len(), 3);
    assert_eq!(muddy.mud_left(), 2, "the double layer keeps one, the untouched tile keeps one");
    assert_eq!(muddy.tally.mud, 3);

    // a box next to a match takes one hit per step
    b.place(p(1, 1), Item::Box { hp: 1 });
    b.place(p(2, 1), Item::Box { hp: 2 });
    b.place(p(5, 5), Item::Box { hp: 1 });
    assert!(b.swap(p(2, 0), p(3, 0)));
    let Step::Clear(r) = b.advance() else { panic!() };
    assert_eq!(r.box_hits.len(), 2, "only the boxes touching the match");
    assert_eq!(b.tally.boxes, 1);
    assert!(b.cell(p(1, 1)).piece.is_none());
    assert_eq!(b.cell(p(2, 1)).piece.unwrap().item, Item::Box { hp: 1 });

    // a bone on the bottom row is delivered once things settle
    let rows = ["1234512", "2345123", "3451234", "4512345", "5123451", "1234512", "2345123", "3451234", "451o345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    let mut sh = Shadow::of(&b);
    let steps = settle(&mut b, &mut sh);
    assert!(steps.iter().any(|s| matches!(s, Step::Bones(v) if v.len() == 1)));
    assert_eq!(b.tally.bones, 1);
}

#[test]
fn pieces_fall_through_gaps_in_a_column() {
    let rows = ["1234512", "2345123", "3 51234", "4 12345", "5123451", "1234512", "2345123", "3451234", "4512345"];
    let mut b = Board::from_rows(&rows, ALL, 7);
    // clear the bottom row; column 1 has no tiles at rows 2 and 3
    b.place(p(1, 8), Item::Normal { kind: 5, special: Special::LineH });
    let above_gap = b.cell(p(1, 1)).piece.unwrap().id;
    let mut sh = Shadow::of(&b);
    assert!(b.fire_powers());
    let steps = settle(&mut b, &mut sh);
    let fall = steps.iter().find_map(|s| if let Step::Fall(f) = s { Some(f) } else { None }).unwrap();
    let crossed = fall.falls.iter().find(|f| f.id == above_gap).expect("piece above the gap moved");
    assert_eq!((crossed.from_y, crossed.to), (1, p(1, 4)), "it dropped straight across the gap");
}

#[test]
fn a_stuck_board_shuffles_itself() {
    let rows = ["0123450", "1234501", "2345012", "3450123", "4501234", "5012345", "0123450", "1234501", "2345012"];
    let mut b = Board::from_rows(&rows, ALL, 5);
    assert!(b.find_moves().is_empty());
    let mut sh = Shadow::of(&b);
    let steps = settle(&mut b, &mut sh);
    assert!(matches!(steps[0], Step::Shuffle(_)));
}

#[test]
fn zoomies_needs_a_full_meter_and_costs_no_move() {
    let mut s = Session::new(5, 11);
    assert!(!s.board.zoomies());
    s.board.charge = ZOOMIES_FULL;
    let moves = s.moves_left;
    let mut sh = Shadow::of(&s.board);
    assert!(s.board.zoomies());
    assert_eq!(s.board.charge, 0);
    let steps = settle(&mut s.board, &mut sh);
    let first = clears(&steps)[0];
    assert_eq!(first.blasts.len(), 3, "two rows and a column");
    assert!(first.pops.len() >= 15);
    assert_eq!(s.moves_left, moves);
    assert_eq!(s.board.charge, 0, "zoomies don't refill their own meter");
}

#[test]
fn the_end_of_level_bonus_is_capped_and_the_rest_is_paid_out() {
    let mut s = Session::new(5, 4);
    s.moves_left = 25;
    let before = s.board.score;
    let mut planted = 0;
    while s.plant_bonus().is_some() {
        planted += 1;
    }
    assert_eq!(planted, level::BONUS_PLANTS);
    assert_eq!(s.moves_left, 25 - level::BONUS_PLANTS);
    let paid = s.cash_in_moves();
    assert_eq!(paid, (25 - level::BONUS_PLANTS) * level::PTS_SPARE_MOVE);
    assert_eq!(s.moves_left, 0);
    assert_eq!(s.board.score, before + planted * PTS_BONUS + paid);
}

#[test]
fn seeds_are_reproducible() {
    let a = bot::play(11, 99, &BotCfg::CASUAL);
    let b = bot::play(11, 99, &BotCfg::CASUAL);
    assert_eq!((a.won, a.score, a.moves_used), (b.won, b.score, b.moves_used));
}

// ---- fuzzers ---------------------------------------------------------------

/// Random legal moves on every level, shadow-checked after every step.
#[test]
fn fuzz_random_play_on_every_level() {
    let mut rng = Rng::new(2024);
    for level in 0..LEVELS.len() {
        // the hand-made levels get the full treatment; the thousand designed
        // ones share their rules, so a few games each is plenty
        let games = if level < level::HAND_MADE { 25 } else { 3 };
        for seed in 0..games as u64 {
            let mut s = Session::new(level, seed * 7919 + level as u64);
            let mut sh = Shadow::of(&s.board);
            settle(&mut s.board, &mut sh);
            let mut ids_seen = 0u32;
            while s.moves_left > 0 {
                if s.board.zoomies_ready() && rng.chance(0.7) {
                    assert!(s.board.zoomies());
                    settle(&mut s.board, &mut sh);
                    continue;
                }
                let moves = s.board.find_moves();
                let m = rng.pick(&moves);
                // an illegal swap first, now and then: must be a no-op
                if rng.chance(0.2) {
                    let (a, b) = (p(rng.below(7) as usize, rng.below(9) as usize), p(rng.below(7) as usize, rng.below(9) as usize));
                    if !s.board.can_swap(a, b) {
                        assert!(!s.swap(a, b));
                        sh.assert_matches(&s.board);
                    }
                }
                assert!(s.swap(m.a, m.b));
                sh.swap(m.a, m.b);
                let steps = settle(&mut s.board, &mut sh);
                assert!(!clears(&steps).is_empty(), "a legal move cleared nothing");
                for r in clears(&steps) {
                    for pop in &r.pops {
                        ids_seen = ids_seen.max(pop.id);
                    }
                }
            }
            // end-of-level bonus path, shadowed too
            s.moves_left = 5;
            while let Some(m) = s.plant_bonus() {
                sh.pieces.get_mut(&m.id).unwrap().1 = m.item;
            }
            while s.board.fire_powers() {
                settle(&mut s.board, &mut sh);
            }
            assert!(!s.board.has_powers());
            assert!(ids_seen > 0);
        }
    }
}

/// Boards seeded with lots of power pieces, to shake out chain reactions.
#[test]
fn fuzz_power_piece_chain_reactions() {
    let mut rng = Rng::new(77);
    for seed in 0..400u64 {
        let mut b = Board::from_rows(&FULL, &ALL[..4 + (seed % 3) as usize], seed);
        for _ in 0..(6 + rng.below(14)) {
            let pos = p(rng.below(7) as usize, rng.below(9) as usize);
            let kind = rng.pick(b.palette());
            let item = match rng.below(7) {
                0 => Item::Super,
                1 | 2 => Item::Normal { kind, special: Special::LineH },
                3 | 4 => Item::Normal { kind, special: Special::LineV },
                5 => Item::Normal { kind, special: Special::Bomb },
                _ => Item::Box { hp: 1 + rng.below(2) as u8 },
            };
            b.place(pos, item);
        }
        let mut sh = Shadow::of(&b);
        settle(&mut b, &mut sh);
        for _ in 0..12 {
            let moves = b.find_moves();
            // prefer combos when there are any: that's what we're here to test
            let combo = moves.iter().find(|m| {
                b.cell(m.a).piece.unwrap().item.is_power() && b.cell(m.b).piece.unwrap().item.is_power()
            });
            let m = *combo.unwrap_or(&rng.pick(&moves));
            assert!(b.swap(m.a, m.b));
            sh.swap(m.a, m.b);
            settle(&mut b, &mut sh);
        }
        while b.fire_powers() {
            settle(&mut b, &mut sh);
        }
    }
}

/// The bot can actually finish every level some of the time.
#[test]
fn every_level_is_winnable() {
    // On Hard, the tightest budgets, a careful player still has to get through.
    for level in 0..LEVELS.len() {
        let hard = Some(LEVELS[level].moves[Difficulty::Hard.index()]);
        let won = (0..30).any(|seed| bot::play_with_moves(level, seed, &BotCfg::CAREFUL, hard).won);
        assert!(won, "level {} was never won in 30 careful attempts on Hard", level + 1);
    }
}

#[test]
fn easier_settings_never_give_fewer_moves() {
    for (i, def) in LEVELS.iter().enumerate() {
        let [easy, normal, hard] = def.moves;
        assert!(easy >= normal && normal >= hard && hard >= 3, "level {} budgets out of order: {:?}", i + 1, def.moves);
        for d in Difficulty::ALL {
            assert_eq!(Session::with(i, 1, d).moves_left, def.moves[d.index()]);
        }
    }
}

#[test]
fn the_pitiful_look_works_once_and_only_when_close() {
    // Level 1 is a plain score goal, which makes "nearly done" easy to stage.
    let Goal::Score(need) = LEVELS[0].goals[0] else { panic!("level 1 should be a score goal") };
    let mut s = Session::new(0, 7);
    s.board.settle();

    assert!(!s.can_beg(), "not while there are moves left");
    s.moves_left = 0;
    s.board.score = need / 4;
    assert!(!s.can_beg() && !s.beg(), "not when the level is nowhere near done");
    assert_eq!(s.moves_left, 0);

    s.board.score = need * 9 / 10;
    assert!(s.can_beg());
    assert!(s.beg());
    assert_eq!(s.moves_left, level::BEG_MOVES);
    assert!(s.begged());

    s.moves_left = 0;
    assert!(!s.can_beg() && !s.beg(), "only once per attempt");

    let mut won = Session::new(0, 7);
    won.board.settle();
    won.moves_left = 0;
    won.board.score = need;
    assert!(won.won() && !won.can_beg(), "not after winning");
}

#[test]
fn zoomies_come_before_the_pitiful_look() {
    let Goal::Score(need) = LEVELS[0].goals[0] else { panic!("level 1 should be a score goal") };
    // Play until the meter fills, then stage being out of moves and nearly done.
    let mut found = false;
    for seed in 0..40 {
        let mut s = Session::new(0, seed);
        s.board.settle();
        let mut rng = Rng::new(seed);
        s.moves_left = 200;
        while !s.board.zoomies_ready() && s.moves_left > 100 {
            let Some(m) = bot::choose(&s, &mut rng, &BotCfg::CAREFUL) else { break };
            s.swap(m.a, m.b);
            s.board.settle();
        }
        if !s.board.zoomies_ready() {
            continue;
        }
        found = true;
        s.moves_left = 0;
        s.board.score = need * 9 / 10;
        assert!(!s.can_beg(), "a full meter has to be spent first");
        s.board.zoomies();
        s.board.settle();
        s.board.score = need * 9 / 10;
        assert!(s.can_beg(), "and then the look works");
    }
    assert!(found, "the meter never filled in 40 games");
}

#[test]
fn no_board_uses_more_kinds_than_it_can_hold() {
    for (i, def) in LEVELS.iter().enumerate() {
        assert!(def.kinds.len() <= MAX_IN_PLAY, "level {} has too many kinds in play", i + 1);
        let mut seen = def.kinds.to_vec();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), def.kinds.len(), "level {} repeats a kind", i + 1);
    }
}

#[test]
fn the_chapters_cover_the_levels() {
    assert_eq!(LEVELS.len(), level::HAND_MADE + 1000, "the first day plus a thousand more");
    assert_eq!(level::chapter_count(), CHAPTERS.len(), "every chapter has a name and every name has levels");
    let mut names: Vec<&str> = CHAPTERS.iter().map(|c| c.name).collect();
    assert!(names.iter().all(|n| !n.is_empty()));
    names.sort();
    names.dedup();
    assert_eq!(names.len(), CHAPTERS.len(), "two chapters share a name");
    // every chapter is a full six, except the very last, which is bedtime
    for c in 0..level::chapter_count() {
        let n = level::chapter_levels(c).len();
        assert!(n == CHAPTER_LEN || (c + 1 == level::chapter_count() && n > 0), "chapter {} has {n} levels", c + 1);
    }
    // each day visits the six places in order
    for (c, chapter) in CHAPTERS.iter().enumerate().take(CHAPTERS.len() - 1) {
        assert_eq!(chapter.place, Place::ALL[c % 6], "chapter {} is out of place", c + 1);
    }
}

#[test]
fn no_two_levels_share_a_title() {
    let mut titles: Vec<&str> = LEVELS.iter().map(|l| l.title).collect();
    assert!(LEVELS.iter().all(|l| !l.title.is_empty() && !l.tip.is_empty()));
    titles.sort();
    let before = titles.len();
    titles.dedup();
    assert_eq!(titles.len(), before);
}

#[test]
fn the_designed_levels_keep_to_the_house_rules() {
    for (i, def) in LEVELS.iter().enumerate().skip(level::HAND_MADE) {
        let tiles = |x: usize| def.rows.iter().filter(|r| r.as_bytes()[x] != b' ').count();
        // bones are handed over at the bottom of a column: a short column would give them away
        if def.goals.iter().any(|g| matches!(g, Goal::Bones(_))) {
            assert!((0..COLS).all(|x| tiles(x) >= 6), "level {} asks for bones on a board with a short column", i + 1);
        }
        assert!(def.goals.len() <= 3, "level {} has more goals than the designer deals out", i + 1);
        assert!(def.moves[0] <= 60 && def.moves[2] >= 9, "level {} has a move budget out of range: {:?}", i + 1, def.moves);
        // the same board twice within a few levels reads as a repeat
        for back in 1..=4 {
            if i >= level::HAND_MADE + back {
                assert!(def.rows != LEVELS[i - back].rows, "level {} repeats the board of level {}", i + 1, i + 1 - back);
            }
        }
    }
}
