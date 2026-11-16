//! A two-number vector. Positions in the rules are measured in map tiles:
//! tile (column, row) covers x in `column..column+1`, y in `row..row+1`.

use std::ops::{Add, AddAssign, Mul, Sub};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn dist(self, o: V2) -> f32 {
        (self - o).len()
    }
    /// Unit vector in the same direction (zero stays zero).
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 1e-6 {
            v2(self.x / l, self.y / l)
        } else {
            v2(0.0, 0.0)
        }
    }
    pub fn lerp(self, o: V2, t: f32) -> V2 {
        v2(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }
    /// Turned a quarter turn: the sideways direction of a path.
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
    /// Moves toward `to` by at most `step`. Returns true on arrival.
    pub fn approach(&mut self, to: V2, step: f32) -> bool {
        let d = to - *self;
        let l = d.len();
        if l <= step {
            *self = to;
            true
        } else {
            *self = *self + d * (step / l);
            false
        }
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, k: f32) -> V2 {
        v2(self.x * k, self.y * k)
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        self.x += o.x;
        self.y += o.y;
    }
}
