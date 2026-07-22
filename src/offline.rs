use crate::engine::{Engine, EngineError, Event, TimedEvent};
use crate::preset::Preset;
use std::path::Path;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSpec {
    pub sample_rate: u32,
    pub note: u8,
    pub velocity: f32,
    pub seconds: f32,
}

#[derive(Debug, Error)]
pub enum OfflineError {
    #[error("sample rate must be between 8,000 and 384,000 Hz")]
    InvalidSampleRate,
    #[error("MIDI note must be between 0 and 127")]
    InvalidNote,
    #[error("velocity must be finite and between 0 and 1")]
    InvalidVelocity,
    #[error("duration must be finite, positive, and no longer than 600 seconds")]
    InvalidDuration,
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),
}

impl RenderSpec {
    fn sample_count(self) -> Result<usize, OfflineError> {
        if !(8_000..=384_000).contains(&self.sample_rate) {
            return Err(OfflineError::InvalidSampleRate);
        }
        if self.note > 127 {
            return Err(OfflineError::InvalidNote);
        }
        if !self.velocity.is_finite() || !(0.0..=1.0).contains(&self.velocity) {
            return Err(OfflineError::InvalidVelocity);
        }
        if !self.seconds.is_finite() || self.seconds <= 0.0 || self.seconds > 600.0 {
            return Err(OfflineError::InvalidDuration);
        }
        Ok((self.seconds * self.sample_rate as f32).round() as usize)
    }
}

pub fn render_note(preset: &Preset, spec: RenderSpec) -> Result<Vec<f32>, OfflineError> {
    let sample_count = spec.sample_count()?;
    let mut left = vec![0.0; sample_count];
    let mut right = vec![0.0; sample_count];
    let note_off = ((sample_count as f32 * 0.8).round() as usize).min(sample_count - 1);
    let events = [
        TimedEvent::new(
            0,
            Event::NoteOn {
                note: spec.note,
                velocity: spec.velocity,
            },
        ),
        TimedEvent::new(note_off, Event::NoteOff { note: spec.note }),
    ];
    let mut engine = Engine::new(spec.sample_rate as f32, preset)?;
    engine.render_block(&events, &mut left, &mut right)?;
    let mut interleaved = Vec::with_capacity(sample_count * 2);
    for (left, right) in left.into_iter().zip(right) {
        interleaved.push(left);
        interleaved.push(right);
    }
    Ok(interleaved)
}

pub fn write_wav(
    path: impl AsRef<Path>,
    preset: &Preset,
    spec: RenderSpec,
) -> Result<(), OfflineError> {
    let samples = render_note(preset, spec)?;
    let wav_spec = hound::WavSpec {
        channels: 2,
        sample_rate: spec.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, wav_spec)?;
    for sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::Preset;

    fn preset() -> Preset {
        Preset::parse(include_str!("../presets/reference.mojsint")).unwrap()
    }

    #[test]
    fn rendering_is_sample_for_sample_deterministic() {
        let spec = RenderSpec {
            sample_rate: 8_000,
            note: 60,
            velocity: 0.8,
            seconds: 0.1,
        };
        let first = render_note(&preset(), spec).unwrap();
        let second = render_note(&preset(), spec).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 1_600);
        assert!(first.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn wav_output_is_stereo_float_and_byte_identical() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first.wav");
        let second = directory.path().join("second.wav");
        let spec = RenderSpec {
            sample_rate: 8_000,
            note: 69,
            velocity: 1.0,
            seconds: 0.05,
        };
        write_wav(&first, &preset(), spec).unwrap();
        write_wav(&second, &preset(), spec).unwrap();
        assert_eq!(
            std::fs::read(&first).unwrap(),
            std::fs::read(&second).unwrap()
        );
        let reader = hound::WavReader::open(first).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        assert_eq!(reader.spec().sample_rate, 8_000);
    }
}
