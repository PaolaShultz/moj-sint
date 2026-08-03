use crate::clean_kick::{DcBlocker, KickConfig, KickError};
use std::f64::consts::{PI, TAU};

const DURATION_SECONDS: f64 = 1.5;
const EXCITATION_SECONDS: f64 = 0.001;
const IMPACT_START_HZ: f64 = 112.0;
const IMPACT_END_HZ: f64 = 76.0;
const IMPACT_SWEEP_SECONDS: f64 = 0.100;
const IMPACT_DECAY_SECONDS: f64 = 0.115;
const BODY_START_HZ: f64 = 58.0;
const BODY_END_HZ: f64 = 48.0;
const BODY_SWEEP_SECONDS: f64 = 0.280;
const BODY_DECAY_SECONDS: f64 = 0.760;
const COUPLING_SECONDS: f64 = 0.100;
const PHASE_OFFSET_CYCLES: f64 = -0.25;

pub struct LongPressure {
    samples: Box<[f32]>,
    frame: usize,
    active: bool,
}

impl LongPressure {
    pub fn new(config: KickConfig, sample_rate: u32) -> Result<Self, KickError> {
        if sample_rate < 8_000
            || !config.output_gain.is_finite()
            || !(0.0..=2.0).contains(&config.output_gain)
            || !config.modifier_amount.is_finite()
            || !(0.0..=2.0).contains(&config.modifier_amount)
        {
            return Err(KickError::InvalidSampleRate);
        }
        let frames = (DURATION_SECONDS * f64::from(sample_rate)).round() as usize;
        let mut dc_blocker = DcBlocker::new(sample_rate);
        let samples = (0..frames)
            .map(|frame| {
                let time = frame as f64 / f64::from(sample_rate);
                let excitation = if time < EXCITATION_SECONDS {
                    let phase = time / EXCITATION_SECONDS;
                    0.5 - 0.5 * (PI * phase).cos()
                } else {
                    1.0
                };
                let impact_envelope =
                    excitation * (-1000.0_f64.ln() * time / IMPACT_DECAY_SECONDS).exp();
                let body_rise = 1.0 - (-1000.0_f64.ln() * time / COUPLING_SECONDS).exp();
                let body_envelope = f64::from(config.modifier_amount)
                    * body_rise
                    * (-1000.0_f64.ln() * time / BODY_DECAY_SECONDS).exp();
                let impact_phase =
                    swept_phase(time, IMPACT_START_HZ, IMPACT_END_HZ, IMPACT_SWEEP_SECONDS);
                let body_phase = swept_phase(time, BODY_START_HZ, BODY_END_HZ, BODY_SWEEP_SECONDS);
                let impact = impact_envelope * (TAU * impact_phase).sin();
                let body = body_envelope * (TAU * body_phase).sin();
                let output = (f64::from(config.output_gain) * (0.24 * impact + body)) as f32;
                dc_blocker.sample(output)
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

fn swept_phase(time: f64, start_hz: f64, end_hz: f64, sweep_seconds: f64) -> f64 {
    let rate = 1000.0_f64.ln() / sweep_seconds;
    PHASE_OFFSET_CYCLES + end_hz * time + (start_hz - end_hz) * (1.0 - (-rate * time).exp()) / rate
}

#[cfg(test)]
fn modal_radius(decay_seconds: f64, sample_rate: u32) -> f64 {
    (-1000.0_f64.ln() / (decay_seconds * f64::from(sample_rate))).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clean_kick::KickTopology;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn analytic_modes_have_strictly_stable_equivalent_radii() {
        for decay in [IMPACT_DECAY_SECONDS, BODY_DECAY_SECONDS] {
            assert!(modal_radius(decay, 48_000) < 1.0);
        }
    }

    #[test]
    fn sample_and_retrigger_are_allocation_free_and_reach_silence() {
        let mut voice =
            LongPressure::new(KickConfig::default_for(KickTopology::LongPressure), 48_000).unwrap();
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
