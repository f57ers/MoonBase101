use bevy::prelude::*;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink, Source};
use std::f32::consts::TAU;
use std::sync::mpsc::{self, Sender};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SfxKind {
    UiClick,
    UiConfirm,
    JumpLowG,
    JetpackBurst,
    LaserMine,
    BuildModule,
    CraftUpgrade,
    RoverToggle,
    FlareThrow,
    AlertSiren,
    HighFive,
}

#[derive(Debug, Clone, Copy)]
pub enum AudioCommand {
    PlaySfx(SfxKind, f32),
    SetMusicVolume(f32),
}

#[derive(Resource, Clone)]
pub struct GameAudio {
    pub tx: Option<Sender<AudioCommand>>,
}

impl GameAudio {
    pub fn play(&self, kind: SfxKind, volume: f32) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(AudioCommand::PlaySfx(kind, volume.clamp(0.0, 1.0)));
        }
    }

    pub fn set_music_volume(&self, volume: f32) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(AudioCommand::SetMusicVolume(volume.clamp(0.0, 1.0)));
        }
    }
}

pub fn setup_audio_engine(mut commands: Commands, settings: Res<crate::types::GameSettings>) {
    let (tx, rx) = mpsc::channel::<AudioCommand>();
    let initial_volume = settings.music_volume * settings.master_volume;

    thread::spawn(move || {
        let Ok((_stream, stream_handle)) = OutputStream::try_default() else {
            return;
        };

        // Background ambient space soundtrack sink (looping procedural cosmic synth)
        let music_sink = Sink::try_new(&stream_handle).ok();
        if let Some(ref sink) = music_sink {
            let soundtrack_samples = synthesize_space_soundtrack(24.0);
            let buf = SamplesBuffer::new(1, 22050, soundtrack_samples);
            sink.set_volume(initial_volume);
            sink.append(buf.repeat_infinite());
        }

        let mut current_music_vol = initial_volume;

        loop {
            // Keep looping the procedural space soundtrack if it finishes
            if let Some(ref sink) = music_sink {
                sink.set_volume(current_music_vol);
            }

            match rx.recv_timeout(std::time::Duration::from_millis(120)) {
                Ok(AudioCommand::PlaySfx(kind, vol)) => {
                    if vol > 0.01 {
                        let samples = synthesize_sfx(kind, vol);
                        let buf = SamplesBuffer::new(1, 22050, samples);
                        let _ = stream_handle.play_raw(buf);
                    }
                }
                Ok(AudioCommand::SetMusicVolume(v)) => {
                    current_music_vol = v;
                    if let Some(ref sink) = music_sink {
                        sink.set_volume(current_music_vol);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    commands.insert_resource(GameAudio { tx: Some(tx) });
}

/// Synthesizes a 24-second seamless ambient space soundtrack (warm pads + cosmic arpeggio + sub bass).
fn synthesize_space_soundtrack(duration_sec: f32) -> Vec<f32> {
    let sr = 22050usize;
    let total = (duration_sec * sr as f32) as usize;
    let mut out = Vec::with_capacity(total);

    // Chords in D Dorian / Space Lydian: (root, fifth, ninth, twelfth)
    let chords: [[f32; 4]; 4] = [
        [146.83, 220.00, 329.63, 440.00], // D9
        [130.81, 196.00, 293.66, 392.00], // Cmaj9
        [174.61, 261.63, 329.63, 523.25], // Fmaj7
        [110.00, 164.81, 246.94, 329.63], // Am9
    ];
    let arp_notes: [f32; 8] = [
        587.33, 659.25, 880.00, 659.25, 783.99, 587.33, 523.25, 440.00,
    ];

    for i in 0..total {
        let t = i as f32 / sr as f32;
        let chord_idx = ((t / 6.0) as usize) % 4;
        let chord = chords[chord_idx];
        let local_t = t % 6.0;
        let env = (local_t / 1.2).min(1.0) * ((6.0 - local_t) / 1.2).min(1.0);

        // Warm detuned space pad
        let mut pad = 0.0f32;
        for &freq in &chord {
            let s1 = (t * freq * TAU).sin();
            let s2 = (t * freq * 1.003 * TAU).sin();
            pad += (s1 + s2) * 0.11;
        }

        // Gentle crystalline space telemetry arpeggio
        let arp_step = ((t * 2.0) as usize) % arp_notes.len();
        let arp_local = (t * 2.0).fract();
        let arp_env = (-arp_local * 5.5).exp();
        let arp_freq = arp_notes[arp_step];
        let arp = (t * arp_freq * TAU).sin() * arp_env * 0.12;

        // Deep lunar heartbeat sub-bass
        let sub = (t * chord[0] * 0.5 * TAU).sin() * 0.16;

        let sample = ((pad * env + arp + sub) * 0.45).clamp(-0.85, 0.85);
        out.push(sample);
    }
    out
}

fn synthesize_sfx(kind: SfxKind, volume: f32) -> Vec<f32> {
    let sr = 22050usize;
    let dur = match kind {
        SfxKind::UiClick => 0.07,
        SfxKind::UiConfirm => 0.24,
        SfxKind::JumpLowG => 0.28,
        SfxKind::JetpackBurst => 0.14,
        SfxKind::LaserMine => 0.22,
        SfxKind::BuildModule => 0.32,
        SfxKind::CraftUpgrade => 0.38,
        SfxKind::RoverToggle => 0.26,
        SfxKind::FlareThrow => 0.30,
        SfxKind::AlertSiren => 0.55,
        SfxKind::HighFive => 0.24,
    };
    let n = (dur * sr as f32) as usize;
    let mut out = Vec::with_capacity(n);

    for i in 0..n {
        let t = i as f32 / sr as f32;
        let progress = t / dur;
        let fade = (1.0 - progress).powf(1.4);

        let raw = match kind {
            SfxKind::UiClick => {
                let f = 920.0 + progress * 480.0;
                (t * f * TAU).sin() * 0.45
            }
            SfxKind::UiConfirm => {
                let f = if progress < 0.45 { 523.25 } else { 783.99 };
                ((t * f * TAU).sin() + 0.5 * (t * f * 1.5 * TAU).sin()) * 0.38
            }
            SfxKind::JumpLowG => {
                let f = 190.0 + progress * 340.0;
                (t * f * TAU).sin() * 0.42
            }
            SfxKind::JetpackBurst => {
                let pseudo_noise = ((i as f32 * 12.9898).sin() * 43758.545).fract() * 2.0 - 1.0;
                pseudo_noise * 0.28
            }
            SfxKind::LaserMine => {
                let f = 880.0 - progress * 540.0 + (t * 65.0 * TAU).sin() * 45.0;
                (t * f * TAU).sin() * 0.40
            }
            SfxKind::BuildModule => {
                let f = if progress < 0.33 {
                    330.0
                } else if progress < 0.66 {
                    440.0
                } else {
                    659.25
                };
                (t * f * TAU).sin() * 0.42
            }
            SfxKind::CraftUpgrade => {
                let f = if progress < 0.25 {
                    440.0
                } else if progress < 0.5 {
                    554.37
                } else if progress < 0.75 {
                    659.25
                } else {
                    880.0
                };
                (t * f * TAU).sin() * 0.45
            }
            SfxKind::RoverToggle => {
                let f = 140.0 + progress * 180.0;
                ((t * f * TAU).sin() + (t * f * 2.0 * TAU).sin() * 0.4) * 0.38
            }
            SfxKind::FlareThrow => {
                let f = 680.0 - progress * 320.0;
                (t * f * TAU).sin() * 0.36
            }
            SfxKind::HighFive => {
                let noise =
                    (((i as f32 * 12.9898).sin() * 43758.545).fract() * 2.0 - 1.0).clamp(-1.0, 1.0);
                let attack = (t * 140.0).min(1.0);
                let decay = (-t * 22.0).exp();
                (noise * 0.75 + (t * 120.0 * TAU).sin() * 0.45) * attack * decay * 1.8
            }
            SfxKind::AlertSiren => {
                let f = if (t * 5.0).fract() < 0.5 {
                    640.0
                } else {
                    480.0
                };
                (t * f * TAU).sin() * 0.45
            }
        };

        out.push((raw * fade * volume).clamp(-0.9, 0.9));
    }
    out
}
