//! Playback. The sounds themselves are made in `synth`.

pub use crate::synth::Sfx;
use crate::synth::{music, render, ALL};
use quad_snd::{AudioContext, PlaySoundParams, Playback, Sound};

pub struct Audio {
    ctx: AudioContext,
    sounds: Vec<(Sfx, Sound, f64)>,
    music: Sound,
    playing: Option<Playback>,
    pub sfx_on: bool,
    pub music_on: bool,
    music_gain: f32,
}

const MUSIC_VOLUME: f32 = 0.32;

impl Audio {
    pub fn new(sfx_on: bool, music_on: bool) -> Audio {
        let ctx = AudioContext::new();
        let sounds = ALL.iter().map(|&s| (s, Sound::load(&ctx, &render(s)), -1.0)).collect();
        let music = Sound::load(&ctx, &music());
        Audio { ctx, sounds, music, playing: None, sfx_on, music_on, music_gain: 1.0 }
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
            if now - *last < 0.035 || !sound.is_loaded() {
                return;
            }
            *last = now;
            sound.play(&self.ctx, PlaySoundParams { looped: false, volume: volume.clamp(0.0, 1.0) });
        }
    }

    /// Call every frame: starts the music once it has decoded, and applies
    /// the on/off switch.
    pub fn update(&mut self) {
        if self.music_on && self.playing.is_none() && self.music.is_loaded() {
            let pb = self.music.play(&self.ctx, PlaySoundParams { looped: true, volume: MUSIC_VOLUME * self.music_gain });
            self.playing = Some(pb);
        } else if !self.music_on {
            if let Some(pb) = self.playing.take() {
                pb.stop(&self.ctx);
            }
        }
    }

    /// Ducks the music under big moments (1.0 = normal).
    pub fn set_music_gain(&mut self, gain: f32) {
        if (gain - self.music_gain).abs() > 0.01 {
            self.music_gain = gain;
            if let Some(pb) = &self.playing {
                pb.set_volume(&self.ctx, MUSIC_VOLUME * gain);
            }
        }
    }
}
