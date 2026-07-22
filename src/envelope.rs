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

#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    sample_rate: f32,
    config: AdsrConfig,
    stage: Stage,
    level: f32,
    release_step: f32,
}

impl Adsr {
    pub fn new(sample_rate: f32, config: AdsrConfig) -> Result<Self, EnvelopeError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EnvelopeError::InvalidSampleRate);
        }
        let config = config.validate()?;
        Ok(Self {
            sample_rate,
            config,
            stage: Stage::Idle,
            level: 0.0,
            release_step: 0.0,
        })
    }

    pub fn note_on(&mut self) {
        self.stage = Stage::Attack;
    }

    pub fn restart(&mut self) {
        self.level = 0.0;
        self.release_step = 0.0;
        self.stage = Stage::Attack;
    }

    pub fn note_off(&mut self) {
        if self.stage != Stage::Idle {
            self.release_step = self.level / (self.config.release_seconds * self.sample_rate);
            self.stage = Stage::Release;
        }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                self.level += 1.0 / (self.config.attack_seconds * self.sample_rate);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level -= (1.0 - self.config.sustain_level)
                    / (self.config.decay_seconds * self.sample_rate);
                if self.level <= self.config.sustain_level {
                    self.level = self.config.sustain_level;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = self.config.sustain_level,
            Stage::Release => {
                self.level -= self.release_step;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
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
    fn envelope_traverses_attack_decay_sustain_and_release() {
        let config = AdsrConfig::new(0.002, 0.002, 0.5, 0.002).unwrap();
        let mut envelope = Adsr::new(1_000.0, config).unwrap();
        assert_eq!(envelope.next(), 0.0);
        envelope.note_on();
        assert!(envelope.next() > 0.0);
        for _ in 0..8 {
            envelope.next();
        }
        assert!((envelope.level() - 0.5).abs() < 1.0e-6);
        envelope.note_off();
        for _ in 0..8 {
            envelope.next();
        }
        assert_eq!(envelope.level(), 0.0);
        assert!(envelope.is_idle());
    }

    #[test]
    fn envelope_rejects_invalid_configuration() {
        assert!(AdsrConfig::new(0.0, 0.1, 0.5, 0.1).is_err());
        assert!(AdsrConfig::new(0.1, 0.1, 1.1, 0.1).is_err());
        let valid = AdsrConfig::new(0.1, 0.1, 0.5, 0.1).unwrap();
        assert!(Adsr::new(f32::NAN, valid).is_err());
    }
}
