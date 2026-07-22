use std::f32::consts::TAU;
use thiserror::Error;

const HALF: f32 = 0.5;
const SQRT_THREE_OVER_TWO: f32 = 0.866_025_4;

#[derive(Clone, Copy, Debug, Error, PartialEq)]
#[error("sample rate must be finite and positive")]
pub struct ThreePhaseError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HarmonicSample {
    pub fundamental: f32,
    pub third: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ThreePhaseBank {
    sine: f32,
    cosine: f32,
    rotation_sine: f32,
    rotation_cosine: f32,
    third_harmonic_weight: f32,
    sample_rate: f32,
    samples_since_normalize: u16,
}

impl ThreePhaseBank {
    pub fn new(sample_rate: f32) -> Result<Self, ThreePhaseError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(ThreePhaseError);
        }
        Ok(Self {
            sine: 0.0,
            cosine: 1.0,
            rotation_sine: 0.0,
            rotation_cosine: 1.0,
            third_harmonic_weight: 1.0,
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
        let phase_increment = frequency_hz / self.sample_rate;
        const TAPER_START: f32 = 0.4 / 3.0;
        const TAPER_END: f32 = 0.48 / 3.0;
        self.third_harmonic_weight =
            ((TAPER_END - phase_increment) / (TAPER_END - TAPER_START)).clamp(0.0, 1.0);
        self.reset();
    }

    pub const fn third_harmonic_weight(&self) -> f32 {
        self.third_harmonic_weight
    }

    #[inline]
    pub fn sample(&mut self) -> HarmonicSample {
        let taps = self.next_taps();
        let cubic_sum =
            taps[0] * taps[0] * taps[0] + taps[1] * taps[1] * taps[1] + taps[2] * taps[2] * taps[2];
        let raw_third = (-4.0 / 3.0) * cubic_sum;
        HarmonicSample {
            fundamental: taps[0],
            third: taps[0] + self.third_harmonic_weight * (raw_third - taps[0]),
        }
    }

    #[inline]
    fn next_taps(&mut self) -> [f32; 3] {
        let taps = [
            self.sine,
            -HALF * self.sine + SQRT_THREE_OVER_TWO * self.cosine,
            -HALF * self.sine - SQRT_THREE_OVER_TWO * self.cosine,
        ];
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
        taps
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
    fn derives_three_taps_that_cancel_linearly() {
        let mut bank = ThreePhaseBank::new(48_000.0).unwrap();
        bank.set_frequency(440.0);
        for _ in 0..4_096 {
            let taps = bank.next_taps();
            assert!((taps[0] + taps[1] + taps[2]).abs() < 2.0e-6, "{taps:?}");
            assert!(
                taps.into_iter()
                    .all(|sample| sample.is_finite() && sample.abs() <= 1.000_01)
            );
        }
    }

    #[test]
    fn cubic_sum_selects_a_unit_third_harmonic() {
        let mut bank = ThreePhaseBank::new(48_000.0).unwrap();
        bank.set_frequency(1_000.0);
        for index in 0..96 {
            let sample = bank.sample();
            let expected = (std::f32::consts::TAU * 3.0 * index as f32 / 48.0).sin();
            assert!(
                (sample.third - expected).abs() < 2.0e-4,
                "index={index} sample={sample:?} expected={expected}"
            );
        }
    }

    #[test]
    fn sampling_is_deterministic_finite_bounded_resettable_and_allocation_free() {
        let mut first = ThreePhaseBank::new(48_000.0).unwrap();
        let mut second = ThreePhaseBank::new(48_000.0).unwrap();
        first.set_frequency(1_046.5);
        second.set_frequency(1_046.5);
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..4_096 {
                let a = first.sample();
                let b = second.sample();
                assert_eq!(a, b);
                assert!(a.fundamental.is_finite() && a.fundamental.abs() <= 1.000_01);
                assert!(a.third.is_finite() && a.third.abs() <= 1.000_1);
            }
        });
        first.reset();
        second.reset();
        assert_eq!(first.sample(), second.sample());
        assert!(ThreePhaseBank::new(0.0).is_err());
        assert!(ThreePhaseBank::new(f32::NAN).is_err());
    }

    #[test]
    fn high_notes_fall_back_to_the_fundamental_before_the_third_aliases() {
        let mut bank = ThreePhaseBank::new(48_000.0).unwrap();
        bank.set_frequency(10_000.0);
        for _ in 0..256 {
            let sample = bank.sample();
            assert!(
                (sample.third - sample.fundamental).abs() < 1.0e-6,
                "{sample:?}"
            );
        }
    }
}
