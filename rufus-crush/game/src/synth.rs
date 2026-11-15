//! The synthesiser: every sound in the game is a few oscillators and
//! envelopes rendered to a WAV in memory at startup. There are no audio
//! files to ship or cache. Pure Rust with no dependencies, so the sounds can
//! also be rendered natively for inspection (see tools/dump_audio.sh).

use std::f32::consts::{PI, TAU};

const SR: f32 = 22_050.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    /// Match pop; the number climbs a scale as chain reactions build.
    Pop(u8),
    Swap,
    Bonk,
    Land,
    Bark,
    Bark2,
    Squeak,
    Zoom,
    Boom,
    Chime,
    Sparkle,
    Win,
    Lose,
    Paw(u8),
    Click,
    Whine,
    Rip,
    Splat,
    Howl,
    Tick,
    Yip,
    Bone,
    /// Something large and enthusiastic entering a lake.
    Splash,
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

    fn drive(&mut self, gain: f32) {
        for v in &mut self.s {
            *v = (*v * gain).tanh();
        }
    }

    fn mix(&mut self, other: &Buf, at: f32, gain: f32) {
        let start = (at * SR) as usize;
        for (k, v) in other.s.iter().enumerate() {
            self.add(start + k, v * gain);
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

fn one_bark(pitch: f32) -> Buf {
    let mut b = Buf::new(0.26);
    let f = move |t: f32| pitch * (225.0 + 300.0 * (-t * 20.0).exp());
    let env = |t: f32| {
        let a = (t / 0.008).min(1.0);
        let d = if t < 0.07 { 1.0 } else { (-(t - 0.07) * 21.0).exp() };
        let growl = 1.0 - 0.28 * (0.5 + 0.5 * (t * TAU * 68.0).sin());
        a * d * growl
    };
    b.tone(0.0, 0.25, Wave::Saw, f, |t| 0.6 * env(t));
    b.tone(0.0, 0.25, Wave::Square, move |t| f(t) * 0.5, |t| 0.2 * env(t));
    b.noise(0.0, 0.07, 7, |t| 0.5 * (1.0 - t / 0.07));
    b.lowpass(2100.0);
    b.lowpass(3200.0);
    b.drive(1.7);
    b
}

pub fn render(s: Sfx) -> Vec<u8> {
    let peak;
    let b = match s {
        Sfx::Pop(i) => {
            peak = 0.8;
            let f = 523.25 * 2f32.powf(SCALE[(i as usize).min(7)] / 12.0);
            let mut b = Buf::new(0.2);
            b.tone(0.0, 0.19, Wave::Pluck, |_| f, |t| pluck_env(t, 0.19, 0.003));
            b.tone(0.0, 0.05, Wave::Sine, |_| f * 3.0, |t| 0.3 * pluck_env(t, 0.05, 0.001));
            b
        }
        Sfx::Swap => {
            peak = 0.4;
            let mut b = Buf::new(0.12);
            b.tone(0.0, 0.11, Wave::Sine, |t| 360.0 + 4200.0 * t, |t| pluck_env(t, 0.11, 0.01));
            b.noise(0.0, 0.1, 3, |t| 0.25 * pluck_env(t, 0.1, 0.02));
            b.highpass(300.0);
            b
        }
        Sfx::Bonk => {
            peak = 0.6;
            let mut b = Buf::new(0.26);
            b.tone(0.0, 0.11, Wave::Tri, |_| 196.0, |t| pluck_env(t, 0.11, 0.004));
            b.tone(0.1, 0.15, Wave::Tri, |_| 139.0, |t| pluck_env(t, 0.15, 0.004));
            b
        }
        Sfx::Land => {
            peak = 0.5;
            let mut b = Buf::new(0.08);
            b.tone(0.0, 0.075, Wave::Sine, |t| 150.0 - 900.0 * t, |t| pluck_env(t, 0.075, 0.002));
            b
        }
        Sfx::Bark => {
            peak = 0.85;
            one_bark(1.0)
        }
        Sfx::Bark2 => {
            peak = 0.85;
            let mut b = Buf::new(0.5);
            b.mix(&one_bark(1.04), 0.0, 1.0);
            b.mix(&one_bark(0.93), 0.21, 1.0);
            b
        }
        Sfx::Squeak => {
            peak = 0.5;
            let mut b = Buf::new(0.3);
            for (at, dur, base, bend, amp) in [(0.0, 0.13, 1500.0, 1100.0, 1.0), (0.15, 0.12, 1250.0, 750.0, 0.75)] {
                let f = move |t: f32| base + bend * (PI * t / dur).sin();
                b.tone(at, dur, Wave::Sine, f, move |t| amp * hold_env(t, dur, 0.008, 0.03));
                b.tone(at, dur, Wave::Sine, move |t| f(t) * 2.0, move |t| amp * 0.22 * hold_env(t, dur, 0.008, 0.03));
            }
            b
        }
        Sfx::Zoom => {
            peak = 0.55;
            let mut b = Buf::new(0.5);
            b.noise(0.0, 0.45, 11, |t| 0.5 * (PI * t / 0.45).sin().powi(2));
            b.highpass(900.0);
            b.tone(0.0, 0.42, Wave::Sine, |t| 420.0 * (1.0 + 2.6 * t / 0.42), |t| 0.35 * hold_env(t, 0.42, 0.03, 0.12));
            // the scrabble of claws on a hard floor
            let mut k = 0;
            while (k as f32) * 0.043 < 0.4 {
                let at = k as f32 * 0.043;
                b.tone(at, 0.012, Wave::Tri, |_| 1900.0, |t| 0.32 * (1.0 - t / 0.012));
                k += 1;
            }
            b
        }
        Sfx::Boom => {
            peak = 0.9;
            let mut b = Buf::new(0.55);
            b.noise(0.0, 0.25, 5, |t| 0.6 * pluck_env(t, 0.25, 0.002));
            b.lowpass(900.0);
            b.tone(0.0, 0.5, Wave::Sine, |t| 45.0 + 85.0 * (-t * 9.0).exp(), |t| pluck_env(t, 0.5, 0.003));
            b
        }
        Sfx::Chime => {
            peak = 0.5;
            let mut b = Buf::new(0.75);
            bell(&mut b, 0.0, 1318.5, 0.45, 1.0);
            bell(&mut b, 0.07, 1568.0, 0.45, 0.9);
            bell(&mut b, 0.14, 2093.0, 0.6, 0.9);
            b
        }
        Sfx::Sparkle => {
            peak = 0.38;
            let mut b = Buf::new(0.3);
            bell(&mut b, 0.0, 1760.0, 0.2, 1.0);
            bell(&mut b, 0.05, 2637.0, 0.24, 0.8);
            b
        }
        Sfx::Win => {
            peak = 0.6;
            let mut b = Buf::new(2.2);
            for (i, n) in [72.0, 76.0, 79.0, 84.0].iter().enumerate() {
                b.tone(i as f32 * 0.115, 0.3, Wave::Pluck, |_| midi(*n), |t| pluck_env(t, 0.3, 0.004));
            }
            for (n, amp) in [(72.0, 0.6), (76.0, 0.55), (79.0, 0.55), (84.0, 0.7), (88.0, 0.35)] {
                b.tone(0.52, 1.6, Wave::Pluck, |_| midi(n), move |t| amp * pluck_env(t, 1.6, 0.006));
                b.tone(0.52, 1.6, Wave::Tri, |t| midi(n) * (1.0 + 0.004 * (t * 36.0).sin()), move |t| amp * 0.3 * hold_env(t, 1.6, 0.05, 1.1));
            }
            bell(&mut b, 0.56, 2093.0, 0.9, 0.3);
            bell(&mut b, 0.74, 2637.0, 0.9, 0.25);
            b
        }
        Sfx::Lose => {
            // the sad trombone
            peak = 0.5;
            let mut b = Buf::new(2.3);
            for (at, dur, n) in [(0.0, 0.36, 58.0), (0.4, 0.36, 57.0), (0.8, 0.36, 56.0), (1.2, 1.0, 55.0)] {
                let long = dur > 0.5;
                let f = move |t: f32| {
                    let vib = if long { 1.0 + 0.012 * (t * TAU * 5.5).sin() * (t / dur) } else { 1.0 };
                    let droop = if long { 1.0 - 0.07 * (t / dur).powi(3) } else { 1.0 };
                    midi(n) * vib * droop
                };
                let env = move |t: f32| hold_env(t, dur, 0.05, if long { 0.5 } else { 0.08 });
                b.tone(at, dur, Wave::Saw, f, move |t| 0.7 * env(t));
                b.tone(at, dur, Wave::Square, f, move |t| 0.25 * env(t));
            }
            b.lowpass(1100.0);
            b
        }
        Sfx::Paw(i) => {
            peak = 0.55;
            let f = 1046.5 * [1.0, 1.26, 1.5][(i as usize).min(2)];
            let mut b = Buf::new(0.7);
            bell(&mut b, 0.0, f, 0.6, 1.0);
            bell(&mut b, 0.05, f * 2.0, 0.5, 0.4);
            b
        }
        Sfx::Click => {
            peak = 0.35;
            let mut b = Buf::new(0.05);
            b.tone(0.0, 0.045, Wave::Sine, |t| 1150.0 - 6000.0 * t, |t| pluck_env(t, 0.045, 0.001));
            b
        }
        Sfx::Whine => {
            peak = 0.35;
            let mut b = Buf::new(0.7);
            let f = |t: f32| (980.0 - 420.0 * t) * (1.0 + 0.02 * (t * TAU * 7.0).sin());
            b.tone(0.0, 0.65, Wave::Sine, f, |t| hold_env(t, 0.65, 0.12, 0.3));
            b.tone(0.0, 0.65, Wave::Tri, move |t| f(t) * 2.0, |t| 0.18 * hold_env(t, 0.65, 0.12, 0.3));
            b
        }
        Sfx::Rip => {
            peak = 0.5;
            let mut b = Buf::new(0.18);
            b.noise(0.0, 0.17, 23, |t| pluck_env(t, 0.17, 0.004) * (0.55 + 0.45 * (t * TAU * 95.0).sin().signum()));
            b.highpass(1400.0);
            b
        }
        Sfx::Splat => {
            peak = 0.6;
            let mut b = Buf::new(0.22);
            b.noise(0.0, 0.2, 31, |t| pluck_env(t, 0.2, 0.004));
            b.lowpass(650.0);
            b.tone(0.0, 0.16, Wave::Sine, |t| 300.0 - 1100.0 * t, |t| 0.7 * pluck_env(t, 0.16, 0.003));
            b
        }
        Sfx::Splash => {
            peak = 0.5;
            let mut b = Buf::new(0.5);
            // the slap, then the water closing over it, then a few drops
            b.noise(0.0, 0.07, 41, |t| pluck_env(t, 0.07, 0.002));
            b.noise(0.03, 0.42, 43, |t| 0.6 * hold_env(t, 0.42, 0.03, 0.34));
            b.lowpass(2100.0);
            b.tone(0.0, 0.2, Wave::Sine, |t| 240.0 - 700.0 * t, |t| 0.5 * pluck_env(t, 0.2, 0.004));
            for (at, f) in [(0.2, 1250.0), (0.27, 1650.0), (0.33, 1400.0)] {
                b.tone(at, 0.05, Wave::Sine, move |t| f + 5200.0 * t, |t| 0.16 * pluck_env(t, 0.05, 0.002));
            }
            b
        }
        Sfx::Howl => {
            peak = 0.45;
            let mut b = Buf::new(1.6);
            let f = |t: f32| {
                let base = if t < 0.3 { 330.0 + 290.0 * (t / 0.3) } else { 620.0 - 140.0 * ((t - 0.3) / 1.2) };
                base * (1.0 + 0.016 * (t * TAU * 5.5).sin() * (t / 1.5))
            };
            b.tone(0.0, 1.5, Wave::Tri, f, |t| hold_env(t, 1.5, 0.12, 0.5));
            b.tone(0.0, 1.5, Wave::Sine, move |t| f(t) * 2.0, |t| 0.3 * hold_env(t, 1.5, 0.12, 0.5));
            b.lowpass(2600.0);
            b
        }
        Sfx::Tick => {
            peak = 0.3;
            let mut b = Buf::new(0.05);
            b.tone(0.0, 0.04, Wave::Sine, |_| 1568.0, |t| pluck_env(t, 0.04, 0.001));
            b
        }
        Sfx::Yip => {
            peak = 0.4;
            let mut b = Buf::new(0.22);
            for at in [0.0, 0.11] {
                b.tone(at, 0.08, Wave::Sine, |t| 700.0 + 6500.0 * t, |t| hold_env(t, 0.08, 0.006, 0.03));
                b.tone(at, 0.08, Wave::Saw, |t| 700.0 + 6500.0 * t, |t| 0.25 * hold_env(t, 0.08, 0.006, 0.03));
            }
            b.lowpass(3200.0);
            b
        }
        Sfx::Bone => {
            peak = 0.5;
            let mut b = Buf::new(0.5);
            b.tone(0.0, 0.2, Wave::Pluck, |_| midi(79.0), |t| pluck_env(t, 0.2, 0.003));
            b.tone(0.1, 0.38, Wave::Pluck, |_| midi(84.0), |t| pluck_env(t, 0.38, 0.003));
            b
        }
    };
    b.wav(peak)
}

/// An eight-bar loop: bouncy bass, ukulele-ish chords, a whistled tune.
pub fn music() -> Vec<u8> {
    let beat = 0.56;
    let bar = beat * 4.0;
    let mut b = Buf::new(bar * 8.0);
    b.wrap = true;
    // (bass root, triad) per half bar
    const C: (f32, [f32; 3]) = (48.0, [60.0, 64.0, 67.0]);
    const G: (f32, [f32; 3]) = (43.0, [59.0, 62.0, 67.0]);
    const AM: (f32, [f32; 3]) = (45.0, [57.0, 60.0, 64.0]);
    const F: (f32, [f32; 3]) = (41.0, [57.0, 60.0, 65.0]);
    let halves = [C, C, G, G, AM, AM, F, F, C, C, G, G, F, G, C, C];
    for (h, (root, triad)) in halves.iter().enumerate() {
        let t0 = h as f32 * beat * 2.0;
        // bass: root, then the fifth on the off beat
        b.tone(t0, beat * 0.9, Wave::Tri, |_| midi(*root), |t| 0.5 * pluck_env(t, beat * 0.9, 0.006));
        b.tone(t0 + beat * 1.5, beat * 0.45, Wave::Tri, |_| midi(*root + 7.0), |t| 0.34 * pluck_env(t, beat * 0.45, 0.006));
        // kick under the bass
        b.tone(t0, 0.11, Wave::Sine, |t| 50.0 + 70.0 * (-t * 40.0).exp(), |t| 0.5 * pluck_env(t, 0.11, 0.002));
        // chord stab on the back beat, strummed a few ms apart
        for (k, n) in triad.iter().enumerate() {
            let at = t0 + beat + k as f32 * 0.012;
            b.tone(at, 0.3, Wave::Pluck, |_| midi(*n + 12.0), |t| 0.15 * pluck_env(t, 0.3, 0.003));
        }
        // light arpeggio on the eighths in between
        for e in 0..4 {
            let n = triad[(e + h) % 3] + 12.0;
            b.tone(t0 + e as f32 * beat * 0.5 + 0.004, 0.2, Wave::Pluck, |_| midi(n), |t| 0.06 * pluck_env(t, 0.2, 0.003));
            // shaker
            let accent = if e % 2 == 1 { 0.075 } else { 0.04 };
            b.noise(t0 + e as f32 * beat * 0.5, 0.03, 40 + (h * 4 + e) as u32, |t| accent * (1.0 - t / 0.03));
        }
    }
    // tune: (bar, start eighth, length in eighths, note)
    let tune: [(u8, u8, u8, f32); 35] = [
        (0, 0, 2, 76.0), (0, 2, 2, 79.0), (0, 5, 1, 76.0), (0, 6, 2, 72.0),
        (1, 0, 2, 74.0), (1, 2, 2, 71.0), (1, 4, 1, 74.0), (1, 5, 1, 79.0), (1, 6, 2, 74.0),
        (2, 0, 2, 72.0), (2, 2, 2, 76.0), (2, 5, 1, 72.0), (2, 6, 2, 69.0),
        (3, 0, 3, 69.0), (3, 3, 1, 72.0), (3, 4, 2, 77.0), (3, 6, 2, 72.0),
        (4, 0, 1, 76.0), (4, 1, 1, 79.0), (4, 2, 2, 84.0), (4, 4, 2, 79.0), (4, 6, 2, 76.0),
        (5, 0, 1, 74.0), (5, 1, 1, 79.0), (5, 2, 2, 83.0), (5, 4, 2, 79.0), (5, 6, 2, 74.0),
        (6, 0, 2, 81.0), (6, 2, 2, 77.0), (6, 4, 2, 79.0), (6, 6, 2, 74.0),
        (7, 0, 2, 76.0), (7, 2, 2, 72.0), (7, 4, 3, 67.0), (7, 7, 1, 72.0),
    ];
    for (bar_i, start, len, note) in tune {
        let at = bar_i as f32 * bar + start as f32 * beat * 0.5;
        let dur = len as f32 * beat * 0.5 * 0.92;
        let f = move |t: f32| midi(note) * (1.0 + 0.005 * (t * TAU * 5.5).sin() * (t / dur).min(1.0));
        let env = move |t: f32| hold_env(t, dur, 0.025, 0.07) * (0.75 + 0.25 * (-t * 6.0).exp());
        b.tone(at, dur, Wave::Sine, f, move |t| 0.26 * env(t));
        b.tone(at, dur, Wave::Sine, move |t| f(t) * 2.0, move |t| 0.045 * env(t));
        b.tone(at, dur, Wave::Tri, f, move |t| 0.05 * env(t));
    }
    b.wav(0.75)
}

pub const ALL: [Sfx; 32] = [
    Sfx::Pop(0), Sfx::Pop(1), Sfx::Pop(2), Sfx::Pop(3), Sfx::Pop(4), Sfx::Pop(5), Sfx::Pop(6), Sfx::Pop(7),
    Sfx::Swap, Sfx::Bonk, Sfx::Land, Sfx::Bark, Sfx::Bark2, Sfx::Squeak, Sfx::Zoom, Sfx::Boom, Sfx::Chime,
    Sfx::Sparkle, Sfx::Win, Sfx::Lose, Sfx::Paw(0), Sfx::Paw(1), Sfx::Paw(2), Sfx::Click, Sfx::Whine, Sfx::Rip,
    Sfx::Splat, Sfx::Howl, Sfx::Tick, Sfx::Yip, Sfx::Bone, Sfx::Splash,
];
