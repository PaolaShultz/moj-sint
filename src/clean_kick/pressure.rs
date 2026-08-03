use crate::clean_kick::KickError;
use std::f32::consts::TAU;

const DURATION_SECONDS: f32 = 1.5;
const EXCITATION_MS: f32 = 1.0;
const IMPACT_START_HZ: f32 = 112.0;
const IMPACT_END_HZ: f32 = 76.0;
const IMPACT_SWEEP_MS: f32 = 100.0;
const IMPACT_DECAY_MS: f32 = 115.0;
const BODY_START_HZ: f32 = 58.0;
const BODY_END_HZ: f32 = 48.0;
const BODY_SWEEP_MS: f32 = 280.0;
const BODY_DECAY_MS: f32 = 760.0;
const COUPLING_MS: f32 = 100.0;
const IMPACT_INPUT_GAIN: f32 = 0.025;
const BODY_INPUT_GAIN: f32 = 0.12;
const COUPLING_GAIN: f32 = 0.16;
const OUTPUT_GAIN: f32 = 0.72;

#[derive(Clone, Copy, Default)]
struct Resonator {
    y1: f32,
    y2: f32,
}

impl Resonator {
    #[inline]
    fn sample(&mut self, input: f32, coefficients: Coefficients) -> f32 {
        let output =
            coefficients.input_gain * input + coefficients.a1 * self.y1 - coefficients.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy)]
struct Coefficients {
    a1: f32,
    a2: f32,
    input_gain: f32,
}

pub struct LongPressure {
    impact: Resonator,
    body: Resonator,
    impact_coefficients: Box<[Coefficients]>,
    body_coefficients: Box<[Coefficients]>,
    excitation: Box<[f32]>,
    coupling: Box<[f32]>,
    frame: usize,
    active: bool,
}

impl LongPressure {
    pub fn new(sample_rate: u32) -> Result<Self, KickError> {
        if sample_rate < 8_000 {
            return Err(KickError::InvalidSampleRate);
        }
        let frames = (DURATION_SECONDS * sample_rate as f32).round() as usize;
        let impact_radius = decay_radius(IMPACT_DECAY_MS, sample_rate);
        let body_radius = decay_radius(BODY_DECAY_MS, sample_rate);
        let impact_coefficients = coefficient_trajectory(
            IMPACT_START_HZ,
            IMPACT_END_HZ,
            IMPACT_SWEEP_MS,
            impact_radius,
            IMPACT_INPUT_GAIN,
            frames,
            sample_rate,
        );
        let body_coefficients = coefficient_trajectory(
            BODY_START_HZ,
            BODY_END_HZ,
            BODY_SWEEP_MS,
            body_radius,
            BODY_INPUT_GAIN,
            frames,
            sample_rate,
        );
        let excitation_frames = ms_frames(EXCITATION_MS, sample_rate) as usize;
        let excitation = (0..frames)
            .map(|frame| {
                if frame < excitation_frames {
                    let phase = frame as f32 / (excitation_frames - 1).max(1) as f32;
                    0.5 - 0.5 * (TAU * phase).cos()
                } else {
                    0.0
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let coupling_frames = ms_frames(COUPLING_MS, sample_rate) as usize;
        let coupling = (0..frames)
            .map(|frame| {
                if frame < coupling_frames {
                    COUPLING_GAIN * (1.0 - frame as f32 / coupling_frames as f32)
                } else {
                    0.0
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self {
            impact: Resonator::default(),
            body: Resonator::default(),
            impact_coefficients,
            body_coefficients,
            excitation,
            coupling,
            frame: 0,
            active: false,
        })
    }

    pub fn trigger(&mut self) {
        self.impact.clear();
        self.body.clear();
        self.frame = 0;
        self.active = true;
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        if !self.active || self.frame >= self.excitation.len() {
            self.impact.clear();
            self.body.clear();
            self.active = false;
            return 0.0;
        }
        let impact = self.impact.sample(
            self.excitation[self.frame],
            self.impact_coefficients[self.frame],
        );
        let body = self.body.sample(
            impact * self.coupling[self.frame],
            self.body_coefficients[self.frame],
        );
        self.frame += 1;
        let output = OUTPUT_GAIN * (0.28 * impact + body);
        if output.is_finite() { output } else { 0.0 }
    }
}

fn coefficient_trajectory(
    start_hz: f32,
    end_hz: f32,
    sweep_ms: f32,
    radius: f32,
    input_gain: f32,
    frames: usize,
    sample_rate: u32,
) -> Box<[Coefficients]> {
    let sweep_frames = ms_frames(sweep_ms, sample_rate).max(1) as usize;
    (0..frames)
        .map(|frame| {
            let progress = (frame as f32 / sweep_frames as f32).min(1.0);
            let frequency = start_hz * (end_hz / start_hz).powf(progress);
            Coefficients {
                a1: 2.0 * radius * (TAU * frequency / sample_rate as f32).cos(),
                a2: radius * radius,
                input_gain,
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn decay_radius(decay_60_db_ms: f32, sample_rate: u32) -> f32 {
    let frames = ms_frames(decay_60_db_ms, sample_rate).max(1);
    1.0e-3_f32.powf(1.0 / frames as f32)
}

fn ms_frames(milliseconds: f32, sample_rate: u32) -> u32 {
    (milliseconds * sample_rate as f32 / 1_000.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn every_prepared_pole_is_strictly_stable() {
        let voice = LongPressure::new(48_000).unwrap();
        for coefficients in voice
            .impact_coefficients
            .iter()
            .chain(voice.body_coefficients.iter())
        {
            assert!(coefficients.a2.sqrt() < 1.0);
            assert!(coefficients.a1.is_finite());
        }
    }

    #[test]
    fn sample_and_retrigger_are_allocation_free_and_reach_silence() {
        let mut voice = LongPressure::new(48_000).unwrap();
        assert_no_alloc(|| {
            voice.trigger();
            for _ in 0..72_000 {
                assert!(voice.sample().is_finite());
            }
            assert_eq!(voice.sample(), 0.0);
            voice.trigger();
            assert_eq!(voice.sample(), 0.0);
        });
    }
}
