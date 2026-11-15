//! A stand-in player, used to check that levels are winnable and to tune
//! their move counts. It looks one move ahead and is deliberately sloppy.

use crate::board::*;
use crate::level::*;
use crate::rng::Rng;

#[derive(Clone, Copy, Debug)]
pub struct BotCfg {
    /// Chance of just playing any legal move instead of the best-looking one.
    pub sloppiness: f32,
    pub use_zoomies: bool,
}

impl BotCfg {
    /// Roughly a casual player: sees the good move about two times in three.
    pub const CASUAL: BotCfg = BotCfg { sloppiness: 0.35, use_zoomies: true };
    pub const CAREFUL: BotCfg = BotCfg { sloppiness: 0.0, use_zoomies: true };
    /// Mostly just swiping at whatever matches, but does use the zoomies.
    pub const SLOPPY: BotCfg = BotCfg { sloppiness: 0.75, use_zoomies: true };
    pub const RANDOM: BotCfg = BotCfg { sloppiness: 1.0, use_zoomies: false };
}

#[derive(Clone, Copy, Debug)]
pub struct Outcome {
    pub won: bool,
    pub moves_used: u32,
    pub score: u32,
    pub paws: u8,
    pub shuffles: u32,
    /// Whether the pitiful look was used.
    pub begged: bool,
}

/// How close the session is to done, as a number that goes up.
fn progress(s: &Session) -> f32 {
    let mut total = 0.0;
    for g in s.goals() {
        let weight = match g.icon {
            GoalIcon::Score => 1.0,
            _ => 4.0,
        };
        total += weight * (g.have.min(g.need) as f32 / g.need.max(1) as f32);
    }
    // Bones lower on the board are closer to delivered.
    if s.def().goals.iter().any(|g| matches!(g, Goal::Bones(_))) {
        for (pos, p) in s.board.pieces() {
            if p.item == Item::Bone {
                total += 0.25 * pos.y as f32 / ROWS as f32;
            }
        }
    }
    total + s.board.score as f32 / 40_000.0
}

/// The move this player would make. Also what the game uses for hints.
pub fn choose(s: &Session, rng: &mut Rng, cfg: &BotCfg) -> Option<Move> {
    let moves = s.board.find_moves();
    if moves.is_empty() {
        return None;
    }
    if rng.chance(cfg.sloppiness) {
        return Some(rng.pick(&moves));
    }
    let mut best = (f32::MIN, moves[0]);
    for m in moves {
        let mut trial = s.clone();
        // Don't let the bot see which pieces are about to drop in.
        trial.board.rng = Rng::new(rng.next_u64());
        trial.swap(m.a, m.b);
        trial.board.settle();
        let value = progress(&trial) + rng.unit() * 0.01;
        if value > best.0 {
            best = (value, m);
        }
    }
    Some(best.1)
}

pub fn play(level: usize, seed: u64, cfg: &BotCfg) -> Outcome {
    play_with_moves(level, seed, cfg, None)
}

/// Like [`play`], but with a different move budget than the level's own.
pub fn play_with_moves(level: usize, seed: u64, cfg: &BotCfg, moves: Option<u32>) -> Outcome {
    let mut s = Session::new(level, seed);
    if let Some(m) = moves {
        s.moves_left = m;
    }
    let mut rng = Rng::new(seed ^ 0xB07);
    let mut shuffles = 0;
    let mut moves_used = 0;
    let count = |steps: &[Step]| steps.iter().filter(|s| matches!(s, Step::Shuffle(_))).count() as u32;
    shuffles += count(&s.board.settle());
    while !s.won() {
        // Zoomies are free, so a full meter is spent even with no moves left.
        if cfg.use_zoomies && s.board.zoomies_ready() {
            s.board.zoomies();
            shuffles += count(&s.board.settle());
            continue;
        }
        if s.moves_left == 0 {
            // Out of moves: the pitiful look, if it is still to be had.
            if s.beg() {
                continue;
            }
            break;
        }
        let Some(m) = choose(&s, &mut rng, cfg) else { break };
        assert!(s.swap(m.a, m.b), "bot picked an illegal move");
        moves_used += 1;
        shuffles += count(&s.board.settle());
    }
    let won = s.won();
    if won {
        s.finish_bonus();
    }
    Outcome { won, moves_used, score: s.board.score, paws: if won { s.paws() } else { 0 }, shuffles, begged: s.begged() }
}
