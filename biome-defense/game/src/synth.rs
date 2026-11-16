//! The synthesiser: every sound in the game is a few oscillators and
//! envelopes rendered to a WAV in memory at startup. There are no audio
//! files to ship or cache. Pure Rust with no dependencies.

use std::f32::consts::{PI, TAU};

const SR: f32 = 22_050.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Click,
    Error,
    Build,
    Upgrade,
    Sell,
    /// An invader shooed; the number climbs a scale as they go in a run.
    Shoo(u8),
    Dart,
    Lob,
    Zap,
    Peck,
    Tongue,
    Burst,
    Leak,
    Wave,
    Boss,
    Pounce,
    Slam,
    Gust,
    Thump,
    Storm,
    Dam,
    Shell,
    Whistle,
    Ko,
    Up,
    Win,
    Lose,
    Star(u8),
    Jam,
    Rattle,
    Root,
    Blip,
    Join,
    Learn,
    Step,
}

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Tri,
    Saw,
    Square,
    /// Sine with a couple of harmonics: a soft plucked/mallet tone.
    Pluck,
}

fn osc(wave: Wave, phase: f32) -> f32 {
    let p = phase.fract();
    match wave {
        Wave::Sine => (p * TAU).sin(),
        Wave::Tri => 1.0 - 4.0 * (p - 0.5).abs(),
        Wave::Saw => 2.0 * p - 1.0,
        Wave::Square => {
            if p < 0.5 {
                0.7
            } else {
                -0.7
            }
        }
        Wave::Pluck => 0.72 * (p * TAU).sin() + 0.24 * (p * 2.0 * TAU).sin() + 0.1 * (p * 3.0 * TAU).sin(),
    }
}

/// Quick attack, exponential fall, guaranteed silent by the end.
fn pluck_env(t: f32, dur: f32, attack: f32) -> f32 {
    let a = (t / attack).min(1.0);
    let d = (-t * 5.0 / dur).exp();
    let tail = ((dur - t) / 0.006).clamp(0.0, 1.0);
    a * d * tail
}

/// Attack, hold, release.
fn hold_env(t: f32, dur: f32, attack: f32, release: f32) -> f32 {
    (t / attack).min(1.0) * ((dur - t) / release).clamp(0.0, 1.0)
}

fn midi(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

struct Buf {
    s: Vec<f32>,
    /// Notes running off the end wrap to the start (for the music loop).
    wrap: bool,
}

impl Buf {
    fn new(secs: f32) -> Buf {
        Buf { s: vec![0.0; (secs * SR) as usize], wrap: false }
    }

    fn add(&mut self, i: usize, v: f32) {
        let n = self.s.len();
        if i < n {
            self.s[i] += v;
        } else if self.wrap {
            self.s[i % n] += v;
        }
    }

    /// Adds an oscillator from `at` for `dur` seconds. `freq` and `amp` are
    /// functions of time since the note started.
    fn tone(&mut self, at: f32, dur: f32, wave: Wave, freq: impl Fn(f32) -> f32, amp: impl Fn(f32) -> f32) {
        let start = (at * SR) as usize;
        let mut phase = 0.0f32;
        for k in 0..(dur * SR) as usize {
            let t = k as f32 / SR;
            phase += freq(t) / SR;
            self.add(start + k, osc(wave, phase) * amp(t));
        }
    }

    fn noise(&mut self, at: f32, dur: f32, seed: u32, amp: impl Fn(f32) -> f32) {
        let start = (at * SR) as usize;
        let mut x = seed.wrapping_mul(2_654_435_761).max(1);
        for k in 0..(dur * SR) as usize {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            let v = (x >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
            self.add(start + k, v * amp(k as f32 / SR));
        }
    }

    fn lowpass(&mut self, cutoff: f32) {
        let a = 1.0 - (-TAU * cutoff / SR).exp();
        let mut y = 0.0;
        for v in &mut self.s {
            y += a * (*v - y);
            *v = y;
        }
    }

    fn highpass(&mut self, cutoff: f32) {
        let a = 1.0 - (-TAU * cutoff / SR).exp();
        let mut y = 0.0;
        for v in &mut self.s {
            y += a * (*v - y);
            *v -= y;
        }
    }

    fn wav(mut self, peak: f32) -> Vec<u8> {
        let max = self.s.iter().fold(0.0f32, |m, v| m.max(v.abs())).max(1e-6);
        let k = peak / max;
        // a few ms of fade at each end so nothing clicks
        let n = self.s.len();
        let fade = ((0.004 * SR) as usize).min(n / 2);
        if !self.wrap {
            for i in 0..fade {
                let g = i as f32 / fade as f32;
                self.s[i] *= g;
                self.s[n - 1 - i] *= g;
            }
        }
        let mut out = Vec::with_capacity(44 + n * 2);
        let sr = SR as u32;
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + n as u32 * 2).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&sr.to_le_bytes());
        out.extend_from_slice(&(sr * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(n as u32 * 2).to_le_bytes());
        for v in &self.s {
            out.extend_from_slice(&(((v * k).clamp(-1.0, 1.0) * 32_000.0) as i16).to_le_bytes());
        }
        out
    }
}

// ---- the sounds ---------------------------------------------------------------

/// Major pentatonic steps, so any run of pops sounds like a tune.
const SCALE: [f32; 8] = [0.0, 2.0, 4.0, 7.0, 9.0, 12.0, 14.0, 16.0];

fn bell(b: &mut Buf, at: f32, freq: f32, dur: f32, amp: f32) {
    b.tone(at, dur, Wave::Sine, |_| freq, |t| amp * pluck_env(t, dur, 0.002));
    b.tone(at, dur * 0.6, Wave::Sine, |_| freq * 2.76, |t| amp * 0.28 * pluck_env(t, dur * 0.6, 0.002));
    b.tone(at, dur * 0.35, Wave::Sine, |_| freq * 5.4, |t| amp * 0.12 * pluck_env(t, dur * 0.35, 0.001));
}

/// A knock on wood.
fn knock(b: &mut Buf, at: f32, freq: f32, amp: f32) {
    b.tone(at, 0.07, Wave::Tri, move |t| freq * (1.0 + 0.6 * (-t * 60.0).exp()), move |t| amp * pluck_env(t, 0.07, 0.001));
    b.noise(at, 0.02, 17, move |t| amp * 0.5 * (1.0 - t / 0.02));
}

fn whoosh(b: &mut Buf, at: f32, dur: f32, seed: u32, amp: f32) {
    b.noise(at, dur, seed, move |t| amp * (PI * t / dur).sin().powi(2));
}

pub fn render(s: Sfx) -> Vec<u8> {
    let peak;
    let b = match s {
        Sfx::Click => {
            peak = 0.35;
            let mut b = Buf::new(0.05);
            b.tone(0.0, 0.045, Wave::Sine, |t| 1150.0 - 6000.0 * t, |t| pluck_env(t, 0.045, 0.001));
            b
        }
        Sfx::Error => {
            peak = 0.5;
            let mut b = Buf::new(0.26);
            b.tone(0.0, 0.11, Wave::Tri, |_| 196.0, |t| pluck_env(t, 0.11, 0.004));
            b.tone(0.1, 0.15, Wave::Tri, |_| 139.0, |t| pluck_env(t, 0.15, 0.004));
            b
        }
        Sfx::Build => {
            // earth patted down, then a little rising pluck
            peak = 0.6;
            let mut b = Buf::new(0.42);
            b.tone(0.0, 0.12, Wave::Sine, |t| 160.0 - 700.0 * t, |t| pluck_env(t, 0.12, 0.002));
            b.noise(0.0, 0.08, 5, |t| 0.4 * pluck_env(t, 0.08, 0.002));
            b.lowpass(1800.0);
            b.tone(0.09, 0.14, Wave::Pluck, |_| midi(67.0), |t| 0.7 * pluck_env(t, 0.14, 0.003));
            b.tone(0.17, 0.22, Wave::Pluck, |_| midi(74.0), |t| 0.7 * pluck_env(t, 0.22, 0.003));
            b
        }
        Sfx::Upgrade => {
            peak = 0.5;
            let mut b = Buf::new(0.6);
            for (i, n) in [72.0, 76.0, 79.0, 84.0].iter().enumerate() {
                bell(&mut b, i as f32 * 0.06, midi(*n), 0.35, 0.8);
            }
            b
        }
        Sfx::Sell => {
            peak = 0.45;
            let mut b = Buf::new(0.4);
            for (i, n) in [84.0, 79.0, 76.0].iter().enumerate() {
                bell(&mut b, i as f32 * 0.07, midi(*n), 0.25, 0.8);
            }
            b
        }
        Sfx::Shoo(i) => {
            peak = 0.7;
            let f = 523.25 * 2f32.powf(SCALE[(i as usize).min(7)] / 12.0);
            let mut b = Buf::new(0.2);
            b.tone(0.0, 0.19, Wave::Pluck, |_| f, |t| pluck_env(t, 0.19, 0.003));
            b.tone(0.0, 0.05, Wave::Sine, |_| f * 3.0, |t| 0.3 * pluck_env(t, 0.05, 0.001));
            b.noise(0.0, 0.05, 9, |t| 0.18 * (1.0 - t / 0.05));
            b
        }
        Sfx::Dart => {
            peak = 0.28;
            let mut b = Buf::new(0.09);
            b.tone(0.0, 0.08, Wave::Sine, |t| 900.0 + 9000.0 * t, |t| pluck_env(t, 0.08, 0.004));
            b.noise(0.0, 0.06, 3, |t| 0.3 * pluck_env(t, 0.06, 0.004));
            b.highpass(700.0);
            b
        }
        Sfx::Lob => {
            peak = 0.4;
            let mut b = Buf::new(0.2);
            b.tone(0.0, 0.18, Wave::Sine, |t| 190.0 + 900.0 * t, |t| hold_env(t, 0.18, 0.01, 0.1));
            b.noise(0.0, 0.1, 13, |t| 0.3 * pluck_env(t, 0.1, 0.01));
            b.lowpass(1500.0);
            b
        }
        Sfx::Zap => {
            peak = 0.4;
            let mut b = Buf::new(0.2);
            b.tone(0.0, 0.18, Wave::Saw, |t| 240.0 + 1800.0 * (t * 97.0).sin().abs(), |t| pluck_env(t, 0.18, 0.002));
            b.noise(0.0, 0.14, 29, |t| 0.5 * pluck_env(t, 0.14, 0.002));
            b.highpass(900.0);
            b
        }
        Sfx::Peck => {
            peak = 0.5;
            let mut b = Buf::new(0.14);
            knock(&mut b, 0.0, 720.0, 1.0);
            knock(&mut b, 0.055, 760.0, 0.7);
            b
        }
        Sfx::Tongue => {
            peak = 0.45;
            let mut b = Buf::new(0.16);
            b.tone(0.0, 0.07, Wave::Sine, |t| 300.0 + 7000.0 * t, |t| hold_env(t, 0.07, 0.004, 0.02));
            b.tone(0.08, 0.07, Wave::Sine, |t| 800.0 - 6000.0 * t, |t| 0.8 * pluck_env(t, 0.07, 0.003));
            b
        }
        Sfx::Burst => {
            peak = 0.5;
            let mut b = Buf::new(0.24);
            b.noise(0.0, 0.2, 31, |t| pluck_env(t, 0.2, 0.003));
            b.lowpass(900.0);
            b.tone(0.0, 0.16, Wave::Sine, |t| 220.0 - 800.0 * t, |t| 0.8 * pluck_env(t, 0.16, 0.003));
            b
        }
        Sfx::Leak => {
            // something got in: a thud, and a note falling
            peak = 0.6;
            let mut b = Buf::new(0.5);
            b.tone(0.0, 0.2, Wave::Sine, |t| 120.0 - 350.0 * t, |t| pluck_env(t, 0.2, 0.003));
            b.tone(0.06, 0.4, Wave::Tri, |t| midi(64.0) * (1.0 - 0.3 * t), |t| 0.5 * hold_env(t, 0.4, 0.02, 0.25));
            b
        }
        Sfx::Wave => {
            // a horn call: here they come
            peak = 0.5;
            let mut b = Buf::new(0.9);
            for (at, dur, n) in [(0.0, 0.22, 62.0), (0.24, 0.6, 69.0)] {
                let f = move |t: f32| midi(n) * (1.0 + 0.006 * (t * TAU * 5.5).sin());
                b.tone(at, dur, Wave::Saw, f, move |t| 0.5 * hold_env(t, dur, 0.04, 0.18));
                b.tone(at, dur, Wave::Tri, move |t| f(t) * 0.5, move |t| 0.5 * hold_env(t, dur, 0.04, 0.18));
            }
            b.lowpass(1500.0);
            b
        }
        Sfx::Boss => {
            peak = 0.7;
            let mut b = Buf::new(1.8);
            for (at, dur, n) in [(0.0, 0.5, 45.0), (0.5, 0.5, 44.0), (1.0, 0.75, 41.0)] {
                b.tone(at, dur, Wave::Saw, move |_| midi(n), move |t| 0.6 * hold_env(t, dur, 0.04, 0.2));
                b.tone(at, dur, Wave::Square, move |_| midi(n - 12.0), move |t| 0.5 * hold_env(t, dur, 0.04, 0.2));
                b.tone(at, 0.2, Wave::Sine, |t| 50.0 + 60.0 * (-t * 20.0).exp(), |t| pluck_env(t, 0.2, 0.003));
            }
            b.lowpass(900.0);
            b
        }
        Sfx::Pounce => {
            peak = 0.5;
            let mut b = Buf::new(0.4);
            whoosh(&mut b, 0.0, 0.22, 11, 0.6);
            b.highpass(700.0);
            for at in [0.2, 0.29] {
                b.tone(at, 0.07, Wave::Sine, |t| 800.0 + 6000.0 * t, |t| hold_env(t, 0.07, 0.006, 0.03));
                b.tone(at, 0.07, Wave::Saw, |t| 800.0 + 6000.0 * t, |t| 0.2 * hold_env(t, 0.07, 0.006, 0.03));
            }
            b
        }
        Sfx::Slam => {
            peak = 0.95;
            let mut b = Buf::new(0.7);
            b.noise(0.0, 0.3, 5, |t| 0.7 * pluck_env(t, 0.3, 0.002));
            b.lowpass(800.0);
            b.tone(0.0, 0.6, Wave::Sine, |t| 42.0 + 90.0 * (-t * 9.0).exp(), |t| pluck_env(t, 0.6, 0.003));
            b
        }
        Sfx::Gust => {
            peak = 0.55;
            let mut b = Buf::new(0.8);
            whoosh(&mut b, 0.0, 0.7, 23, 0.8);
            b.highpass(500.0);
            b.lowpass(3500.0);
            b.tone(0.0, 0.6, Wave::Sine, |t| 300.0 + 500.0 * (PI * t / 0.6).sin(), |t| 0.2 * hold_env(t, 0.6, 0.1, 0.3));
            b
        }
        Sfx::Thump => {
            peak = 0.8;
            let mut b = Buf::new(0.4);
            for at in [0.0, 0.12] {
                b.tone(at, 0.2, Wave::Sine, |t| 70.0 + 120.0 * (-t * 25.0).exp(), |t| pluck_env(t, 0.2, 0.002));
                b.noise(at, 0.05, 7, |t| 0.3 * (1.0 - t / 0.05));
            }
            b.lowpass(1200.0);
            b
        }
        Sfx::Storm => {
            // a patter of acorns
            peak = 0.5;
            let mut b = Buf::new(0.7);
            for k in 0..9 {
                let at = 0.03 + k as f32 * 0.065 + (k % 3) as f32 * 0.012;
                knock(&mut b, at, 520.0 + 90.0 * ((k * 5) % 7) as f32, 0.8);
            }
            b
        }
        Sfx::Dam => {
            peak = 0.6;
            let mut b = Buf::new(0.45);
            for (at, f) in [(0.0, 180.0), (0.1, 150.0), (0.21, 205.0)] {
                knock(&mut b, at, f, 1.0);
                b.tone(at, 0.12, Wave::Sine, move |_| f * 0.5, |t| 0.6 * pluck_env(t, 0.12, 0.003));
            }
            b
        }
        Sfx::Shell => {
            peak = 0.6;
            let mut b = Buf::new(0.35);
            b.tone(0.0, 0.3, Wave::Tri, |t| 260.0 * (1.0 + 0.4 * (-t * 30.0).exp()), |t| pluck_env(t, 0.3, 0.002));
            b.tone(0.0, 0.2, Wave::Sine, |_| 655.0, |t| 0.35 * pluck_env(t, 0.2, 0.002));
            b
        }
        Sfx::Whistle => {
            peak = 0.4;
            let mut b = Buf::new(0.6);
            let f = |t: f32| (2100.0 + 900.0 * (t / 0.5).min(1.0)) * (1.0 + 0.012 * (t * TAU * 22.0).sin());
            b.tone(0.0, 0.52, Wave::Sine, f, |t| hold_env(t, 0.52, 0.03, 0.12));
            b.tone(0.0, 0.52, Wave::Sine, move |t| f(t) * 2.0, |t| 0.12 * hold_env(t, 0.52, 0.03, 0.12));
            b
        }
        Sfx::Ko => {
            peak = 0.4;
            let mut b = Buf::new(0.6);
            let f = |t: f32| (760.0 - 520.0 * t) * (1.0 + 0.02 * (t * TAU * 7.0).sin());
            b.tone(0.0, 0.55, Wave::Sine, f, |t| hold_env(t, 0.55, 0.05, 0.3));
            b.tone(0.0, 0.55, Wave::Tri, move |t| f(t) * 2.0, |t| 0.18 * hold_env(t, 0.55, 0.05, 0.3));
            b
        }
        Sfx::Up => {
            peak = 0.45;
            let mut b = Buf::new(0.5);
            bell(&mut b, 0.0, midi(79.0), 0.3, 0.8);
            bell(&mut b, 0.09, midi(86.0), 0.4, 0.9);
            b
        }
        Sfx::Win => {
            peak = 0.6;
            let mut b = Buf::new(2.2);
            for (i, n) in [69.0, 74.0, 78.0, 81.0].iter().enumerate() {
                b.tone(i as f32 * 0.115, 0.3, Wave::Pluck, |_| midi(*n), |t| pluck_env(t, 0.3, 0.004));
            }
            for (n, amp) in [(74.0, 0.6), (78.0, 0.55), (81.0, 0.55), (86.0, 0.7), (90.0, 0.35)] {
                b.tone(0.52, 1.6, Wave::Pluck, |_| midi(n), move |t| amp * pluck_env(t, 1.6, 0.006));
                b.tone(0.52, 1.6, Wave::Tri, |t| midi(n) * (1.0 + 0.004 * (t * 36.0).sin()), move |t| amp * 0.3 * hold_env(t, 1.6, 0.05, 1.1));
            }
            bell(&mut b, 0.56, midi(98.0), 0.9, 0.3);
            bell(&mut b, 0.74, midi(102.0), 0.9, 0.25);
            b
        }
        Sfx::Lose => {
            peak = 0.5;
            let mut b = Buf::new(2.0);
            for (at, dur, n) in [(0.0, 0.34, 62.0), (0.38, 0.34, 60.0), (0.76, 1.1, 57.0)] {
                let long = dur > 0.5;
                let f = move |t: f32| midi(n) * if long { 1.0 + 0.01 * (t * TAU * 5.0).sin() * (t / dur) - 0.04 * (t / dur).powi(3) } else { 1.0 };
                let env = move |t: f32| hold_env(t, dur, 0.05, if long { 0.6 } else { 0.08 });
                b.tone(at, dur, Wave::Tri, f, move |t| 0.8 * env(t));
                b.tone(at, dur, Wave::Sine, move |t| f(t) * 0.5, move |t| 0.6 * env(t));
            }
            b.lowpass(1400.0);
            b
        }
        Sfx::Star(i) => {
            peak = 0.55;
            let f = 1046.5 * [1.0, 1.26, 1.5][(i as usize).min(2)];
            let mut b = Buf::new(0.7);
            bell(&mut b, 0.0, f, 0.6, 1.0);
            bell(&mut b, 0.05, f * 2.0, 0.5, 0.4);
            b
        }
        Sfx::Jam => {
            peak = 0.4;
            let mut b = Buf::new(0.5);
            b.noise(0.0, 0.45, 41, |t| hold_env(t, 0.45, 0.03, 0.3));
            b.highpass(2200.0);
            b
        }
        Sfx::Rattle => {
            peak = 0.45;
            let mut b = Buf::new(0.5);
            b.noise(0.0, 0.45, 43, |t| hold_env(t, 0.45, 0.02, 0.2) * (0.5 + 0.5 * (t * TAU * 38.0).sin().signum()));
            b.highpass(2600.0);
            b
        }
        Sfx::Root => {
            peak = 0.45;
            let mut b = Buf::new(0.3);
            b.noise(0.0, 0.25, 47, |t| pluck_env(t, 0.25, 0.004) * (0.55 + 0.45 * (t * TAU * 70.0).sin().signum()));
            b.highpass(1200.0);
            b.tone(0.0, 0.2, Wave::Sine, |t| 1400.0 - 2500.0 * t, |t| 0.3 * pluck_env(t, 0.2, 0.003));
            b
        }
        Sfx::Blip => {
            peak = 0.22;
            let mut b = Buf::new(0.05);
            b.tone(0.0, 0.04, Wave::Tri, |_| 620.0, |t| pluck_env(t, 0.04, 0.002));
            b
        }
        Sfx::Join => {
            peak = 0.55;
            let mut b = Buf::new(1.4);
            for (i, n) in [62.0, 66.0, 69.0, 74.0, 78.0, 81.0].iter().enumerate() {
                bell(&mut b, i as f32 * 0.09, midi(*n + 12.0), 0.7, 0.8);
                b.tone(i as f32 * 0.09, 0.4, Wave::Pluck, |_| midi(*n), |t| 0.5 * pluck_env(t, 0.4, 0.004));
            }
            b
        }
        Sfx::Learn => {
            peak = 0.5;
            let mut b = Buf::new(0.9);
            for (n, at) in [(74.0, 0.0), (81.0, 0.07), (86.0, 0.14)] {
                bell(&mut b, at, midi(n), 0.7, 0.8);
            }
            b.tone(0.0, 0.5, Wave::Sine, |t| 500.0 + 2400.0 * t, |t| 0.12 * hold_env(t, 0.5, 0.05, 0.3));
            b
        }
        Sfx::Step => {
            peak = 0.3;
            let mut b = Buf::new(0.1);
            b.tone(0.0, 0.08, Wave::Sine, |t| 240.0 - 900.0 * t, |t| pluck_env(t, 0.08, 0.002));
            b.noise(0.0, 0.05, 53, |t| 0.4 * pluck_env(t, 0.05, 0.003));
            b.lowpass(1600.0);
            b
        }
    };
    b.wav(peak)
}

// ---- music ----------------------------------------------------------------------

/// A whistled or plucked tune: (bar, start in eighths, length in eighths, note).
type Tune = [(u8, u8, u8, f32)];

fn lead(b: &mut Buf, tune: &Tune, bar: f32, eighth: f32, amp: f32) {
    for &(bar_i, start, len, note) in tune {
        let at = bar_i as f32 * bar + start as f32 * eighth;
        let dur = len as f32 * eighth * 0.92;
        let f = move |t: f32| midi(note) * (1.0 + 0.005 * (t * TAU * 5.5).sin() * (t / dur).min(1.0));
        let env = move |t: f32| hold_env(t, dur, 0.025, 0.07) * (0.75 + 0.25 * (-t * 6.0).exp());
        b.tone(at, dur, Wave::Sine, f, move |t| amp * env(t));
        b.tone(at, dur, Wave::Sine, move |t| f(t) * 2.0, move |t| amp * 0.17 * env(t));
        b.tone(at, dur, Wave::Tri, f, move |t| amp * 0.2 * env(t));
    }
}

/// The trail: an easy walking tune in D for the map and the quiet screens.
pub fn trail() -> Vec<u8> {
    let beat = 0.62;
    let bar = beat * 4.0;
    let mut b = Buf::new(bar * 8.0);
    b.wrap = true;
    const D: (f32, [f32; 3]) = (50.0, [62.0, 66.0, 69.0]);
    const G: (f32, [f32; 3]) = (43.0, [62.0, 67.0, 71.0]);
    const A: (f32, [f32; 3]) = (45.0, [61.0, 64.0, 69.0]);
    const BM: (f32, [f32; 3]) = (47.0, [62.0, 66.0, 71.0]);
    let bars = [D, D, G, A, BM, G, A, D];
    for (i, (root, triad)) in bars.iter().enumerate() {
        let t0 = i as f32 * bar;
        // bass: root on one, the fifth on three
        b.tone(t0, beat * 1.6, Wave::Tri, |_| midi(*root), |t| 0.42 * pluck_env(t, beat * 1.6, 0.008));
        b.tone(t0 + beat * 2.0, beat * 1.4, Wave::Tri, |_| midi(*root + 7.0), |t| 0.3 * pluck_env(t, beat * 1.4, 0.008));
        // a kalimba picking through the chord in eighths
        for e in 0..8 {
            let n = triad[[0, 1, 2, 1, 0, 2, 1, 2][e]] + if e == 5 { 12.0 } else { 0.0 };
            b.tone(t0 + e as f32 * beat * 0.5 + 0.004, 0.42, Wave::Pluck, |_| midi(n + 12.0), |t| 0.085 * pluck_env(t, 0.42, 0.003));
        }
        // a soft shaker on the off beats
        for e in [1, 3, 5, 7] {
            b.noise(t0 + e as f32 * beat * 0.5, 0.035, 60 + (i * 8 + e) as u32, |t| 0.035 * (1.0 - t / 0.035));
        }
    }
    let tune: [(u8, u8, u8, f32); 30] = [
        (0, 0, 3, 78.0), (0, 3, 1, 76.0), (0, 4, 2, 74.0), (0, 6, 2, 69.0),
        (1, 0, 2, 74.0), (1, 2, 2, 76.0), (1, 4, 4, 78.0),
        (2, 0, 3, 79.0), (2, 3, 1, 78.0), (2, 4, 2, 76.0), (2, 6, 2, 74.0),
        (3, 0, 2, 76.0), (3, 2, 2, 73.0), (3, 4, 4, 69.0),
        (4, 0, 3, 78.0), (4, 3, 1, 81.0), (4, 4, 2, 83.0), (4, 6, 2, 81.0),
        (5, 0, 2, 79.0), (5, 2, 2, 78.0), (5, 4, 2, 74.0), (5, 6, 2, 71.0),
        (6, 0, 2, 73.0), (6, 2, 2, 76.0), (6, 4, 2, 81.0), (6, 6, 2, 76.0),
        (7, 0, 2, 78.0), (7, 2, 2, 74.0), (7, 4, 3, 74.0), (7, 7, 1, 69.0),
    ];
    lead(&mut b, &tune, bar, beat * 0.5, 0.2);
    b.wav(0.75)
}

/// The defence: a brisk march in A minor for the stages.
pub fn defend() -> Vec<u8> {
    let beat = 0.46;
    let bar = beat * 4.0;
    let mut b = Buf::new(bar * 8.0);
    b.wrap = true;
    const AM: (f32, [f32; 3]) = (45.0, [57.0, 60.0, 64.0]);
    const F: (f32, [f32; 3]) = (41.0, [57.0, 60.0, 65.0]);
    const G: (f32, [f32; 3]) = (43.0, [59.0, 62.0, 67.0]);
    const C: (f32, [f32; 3]) = (48.0, [60.0, 64.0, 67.0]);
    const E: (f32, [f32; 3]) = (40.0, [56.0, 59.0, 64.0]);
    let bars = [AM, AM, F, G, AM, C, F, E];
    for (i, (root, triad)) in bars.iter().enumerate() {
        let t0 = i as f32 * bar;
        for q in 0..4 {
            let at = t0 + q as f32 * beat;
            // marching bass: root, fifth, root, octave
            let n = *root + [0.0, 7.0, 0.0, 12.0][q];
            b.tone(at, beat * 0.8, Wave::Tri, |_| midi(n), |t| 0.42 * pluck_env(t, beat * 0.8, 0.006));
            // drum: low on one and three, a rattle on two and four
            if q % 2 == 0 {
                b.tone(at, 0.12, Wave::Sine, |t| 52.0 + 80.0 * (-t * 40.0).exp(), |t| 0.55 * pluck_env(t, 0.12, 0.002));
            } else {
                b.noise(at, 0.07, 80 + (i * 4 + q) as u32, |t| 0.11 * pluck_env(t, 0.07, 0.002));
                // chord stab with the rattle
                for (k, c) in triad.iter().enumerate() {
                    b.tone(at + k as f32 * 0.01, 0.22, Wave::Pluck, |_| midi(*c + 12.0), |t| 0.11 * pluck_env(t, 0.22, 0.003));
                }
            }
            // ticking eighths
            for e in 0..2 {
                b.noise(at + e as f32 * beat * 0.5, 0.025, 120 + (i * 8 + q * 2 + e) as u32, |t| if e == 1 { 0.05 } else { 0.028 } * (1.0 - t / 0.025));
            }
        }
    }
    let tune: [(u8, u8, u8, f32); 31] = [
        (0, 0, 2, 69.0), (0, 2, 1, 72.0), (0, 3, 1, 76.0), (0, 4, 3, 76.0), (0, 7, 1, 74.0),
        (1, 0, 2, 72.0), (1, 2, 2, 74.0), (1, 4, 4, 69.0),
        (2, 0, 2, 72.0), (2, 2, 1, 77.0), (2, 3, 1, 81.0), (2, 4, 3, 81.0), (2, 7, 1, 79.0),
        (3, 0, 2, 79.0), (3, 2, 2, 74.0), (3, 4, 4, 71.0),
        (4, 0, 2, 76.0), (4, 2, 1, 76.0), (4, 3, 1, 79.0), (4, 4, 2, 81.0), (4, 6, 2, 84.0),
        (5, 0, 3, 79.0), (5, 3, 1, 76.0), (5, 4, 4, 72.0),
        (6, 0, 2, 77.0), (6, 2, 2, 76.0), (6, 4, 2, 74.0), (6, 6, 2, 72.0),
        (7, 0, 3, 71.0), (7, 3, 1, 68.0), (7, 4, 4, 64.0),
    ];
    lead(&mut b, &tune, bar, beat * 0.5, 0.19);
    b.wav(0.75)
}

pub const ALL: [Sfx; 44] = [
    Sfx::Click, Sfx::Error, Sfx::Build, Sfx::Upgrade, Sfx::Sell,
    Sfx::Shoo(0), Sfx::Shoo(1), Sfx::Shoo(2), Sfx::Shoo(3), Sfx::Shoo(4), Sfx::Shoo(5), Sfx::Shoo(6), Sfx::Shoo(7),
    Sfx::Dart, Sfx::Lob, Sfx::Zap, Sfx::Peck, Sfx::Tongue, Sfx::Burst, Sfx::Leak, Sfx::Wave, Sfx::Boss,
    Sfx::Pounce, Sfx::Slam, Sfx::Gust, Sfx::Thump, Sfx::Storm, Sfx::Dam, Sfx::Shell, Sfx::Whistle,
    Sfx::Ko, Sfx::Up, Sfx::Win, Sfx::Lose, Sfx::Star(0), Sfx::Star(1), Sfx::Star(2),
    Sfx::Jam, Sfx::Rattle, Sfx::Root, Sfx::Blip, Sfx::Join, Sfx::Learn, Sfx::Step,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the samples back out of a rendered WAV.
    fn samples(wav: &[u8]) -> Vec<i16> {
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        wav[44..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
    }

    #[test]
    fn every_sound_is_audible_and_does_not_clip() {
        for s in ALL {
            let data = samples(&render(s));
            let peak = data.iter().map(|v| v.unsigned_abs()).max().unwrap();
            assert!(data.len() > 400, "{s:?} is too short to hear");
            assert!(peak > 3000, "{s:?} is nearly silent (peak {peak})");
            assert!(peak < 32_600, "{s:?} clips");
            // no click at either end
            assert!(data[0].abs() < 600 && data[data.len() - 1].abs() < 600, "{s:?} starts or ends with a click");
        }
    }

    #[test]
    fn both_tunes_loop_without_a_gap() {
        for (name, wav) in [("trail", trail()), ("defend", defend())] {
            let data = samples(&wav);
            let secs = data.len() as f32 / SR;
            assert!(secs > 12.0 && secs < 30.0, "{name} is {secs}s long");
            let peak = data.iter().map(|v| v.unsigned_abs()).max().unwrap();
            assert!(peak > 12_000 && peak < 32_600, "{name} peak {peak}");
            // sound right up to the end, so the loop has no hole in it
            let tail = &data[data.len() - (SR as usize) / 2..];
            assert!(tail.iter().any(|v| v.abs() > 800), "{name} goes quiet before it loops");
            // and no long silence anywhere in the middle
            let window = (SR * 0.8) as usize;
            for chunk in data.chunks(window) {
                assert!(chunk.iter().any(|v| v.abs() > 500), "{name} has a silent stretch");
            }
        }
    }
}
