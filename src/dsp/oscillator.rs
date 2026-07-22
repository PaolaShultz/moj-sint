use std::f32::consts::TAU;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, PartialEq)]
#[error("sample rate must be finite and positive")]
pub struct OscillatorError;

#[derive(Clone, Copy, Debug)]
pub struct SineOscillator {
    sine: f32,
    cosine: f32,
    rotation_sine: f32,
    rotation_cosine: f32,
    sample_rate: f32,
    samples_since_normalize: u16,
}

impl SineOscillator {
    pub fn new(sample_rate: f32) -> Result<Self, OscillatorError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(OscillatorError);
        }
        Ok(Self {
            sine: 0.0,
            cosine: 1.0,
            rotation_sine: 0.0,
            rotation_cosine: 1.0,
            sample_rate,
            samples_since_normalize: 0,
        })
    }

    pub fn set_frequency(&mut self, frequency_hz: f32) {
        let frequency_hz = if frequency_hz.is_finite() {
            frequency_hz.clamp(0.0, self.sample_rate * 0.5)
        } else {
            0.0
        };
        let angle = TAU * frequency_hz / self.sample_rate;
        (self.rotation_sine, self.rotation_cosine) = angle.sin_cos();
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        let sample = self.sine;
        let sine = self.sine * self.rotation_cosine + self.cosine * self.rotation_sine;
        let cosine = self.cosine * self.rotation_cosine - self.sine * self.rotation_sine;
        self.sine = sine;
        self.cosine = cosine;
        self.samples_since_normalize += 1;
        if self.samples_since_normalize == 1_024 {
            let scale = (self.sine * self.sine + self.cosine * self.cosine)
                .sqrt()
                .recip();
            self.sine *= scale;
            self.cosine *= scale;
            self.samples_since_normalize = 0;
        }
        sample
    }

    pub fn reset(&mut self) {
        self.sine = 0.0;
        self.cosine = 1.0;
        self.samples_since_normalize = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oscillator_is_deterministic_bounded_and_resettable() {
        let mut first = SineOscillator::new(48_000.0).unwrap();
        let mut second = SineOscillator::new(48_000.0).unwrap();
        first.set_frequency(440.0);
        second.set_frequency(440.0);
        let a: Vec<_> = (0..1_000).map(|_| first.sample()).collect();
        let b: Vec<_> = (0..1_000).map(|_| second.sample()).collect();
        assert_eq!(a, b);
        assert!(
            a.iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
        );
        first.reset();
        assert_eq!(first.sample(), a[0]);
    }

    #[test]
    fn oscillator_rejects_invalid_sample_rates() {
        assert!(SineOscillator::new(0.0).is_err());
        assert!(SineOscillator::new(f32::NAN).is_err());
    }
}
