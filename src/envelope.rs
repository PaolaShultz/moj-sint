use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum EnvelopeError {
    #[error("envelope times must be finite and positive")]
    InvalidTime,
    #[error("sustain must be finite and between 0 and 1")]
    InvalidSustain,
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdsrConfig {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_level: f32,
    pub release_seconds: f32,
}

impl AdsrConfig {
    pub fn new(
        attack_seconds: f32,
        decay_seconds: f32,
        sustain_level: f32,
        release_seconds: f32,
    ) -> Result<Self, EnvelopeError> {
        if !attack_seconds.is_finite()
            || attack_seconds <= 0.0
            || !decay_seconds.is_finite()
            || decay_seconds <= 0.0
            || !release_seconds.is_finite()
            || release_seconds <= 0.0
        {
            return Err(EnvelopeError::InvalidTime);
        }
        if !sustain_level.is_finite() || !(0.0..=1.0).contains(&sustain_level) {
            return Err(EnvelopeError::InvalidSustain);
        }
        Ok(Self {
            attack_seconds,
            decay_seconds,
            sustain_level,
            release_seconds,
        })
    }

    pub fn validate(self) -> Result<Self, EnvelopeError> {
        Self::new(
            self.attack_seconds,
            self.decay_seconds,
            self.sustain_level,
            self.release_seconds,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Live-controllable ADSR. Stage time is kept independently from its current
/// parameters, so smoothed time changes affect held and released notes without
/// restarting them.
#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    sample_rate: f32,
    config: AdsrConfig,
    stage: Stage,
    level: f32,
    stage_samples: u64,
    stage_start_level: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32, config: AdsrConfig) -> Result<Self, EnvelopeError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EnvelopeError::InvalidSampleRate);
        }
        Ok(Self {
            sample_rate,
            config: config.validate()?,
            stage: Stage::Idle,
            level: 0.0,
            stage_samples: 0,
            stage_start_level: 0.0,
        })
    }

    pub fn set_config(&mut self, config: AdsrConfig) {
        if let Ok(config) = config.validate() {
            self.config = config;
        }
    }

    pub fn note_on(&mut self) {
        self.stage_start_level = self.level;
        self.stage_samples = 0;
        self.stage = Stage::Attack;
    }

    pub fn restart(&mut self) {
        self.level = 0.0;
        self.note_on();
    }

    pub fn note_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage_start_level = self.level;
            self.stage_samples = 0;
            self.stage = Stage::Release;
        }
    }

    pub fn reset(&mut self) {
        self.stage = Stage::Idle;
        self.level = 0.0;
        self.stage_samples = 0;
        self.stage_start_level = 0.0;
    }

    #[inline]
    pub fn advance(&mut self) -> f32 {
        self.stage_samples = self.stage_samples.saturating_add(1);
        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                let progress =
                    self.stage_samples as f32 / (self.config.attack_seconds * self.sample_rate);
                self.level = self.stage_start_level + progress * (1.0 - self.stage_start_level);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                    self.stage_samples = 0;
                }
            }
            Stage::Decay => {
                let progress =
                    self.stage_samples as f32 / (self.config.decay_seconds * self.sample_rate);
                self.level = 1.0 - progress * (1.0 - self.config.sustain_level);
                if self.level <= self.config.sustain_level {
                    self.level = self.config.sustain_level;
                    self.stage = Stage::Sustain;
                    self.stage_samples = 0;
                }
            }
            Stage::Sustain => self.level = self.config.sustain_level,
            Stage::Release => {
                let progress =
                    self.stage_samples as f32 / (self.config.release_seconds * self.sample_rate);
                self.level = self.stage_start_level * (1.0 - progress);
                if self.level <= 0.0 {
                    self.reset();
                }
            }
        }
        self.level
    }

    pub const fn level(&self) -> f32 {
        self.level
    }

    pub fn is_idle(&self) -> bool {
        self.stage == Stage::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_adsr_times_and_sustain_change_active_notes() {
        let mut envelope =
            Adsr::new(1_000.0, AdsrConfig::new(0.010, 0.010, 0.5, 0.010).unwrap()).unwrap();
        envelope.note_on();
        for _ in 0..5 {
            envelope.advance();
        }
        assert!((envelope.level() - 0.5).abs() < 1.0e-6);
        envelope.set_config(AdsrConfig::new(0.005, 0.010, 0.25, 0.010).unwrap());
        assert_eq!(envelope.advance(), 1.0);
        for _ in 0..10 {
            envelope.advance();
        }
        assert!((envelope.level() - 0.25).abs() < 1.0e-6);
        envelope.set_config(AdsrConfig::new(0.005, 0.010, 0.8, 0.010).unwrap());
        assert_eq!(envelope.advance(), 0.8);
    }

    #[test]
    fn live_release_change_affects_an_already_released_note() {
        let mut envelope =
            Adsr::new(1_000.0, AdsrConfig::new(0.001, 0.001, 1.0, 0.100).unwrap()).unwrap();
        envelope.note_on();
        envelope.advance();
        envelope.note_off();
        for _ in 0..10 {
            envelope.advance();
        }
        assert!(envelope.level() > 0.8);
        envelope.set_config(AdsrConfig::new(0.001, 0.001, 1.0, 0.020).unwrap());
        for _ in 0..10 {
            envelope.advance();
        }
        assert!(envelope.is_idle());
    }
}
