use crate::clean_kick::{DcBlocker, KickConfig, KickError};
use std::f64::consts::{PI, TAU};

const DURATION_SECONDS: f64 = 1.0;
const START_HZ: f64 = 156.0;
const BODY_HZ: f64 = 52.0;
const PITCH_SECONDS: f64 = 0.055;
const MODIFIER_RATIO: f64 = 2.0;
const INDEX_SECONDS: f64 = 0.045;
const ATTACK_SECONDS: f64 = 0.001;
const DECAY_60_DB_SECONDS: f64 = 0.320;
const PHASE_OFFSET_CYCLES: f64 = 0.75;

/// A phase-modulated one-shot whose complete trajectory is prepared off the
/// audio thread. Triggering and sampling only reset/read an index.
pub struct HouseImpact {
    samples: Box<[f32]>,
    frame: usize,
    active: bool,
}

impl HouseImpact {
    pub fn new(config: KickConfig, sample_rate: u32) -> Result<Self, KickError> {
        if sample_rate < 8_000
            || !config.output_gain.is_finite()
            || !(0.0..=2.0).contains(&config.output_gain)
            || !config.modifier_amount.is_finite()
            || !(0.0..=1.2).contains(&config.modifier_amount)
        {
            return Err(KickError::InvalidConfig);
        }
        let frames = (DURATION_SECONDS * f64::from(sample_rate)).round() as usize;
        let mut dc_blocker = DcBlocker::new(sample_rate);
        let samples = (0..frames)
            .map(|frame| {
                let time = frame as f64 / f64::from(sample_rate);
                let amplitude = amplitude_at(time);
                let carrier_phase = swept_phase(time);
                let modifier_phase = MODIFIER_RATIO * (carrier_phase - PHASE_OFFSET_CYCLES);
                let index = f64::from(config.modifier_amount)
                    * (-1000.0_f64.ln() * time / INDEX_SECONDS).exp();
                let modifier = (TAU * modifier_phase).sin();
                let output = f64::from(config.output_gain)
                    * amplitude
                    * (TAU * carrier_phase + index * modifier).sin();
                dc_blocker.sample(output as f32)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Ok(Self {
            samples,
            frame: 0,
            active: false,
        })
    }

    pub fn trigger(&mut self) {
        self.frame = 0;
        self.active = true;
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        if !self.active || self.frame >= self.samples.len() {
            self.active = false;
            return 0.0;
        }
        let output = self.samples[self.frame];
        self.frame += 1;
        if output.is_finite() { output } else { 0.0 }
    }
}

#[cfg(test)]
fn frequency_at(time: f64) -> f64 {
    let rate = 1000.0_f64.ln() / PITCH_SECONDS;
    BODY_HZ + (START_HZ - BODY_HZ) * (-rate * time).exp()
}

fn swept_phase(time: f64) -> f64 {
    let rate = 1000.0_f64.ln() / PITCH_SECONDS;
    PHASE_OFFSET_CYCLES
        + BODY_HZ * time
        + (START_HZ - BODY_HZ) * (1.0 - (-rate * time).exp()) / rate
}

fn amplitude_at(time: f64) -> f64 {
    let attack = if time < ATTACK_SECONDS {
        0.5 - 0.5 * (PI * time / ATTACK_SECONDS).cos()
    } else {
        1.0
    };
    attack * (-1000.0_f64.ln() * time / DECAY_60_DB_SECONDS).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clean_kick::KickTopology;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn pitch_settles_in_the_declared_body_range_without_a_phase_branch() {
        assert_eq!(frequency_at(0.0), START_HZ);
        assert!((48.0..=58.0).contains(&frequency_at(0.080)));
        let before = swept_phase(PITCH_SECONDS - 1.0e-9);
        let after = swept_phase(PITCH_SECONDS + 1.0e-9);
        assert!((after - before).abs() < 1.0e-5);
    }

    #[test]
    fn amplitude_envelope_decreases_after_the_attack_and_reaches_its_decay_window() {
        let mut previous = amplitude_at(ATTACK_SECONDS);
        for frame in 49..=48_000 {
            let current = amplitude_at(frame as f64 / 48_000.0);
            assert!(current <= previous);
            previous = current;
        }
        assert!(amplitude_at(DECAY_60_DB_SECONDS) <= 1.0e-3);
    }

    #[test]
    fn sample_and_retrigger_are_allocation_free_and_reach_exact_silence() {
        let mut voice =
            HouseImpact::new(KickConfig::default_for(KickTopology::HouseImpact), 48_000).unwrap();
        assert_no_alloc(|| {
            voice.trigger();
            for _ in 0..48_000 {
                assert!(voice.sample().is_finite());
            }
            assert_eq!(voice.sample(), 0.0);
            voice.trigger();
            assert_eq!(voice.sample(), 0.0);
        });
    }

    #[test]
    fn invalid_sample_rate_is_rejected() {
        assert!(HouseImpact::new(KickConfig::default_for(KickTopology::HouseImpact), 0).is_err());
    }
}
