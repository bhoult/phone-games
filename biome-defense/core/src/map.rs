//! A stage's ground: where the trail runs, where structures can stand, and
//! the routes invaders walk. Maps are drawn as 13 rows of 9 characters:
//!
//! ```text
//!   .  open ground        #  trail            o  a mound to build on
//!   A  B  where invaders come in              H  the home being defended
//!   ~  water              T  something tall   r  a rock
//! ```
//!
//! Water, tall things and rocks are scenery: nothing walks the map except
//! along the trail, and the pack can stand anywhere.

use crate::v2::{v2, V2};

pub const COLS: usize = 9;
pub const ROWS: usize = 13;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Ground,
    Trail,
    Mound,
    Spawn(u8),
    Home,
    Water,
    Tall,
    Rock,
}

/// One way from a spawn to the home, as a smoothed line.
#[derive(Clone, Debug)]
pub struct Route {
    pub spawn: u8,
    pub pts: Vec<V2>,
    /// Distance from the start to each point.
    cum: Vec<f32>,
    pub len: f32,
}

impl Route {
    fn new(spawn: u8, pts: Vec<V2>) -> Route {
        let mut cum = Vec::with_capacity(pts.len());
        let mut len = 0.0;
        for (i, p) in pts.iter().enumerate() {
            if i > 0 {
                len += p.dist(pts[i - 1]);
            }
            cum.push(len);
        }
        Route { spawn, pts, cum, len }
    }

    /// The point `dist` along the route, and the direction of travel there.
    pub fn at(&self, dist: f32) -> (V2, V2) {
        let d = dist.clamp(0.0, self.len);
        let i = match self.cum.binary_search_by(|c| c.partial_cmp(&d).unwrap()) {
            Ok(i) => i.min(self.pts.len() - 2),
            Err(i) => i.saturating_sub(1).min(self.pts.len() - 2),
        };
        let (a, b) = (self.pts[i], self.pts[i + 1]);
        let seg = (self.cum[i + 1] - self.cum[i]).max(1e-6);
        (a.lerp(b, (d - self.cum[i]) / seg), (b - a).norm())
    }

    /// How far along the route its nearest point to `p` is, and how far away.
    pub fn nearest(&self, p: V2) -> (f32, f32) {
        let mut best = (0.0, f32::MAX);
        for i in 0..self.pts.len() - 1 {
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let ab = b - a;
            let l2 = ab.x * ab.x + ab.y * ab.y;
            let t = if l2 > 1e-9 { (((p.x - a.x) * ab.x + (p.y - a.y) * ab.y) / l2).clamp(0.0, 1.0) } else { 0.0 };
            let q = a.lerp(b, t);
            let d = q.dist(p);
            if d < best.1 {
                best = (self.cum[i] + (self.cum[i + 1] - self.cum[i]) * t, d);
            }
        }
        best
    }
}

#[derive(Clone, Debug)]
pub struct Map {
    pub tiles: [[Tile; COLS]; ROWS],
    pub home: V2,
    pub spawns: Vec<V2>,
    /// Centres of the build mounds, in reading order.
    pub mounds: Vec<V2>,
    pub routes: Vec<Route>,
}

fn centre(c: usize, r: usize) -> V2 {
    v2(c as f32 + 0.5, r as f32 + 0.5)
}

impl Map {
    /// Reads a map. Panics on a malformed one: maps are fixed data, and the
    /// tests load every one of them.
    pub fn parse(rows: &[&str; ROWS]) -> Map {
        let mut tiles = [[Tile::Ground; COLS]; ROWS];
        let mut home = None;
        let mut spawns: Vec<(u8, usize, usize)> = Vec::new();
        let mut mounds = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), COLS, "map row {r} is not {COLS} wide: {row:?}");
            for (c, ch) in row.bytes().enumerate() {
                tiles[r][c] = match ch {
                    b'.' => Tile::Ground,
                    b'#' => Tile::Trail,
                    b'o' => {
                        mounds.push(centre(c, r));
                        Tile::Mound
                    }
                    b'A' | b'B' => {
                        spawns.push((ch - b'A', c, r));
                        Tile::Spawn(ch - b'A')
                    }
                    b'H' => {
                        home = Some((c, r));
                        Tile::Home
                    }
                    b'~' => Tile::Water,
                    b'T' => Tile::Tall,
                    b'r' => Tile::Rock,
                    other => panic!("unknown map character {:?}", other as char),
                };
            }
        }
        let (hc, hr) = home.expect("map has no home");
        spawns.sort();
        assert!(!spawns.is_empty(), "map has no spawn");

        // Steps to the home from every walkable tile.
        let walk = |c: usize, r: usize| matches!(tiles[r][c], Tile::Trail | Tile::Spawn(_) | Tile::Home);
        let mut steps = [[u16::MAX; COLS]; ROWS];
        steps[hr][hc] = 0;
        let mut queue = vec![(hc, hr)];
        let mut head = 0;
        while head < queue.len() {
            let (c, r) = queue[head];
            head += 1;
            for (nc, nr) in neighbours(c, r) {
                if walk(nc, nr) && steps[nr][nc] == u16::MAX {
                    steps[nr][nc] = steps[r][c] + 1;
                    queue.push((nc, nr));
                }
            }
        }

        // Every shortest way down from each spawn (forks give more than one).
        let mut routes = Vec::new();
        for &(id, sc, sr) in &spawns {
            assert!(steps[sr][sc] != u16::MAX, "spawn {} cannot reach the home", (b'A' + id) as char);
            let mut found: Vec<Vec<(usize, usize)>> = Vec::new();
            let mut stack = vec![vec![(sc, sr)]];
            while let Some(path) = stack.pop() {
                let (c, r) = *path.last().unwrap();
                if steps[r][c] == 0 {
                    found.push(path);
                    continue;
                }
                for (nc, nr) in neighbours(c, r) {
                    if walk(nc, nr) && steps[nr][nc] + 1 == steps[r][c] && found.len() + stack.len() < 8 {
                        let mut next = path.clone();
                        next.push((nc, nr));
                        stack.push(next);
                    }
                }
            }
            found.sort();
            for path in found {
                let mut pts: Vec<V2> = path.iter().map(|&(c, r)| centre(c, r)).collect();
                // walk in from just off the edge of the map
                let first = pts[0];
                let out = if sr == 0 {
                    v2(0.0, -1.0)
                } else if sr == ROWS - 1 {
                    v2(0.0, 1.0)
                } else if sc == 0 {
                    v2(-1.0, 0.0)
                } else if sc == COLS - 1 {
                    v2(1.0, 0.0)
                } else {
                    v2(0.0, 0.0)
                };
                pts.insert(0, first + out * 1.1);
                routes.push(Route::new(id, round_corners(&pts)));
            }
        }
        Map { tiles, home: centre(hc, hr), spawns: spawns.iter().map(|&(_, c, r)| centre(c, r)).collect(), mounds, routes }
    }

    pub fn tile(&self, c: i32, r: i32) -> Tile {
        if c < 0 || r < 0 || c >= COLS as i32 || r >= ROWS as i32 {
            Tile::Ground
        } else {
            self.tiles[r as usize][c as usize]
        }
    }

    /// The point on any route nearest to `p`: (route, distance along, how far off).
    pub fn nearest_trail(&self, p: V2) -> (usize, f32, f32) {
        let mut best = (0, 0.0, f32::MAX);
        for (i, route) in self.routes.iter().enumerate() {
            let (along, off) = route.nearest(p);
            if off < best.2 {
                best = (i, along, off);
            }
        }
        best
    }

    /// Indices of the routes that start at a spawn.
    pub fn routes_from(&self, spawn: u8) -> Vec<usize> {
        let found: Vec<usize> = self.routes.iter().enumerate().filter(|(_, r)| r.spawn == spawn).map(|(i, _)| i).collect();
        if found.is_empty() {
            vec![0]
        } else {
            found
        }
    }
}

fn neighbours(c: usize, r: usize) -> impl Iterator<Item = (usize, usize)> {
    [(0i32, -1i32), (1, 0), (0, 1), (-1, 0)].into_iter().filter_map(move |(dc, dr)| {
        let (nc, nr) = (c as i32 + dc, r as i32 + dr);
        (nc >= 0 && nr >= 0 && nc < COLS as i32 && nr < ROWS as i32).then_some((nc as usize, nr as usize))
    })
}

/// Cuts each corner twice (Chaikin), so walkers curve round bends instead of
/// turning on the spot. The two ends stay where they are.
fn round_corners(pts: &[V2]) -> Vec<V2> {
    let mut cur = pts.to_vec();
    for _ in 0..2 {
        let mut next = Vec::with_capacity(cur.len() * 2);
        next.push(cur[0]);
        for w in cur.windows(2) {
            next.push(w[0].lerp(w[1], 0.25));
            next.push(w[0].lerp(w[1], 0.75));
        }
        next.push(*cur.last().unwrap());
        cur = next;
    }
    cur
}
