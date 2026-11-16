//! Playback. The sounds themselves are made in `synth`.

pub use crate::synth::Sfx;
use crate::synth::{defend, render, trail, ALL};
use quad_snd::{AudioContext, PlaySoundParams, Playback, Sound};

/// Which tune is playing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    /// The map, the menus, the story.
    Trail,
    /// A stage.
    Defend,
}

pub struct Audio {
    ctx: AudioContext,
    sounds: Vec<(Sfx, Sound, f64)>,
    tracks: [Sound; 2],
    playing: Option<(Track, Playback)>,
    want: Track,
    pub sfx_on: bool,
    pub music_on: bool,
    music_gain: f32,
}

const MUSIC_VOLUME: f32 = 0.3;

/// Has a sound finished decoding? Only the browser decodes in the
/// background; everywhere else a sound is ready as soon as it is made.
#[cfg(target_arch = "wasm32")]
fn loaded(s: &Sound) -> bool {
    s.is_loaded()
}
#[cfg(not(target_arch = "wasm32"))]
fn loaded(_: &Sound) -> bool {
    true
}

impl Audio {
    pub fn new(sfx_on: bool, music_on: bool) -> Audio {
        let ctx = AudioContext::new();
        let sounds = ALL.iter().map(|&s| (s, Sound::load(&ctx, &render(s)), -1.0)).collect();
        let tracks = [Sound::load(&ctx, &trail()), Sound::load(&ctx, &defend())];
        Audio { ctx, sounds, tracks, playing: None, want: Track::Trail, sfx_on, music_on, music_gain: 1.0 }
    }

    /// Plays an effect. Browsers keep audio silent until the first tap; calls
    /// before that are simply not heard, and the game never waits on sound.
    pub fn play(&mut self, sfx: Sfx, volume: f32) {
        if !self.sfx_on {
            return;
        }
        let now = macroquad::time::get_time();
        if let Some((_, sound, last)) = self.sounds.iter_mut().find(|(s, _, _)| *s == sfx) {
            // the same sound stacked on itself within a frame or two just gets louder and harsher
            if now - *last < 0.05 || !loaded(sound) {
                return;
            }
            *last = now;
            sound.play(&self.ctx, PlaySoundParams { looped: false, volume: volume.clamp(0.0, 1.0) });
        }
    }

    /// Asks for a tune. The change happens on the next `update`.
    pub fn track(&mut self, t: Track) {
        self.want = t;
    }

    /// Call every frame: starts the music once it has decoded, swaps tunes,
    /// and applies the on/off switch.
    pub fn update(&mut self) {
        let current = self.playing.as_ref().map(|p| p.0);
        if !self.music_on || current.is_some_and(|c| c != self.want) {
            if let Some((_, pb)) = self.playing.take() {
                pb.stop(&self.ctx);
            }
        }
        if self.music_on && self.playing.is_none() {
            let sound = &self.tracks[self.want as usize];
            if loaded(sound) {
                let pb = sound.play(&self.ctx, PlaySoundParams { looped: true, volume: MUSIC_VOLUME * self.music_gain });
                self.playing = Some((self.want, pb));
            }
        }
    }

    /// Ducks the music under big moments (1.0 = normal).
    pub fn set_music_gain(&mut self, gain: f32) {
        if (gain - self.music_gain).abs() > 0.01 {
            self.music_gain = gain;
            if let Some((_, pb)) = &self.playing {
                pb.set_volume(&self.ctx, MUSIC_VOLUME * gain);
            }
        }
    }
}
