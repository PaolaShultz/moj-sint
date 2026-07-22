use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroId {
    Evolve,
    Shape,
    Color,
    Edge,
    Couple,
    Motion,
    Depth,
    Width,
    Space,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl MacroId {
    pub const ALL: [Self; 13] = [
        Self::Evolve,
        Self::Shape,
        Self::Color,
        Self::Edge,
        Self::Couple,
        Self::Motion,
        Self::Depth,
        Self::Width,
        Self::Space,
        Self::Attack,
        Self::Decay,
        Self::Sustain,
        Self::Release,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Evolve => "evolve",
            Self::Shape => "shape",
            Self::Color => "color",
            Self::Edge => "edge",
            Self::Couple => "couple",
            Self::Motion => "motion",
            Self::Depth => "depth",
            Self::Width => "width",
            Self::Space => "space",
            Self::Attack => "attack",
            Self::Decay => "decay",
            Self::Sustain => "sustain",
            Self::Release => "release",
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
#[error("normalized value must be finite and between 0 and 1, got {0}")]
pub struct NormalizedError(pub f32);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f32", into = "f32")]
pub struct Normalized(f32);

impl Normalized {
    pub fn new(value: f32) -> Result<Self, NormalizedError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(NormalizedError(value))
        }
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for Normalized {
    type Error = NormalizedError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Normalized> for f32 {
    fn from(value: Normalized) -> Self {
        value.get()
    }
}

pub fn envelope_seconds(value: Normalized) -> f32 {
    const MIN_SECONDS: f32 = 0.001;
    const MAX_SECONDS: f32 = 20.0;
    MIN_SECONDS * (MAX_SECONDS / MIN_SECONDS).powf(value.get())
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum SmootherError {
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
    #[error("smoothing time must be finite and positive")]
    InvalidTime,
}

#[derive(Clone, Copy, Debug)]
pub struct Smoother {
    current: f32,
    target: f32,
    coefficient: f32,
}

impl Smoother {
    pub fn new(initial: f32, sample_rate: f32, seconds: f32) -> Result<Self, SmootherError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(SmootherError::InvalidSampleRate);
        }
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err(SmootherError::InvalidTime);
        }
        let coefficient = (-1.0 / (sample_rate * seconds)).exp();
        Ok(Self {
            current: initial,
            target: initial,
            coefficient,
        })
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn next(&mut self) -> f32 {
        self.current = self.target + self.coefficient * (self.current - self.target);
        self.current
    }

    pub const fn current(&self) -> f32 {
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_exactly_thirteen_unique_macros() {
        assert_eq!(MacroId::ALL.len(), 13);
        let mut names = MacroId::ALL.map(MacroId::as_str).to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 13);
    }

    #[test]
    fn normalized_rejects_non_finite_and_out_of_range_values() {
        for invalid in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            assert!(Normalized::new(invalid).is_err());
        }
        assert_eq!(Normalized::new(0.0).unwrap().get(), 0.0);
        assert_eq!(Normalized::new(1.0).unwrap().get(), 1.0);
    }

    #[test]
    fn envelope_time_mapping_is_exponential() {
        let low = envelope_seconds(Normalized::new(0.0).unwrap());
        let middle = envelope_seconds(Normalized::new(0.5).unwrap());
        let high = envelope_seconds(Normalized::new(1.0).unwrap());
        assert!((low - 0.001).abs() < 1.0e-6);
        assert!((high - 20.0).abs() < 1.0e-3);
        assert!(middle < (low + high) * 0.25);
    }

    #[test]
    fn smoother_converges_without_overshoot() {
        let mut smoother = Smoother::new(0.0, 48_000.0, 0.005).unwrap();
        smoother.set_target(1.0);
        let mut previous = 0.0;
        for _ in 0..4_800 {
            let current = smoother.next();
            assert!(current >= previous && current <= 1.0);
            previous = current;
        }
        assert!((smoother.current() - 1.0).abs() < 1.0e-4);
    }
}
