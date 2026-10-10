//! Alert sounds. Tones are synthesized (no audio files to ship or license) and played on a
//! dedicated thread that opens the default output device per play, so a headset plugged in
//! after launch is used.

use std::num::NonZero;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use crate::settings::SoundName;

pub const SAMPLE_RATE: u32 = 44_100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneKind {
    /// Deadline reached: full tone.
    Due,
    /// Lead-time warning: shorter and quieter.
    Lead,
}

fn tone(freqs: &[(f32, f32)], seconds: f32, attack: f32, decay: f32, out: &mut Vec<f32>, gain: f32) {
    let n = (seconds * SAMPLE_RATE as f32) as usize;
    let release = (0.03 * SAMPLE_RATE as f32) as usize; // 30 ms fade to zero: no click
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env_attack = (t / attack).min(1.0);
        let env_decay = (-t / decay).exp();
        let env_release = if i + release > n { (n - i) as f32 / release as f32 } else { 1.0 };
        let mut s = 0.0;
        for (f, a) in freqs {
            s += a * (std::f32::consts::TAU * f * t).sin();
        }
        out.push(s * env_attack * env_decay * env_release * gain);
    }
}

fn silence(seconds: f32, out: &mut Vec<f32>) {
    out.extend(std::iter::repeat_n(0.0, (seconds * SAMPLE_RATE as f32) as usize));
}

/// Mono samples in [-1, 1].
pub fn synthesize(sound: SoundName, kind: ToneKind) -> Vec<f32> {
    let mut v = Vec::new();
    match (sound, kind) {
        (SoundName::Chime, ToneKind::Due) => {
            for _ in 0..2 {
                tone(&[(880.0, 0.7), (1760.0, 0.15)], 0.32, 0.005, 0.18, &mut v, 0.8);
                tone(&[(1318.5, 0.7), (2637.0, 0.12)], 0.55, 0.005, 0.25, &mut v, 0.8);
                silence(0.18, &mut v);
            }
        }
        (SoundName::Bell, ToneKind::Due) => {
            // Inharmonic partials give a struck-bell character.
            for _ in 0..2 {
                tone(&[(660.0, 0.6), (660.0 * 2.76, 0.25), (660.0 * 5.40, 0.1)], 1.0, 0.003, 0.35, &mut v, 0.8);
            }
        }
        (SoundName::Soft, ToneKind::Due) => {
            for _ in 0..2 {
                tone(&[(523.25, 0.5), (659.25, 0.4)], 0.6, 0.06, 0.4, &mut v, 0.7);
                silence(0.15, &mut v);
            }
        }
        (_, ToneKind::Lead) => {
            tone(&[(784.0, 0.6), (1568.0, 0.1)], 0.45, 0.02, 0.2, &mut v, 0.5);
        }
    }
    v
}

enum Cmd {
    Play { samples: Vec<f32>, volume: f32, reply: Sender<Result<(), String>> },
    Stop,
}

/// Owns the audio thread.
#[derive(Clone)]
pub struct AudioPlayer {
    tx: Sender<Cmd>,
}

impl AudioPlayer {
    pub fn spawn() -> AudioPlayer {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("alert-audio".into())
            .spawn(move || audio_thread(rx))
            .expect("spawn audio thread");
        AudioPlayer { tx }
    }

    /// Play a tone, replacing anything playing. Blocks only until the device is opened.
    pub fn play(&self, sound: SoundName, kind: ToneKind, volume: f64) -> Result<(), String> {
        let (reply, rx) = mpsc::channel();
        let samples = synthesize(sound, kind);
        self.tx
            .send(Cmd::Play { samples, volume: volume.clamp(0.0, 1.0) as f32, reply })
            .map_err(|_| "audio thread is not running".to_string())?;
        rx.recv_timeout(Duration::from_secs(5)).unwrap_or_else(|_| Err("audio device did not respond".into()))
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Cmd::Stop);
    }
}

fn audio_thread(rx: Receiver<Cmd>) {
    let mut current: Option<(rodio::MixerDeviceSink, rodio::Player)> = None;
    loop {
        // While playing, poll so the device is released as soon as the tone ends.
        let msg = if current.is_some() {
            match rx.recv_timeout(Duration::from_millis(150)) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match rx.recv() {
                Ok(m) => Some(m),
                Err(_) => return,
            }
        };
        match msg {
            Some(Cmd::Stop) => {
                if let Some((_, p)) = current.take() {
                    p.stop();
                }
            }
            Some(Cmd::Play { samples, volume, reply }) => {
                if let Some((_, p)) = current.take() {
                    p.stop();
                }
                let result = (|| {
                    let mut sink = rodio::DeviceSinkBuilder::open_default_sink()
                        .map_err(|e| format!("no audio output device available ({e})"))?;
                    sink.log_on_drop(false);
                    let player = rodio::Player::connect_new(sink.mixer());
                    player.set_volume(volume);
                    let buf = rodio::buffer::SamplesBuffer::new(
                        NonZero::new(1).expect("1 channel"),
                        NonZero::new(SAMPLE_RATE).expect("rate"),
                        samples,
                    );
                    player.append(buf);
                    Ok((sink, player))
                })();
                match result {
                    Ok(pair) => {
                        current = Some(pair);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            None => {
                if current.as_ref().is_some_and(|(_, p)| p.empty()) {
                    current = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tone_is_short_bounded_and_ends_silent() {
        for sound in [SoundName::Chime, SoundName::Bell, SoundName::Soft] {
            for kind in [ToneKind::Due, ToneKind::Lead] {
                let s = synthesize(sound, kind);
                let secs = s.len() as f32 / SAMPLE_RATE as f32;
                assert!(secs > 0.3 && secs < 3.0, "{sound:?}/{kind:?} length {secs}");
                assert!(s.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "{sound:?}/{kind:?} out of range");
                let peak = s.iter().fold(0f32, |m, x| m.max(x.abs()));
                assert!(peak > 0.1, "{sound:?}/{kind:?} is near-silent");
                let tail = s.iter().rev().find(|x| **x != 0.0).copied().unwrap_or(0.0);
                assert!(tail.abs() < 0.02, "{sound:?}/{kind:?} ends abruptly");
            }
        }
    }

    #[test]
    fn lead_tone_is_shorter_than_due_tone() {
        let due = synthesize(SoundName::Chime, ToneKind::Due).len();
        let lead = synthesize(SoundName::Chime, ToneKind::Lead).len();
        assert!(lead < due);
    }
}
