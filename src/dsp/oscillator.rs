use std::f32::consts::TAU;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, PartialEq)]
#[error("sample rate must be finite and positive")]
pub struct OscillatorError;

#[derive(Clone, Copy, Debug)]
pub struct SineOscillator {
    phase: f32,
    sample_rate: f32,
}

impl SineOscillator {
    pub fn new(sample_rate: f32) -> Result<Self, OscillatorError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(OscillatorError);
        }
        Ok(Self {
            phase: 0.0,
            sample_rate,
        })
    }

    #[inline]
    pub fn next(&mut self, frequency_hz: f32) -> f32 {
        let sample = (self.phase * TAU).sin();
        let increment = if frequency_hz.is_finite() {
            frequency_hz.max(0.0) / self.sample_rate
        } else {
            0.0
        };
        self.phase = (self.phase + increment).fract();
        sample
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oscillator_is_deterministic_bounded_and_resettable() {
        let mut first = SineOscillator::new(48_000.0).unwrap();
        let mut second = SineOscillator::new(48_000.0).unwrap();
        let a: Vec<_> = (0..1_000).map(|_| first.next(440.0)).collect();
        let b: Vec<_> = (0..1_000).map(|_| second.next(440.0)).collect();
        assert_eq!(a, b);
        assert!(
            a.iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
        );
        first.reset();
        assert_eq!(first.next(440.0), a[0]);
    }

    #[test]
    fn oscillator_rejects_invalid_sample_rates() {
        assert!(SineOscillator::new(0.0).is_err());
        assert!(SineOscillator::new(f32::NAN).is_err());
    }
}
