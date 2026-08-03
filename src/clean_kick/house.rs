use crate::clean_kick::KickError;
use crate::dsp::six_op_pm::SineTable;
use std::f32::consts::TAU;

const START_HZ: f32 = 156.0;
const BODY_HZ: f32 = 52.0;
const PITCH_MS: f32 = 55.0;
const MODIFIER_RATIO: f32 = 2.0;
const INITIAL_INDEX_RADIANS: f32 = 0.9;
const INDEX_MS: f32 = 45.0;
const ATTACK_MS: f32 = 1.0;
const DECAY_60_DB_MS: f32 = 320.0;
const OUTPUT_GAIN: f32 = 0.8;

pub struct HouseImpact {
    table: SineTable,
    sample_rate: f32,
    carrier_phase: f32,
    modifier_phase: f32,
    frequency: f32,
    pitch_step: f32,
    pitch_remaining: u32,
    pitch_frames: u32,
    index: f32,
    index_step: f32,
    index_remaining: u32,
    index_frames: u32,
    amplitude: f32,
    decay_step: f32,
    attack_frames: u32,
    frame: u32,
    active: bool,
}

impl HouseImpact {
    pub fn new(sample_rate: u32) -> Result<Self, KickError> {
        if sample_rate < 8_000 {
            return Err(KickError::InvalidSampleRate);
        }
        let pitch_frames = ms_frames(PITCH_MS, sample_rate);
        let index_frames = ms_frames(INDEX_MS, sample_rate);
        let attack_frames = ms_frames(ATTACK_MS, sample_rate);
        let decay_frames = ms_frames(DECAY_60_DB_MS, sample_rate);
        Ok(Self {
            table: SineTable::new(),
            sample_rate: sample_rate as f32,
            carrier_phase: 0.0,
            modifier_phase: 0.0,
            frequency: START_HZ,
            pitch_step: (BODY_HZ / START_HZ).powf(1.0 / pitch_frames as f32),
            pitch_remaining: 0,
            pitch_frames,
            index: INITIAL_INDEX_RADIANS,
            index_step: 1.0e-3_f32.powf(1.0 / index_frames as f32),
            index_remaining: 0,
            index_frames,
            amplitude: 0.0,
            decay_step: 1.0e-3_f32.powf(1.0 / decay_frames as f32),
            attack_frames,
            frame: 0,
            active: false,
        })
    }

    pub fn trigger(&mut self) {
        self.carrier_phase = 0.0;
        self.modifier_phase = 0.0;
        self.frequency = START_HZ;
        self.pitch_remaining = self.pitch_frames;
        self.index = INITIAL_INDEX_RADIANS;
        self.index_remaining = self.index_frames;
        self.amplitude = 0.0;
        self.frame = 0;
        self.active = true;
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.frame < self.attack_frames {
            self.amplitude = self.frame as f32 / self.attack_frames as f32;
        } else {
            self.amplitude *= self.decay_step;
        }
        let modifier = SineTable::lookup(&self.table, self.modifier_phase);
        let offset_cycles = self.index * modifier / TAU;
        let output = OUTPUT_GAIN
            * self.amplitude
            * SineTable::lookup(&self.table, self.carrier_phase + offset_cycles);
        self.carrier_phase = (self.carrier_phase + self.frequency / self.sample_rate).fract();
        self.modifier_phase =
            (self.modifier_phase + MODIFIER_RATIO * self.frequency / self.sample_rate).fract();
        if self.pitch_remaining > 0 {
            self.pitch_remaining -= 1;
            self.frequency = if self.pitch_remaining == 0 {
                BODY_HZ
            } else {
                self.frequency * self.pitch_step
            };
        }
        if self.index_remaining > 0 {
            self.index_remaining -= 1;
            self.index = if self.index_remaining == 0 {
                0.0
            } else {
                self.index * self.index_step
            };
        }
        self.frame = self.frame.saturating_add(1);
        if self.amplitude < 1.0e-8 && self.frame > self.attack_frames {
            self.active = false;
            return 0.0;
        }
        if output.is_finite() { output } else { 0.0 }
    }
}

fn ms_frames(milliseconds: f32, sample_rate: u32) -> u32 {
    (milliseconds * sample_rate as f32 / 1_000.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn pitch_and_modifier_settle_on_the_declared_frames() {
        let mut voice = HouseImpact::new(48_000).unwrap();
        voice.trigger();
        assert_no_alloc(|| {
            for _ in 0..voice.pitch_frames {
                voice.sample();
            }
        });
        assert_eq!(voice.frequency.to_bits(), BODY_HZ.to_bits());
        assert_eq!(voice.pitch_remaining, 0);
        assert_eq!(voice.index, 0.0);
    }

    #[test]
    fn output_is_bounded_continuous_and_reaches_exact_silence() {
        let mut voice = HouseImpact::new(48_000).unwrap();
        voice.trigger();
        let mut previous = 0.0;
        let mut maximum_jump = 0.0_f32;
        assert_no_alloc(|| {
            for _ in 0..48_000 {
                let sample = voice.sample();
                maximum_jump = maximum_jump.max((sample - previous).abs());
                previous = sample;
            }
        });
        assert!(maximum_jump < 0.10, "maximum_jump={maximum_jump}");
        assert_eq!(voice.sample(), 0.0);
    }

    #[test]
    fn invalid_sample_rate_is_rejected() {
        assert!(matches!(
            HouseImpact::new(0),
            Err(KickError::InvalidSampleRate)
        ));
    }
}
