use std::f32::consts::TAU;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, PartialEq)]
#[error("sample rate must be finite and positive")]
pub struct OscillatorError;

const INTEGRATED_TABLE_SIZE: usize = 2_048;

const fn build_integrated_saw_table() -> [f32; INTEGRATED_TABLE_SIZE + 1] {
    let mut table = [0.0; INTEGRATED_TABLE_SIZE + 1];
    let mut index = 0;
    while index <= INTEGRATED_TABLE_SIZE {
        let phase = index as f32 / INTEGRATED_TABLE_SIZE as f32;
        table[index] = phase * phase - phase;
        index += 1;
    }
    table
}

const fn build_integrated_square_table() -> [f32; INTEGRATED_TABLE_SIZE + 1] {
    let mut table = [0.0; INTEGRATED_TABLE_SIZE + 1];
    let mut index = 0;
    while index <= INTEGRATED_TABLE_SIZE {
        let phase = index as f32 / INTEGRATED_TABLE_SIZE as f32;
        table[index] = if phase <= 0.5 { phase } else { 1.0 - phase };
        index += 1;
    }
    table
}

static INTEGRATED_SAW: [f32; INTEGRATED_TABLE_SIZE + 1] = build_integrated_saw_table();
static INTEGRATED_SQUARE: [f32; INTEGRATED_TABLE_SIZE + 1] = build_integrated_square_table();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OscillatorMethod {
    PolyBlep,
    IntegratedWavetable,
}

#[derive(Clone, Copy, Debug)]
pub struct BandlimitedOscillator {
    method: OscillatorMethod,
    phase: f32,
    phase_increment: f32,
    pitch_ratio: f32,
    previous_integrated_saw: f32,
    previous_integrated_square: f32,
    sample_rate: f32,
}

impl BandlimitedOscillator {
    pub fn new(sample_rate: f32, method: OscillatorMethod) -> Result<Self, OscillatorError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(OscillatorError);
        }
        Ok(Self {
            method,
            phase: 0.0,
            phase_increment: 0.0,
            pitch_ratio: 1.0,
            previous_integrated_saw: 0.0,
            previous_integrated_square: 0.0,
            sample_rate,
        })
    }

    pub fn set_frequency(&mut self, frequency_hz: f32) {
        let frequency_hz = if frequency_hz.is_finite() {
            frequency_hz.clamp(0.0, self.sample_rate * 0.5)
        } else {
            0.0
        };
        self.phase_increment = frequency_hz / self.sample_rate;
        self.reset();
    }

    pub(crate) fn set_pitch_ratio(&mut self, ratio: f32) {
        self.pitch_ratio = ratio;
    }

    const fn effective_increment(&self) -> f32 {
        let increment = self.phase_increment * self.pitch_ratio;
        if increment > 0.5 { 0.5 } else { increment }
    }

    pub const fn phase_increment(&self) -> f32 {
        self.effective_increment()
    }

    #[inline]
    pub fn sample(&mut self, shape: f32) -> f32 {
        if self.effective_increment() <= 0.0 {
            return 0.0;
        }
        let shape = if shape.is_finite() {
            shape.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (saw, square) = match self.method {
            OscillatorMethod::PolyBlep => self.poly_blep_pair(),
            OscillatorMethod::IntegratedWavetable => self.integrated_wavetable_pair(),
        };
        self.advance_phase();
        saw + shape * (square - saw)
    }

    #[inline]
    fn poly_blep_pair(self) -> (f32, f32) {
        let phase = wrap_phase(self.phase - 0.5 * self.effective_increment());
        let saw = 2.0 * phase - 1.0 - poly_blep(phase, self.effective_increment());
        let opposite_phase = wrap_phase(phase + 0.5);
        let square = if phase < 0.5 { 1.0 } else { -1.0 }
            + poly_blep(phase, self.effective_increment())
            - poly_blep(opposite_phase, self.effective_increment());
        (saw, square)
    }

    #[inline]
    fn integrated_wavetable_pair(&mut self) -> (f32, f32) {
        let integrated_saw = integrated_lookup(&INTEGRATED_SAW, self.phase);
        let integrated_square = integrated_lookup(&INTEGRATED_SQUARE, self.phase);
        let scale = self.effective_increment().recip();
        let saw = (integrated_saw - self.previous_integrated_saw) * scale;
        let square = (integrated_square - self.previous_integrated_square) * scale;
        self.previous_integrated_saw = integrated_saw;
        self.previous_integrated_square = integrated_square;
        (saw, square)
    }

    #[inline]
    fn advance_phase(&mut self) {
        self.phase += self.effective_increment();
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }

    pub fn reset(&mut self) {
        self.phase = self.effective_increment();
        self.previous_integrated_saw = INTEGRATED_SAW[0];
        self.previous_integrated_square = INTEGRATED_SQUARE[0];
    }
}

#[inline]
fn wrap_phase(mut phase: f32) -> f32 {
    if phase < 0.0 {
        phase += 1.0;
    } else if phase >= 1.0 {
        phase -= 1.0;
    }
    phase
}

#[inline]
fn poly_blep(phase: f32, phase_increment: f32) -> f32 {
    if phase < phase_increment {
        let normalized = phase / phase_increment;
        normalized + normalized - normalized * normalized - 1.0
    } else if phase > 1.0 - phase_increment {
        let normalized = (phase - 1.0) / phase_increment;
        normalized * normalized + normalized + normalized + 1.0
    } else {
        0.0
    }
}

#[inline]
fn integrated_lookup(table: &[f32; INTEGRATED_TABLE_SIZE + 1], phase: f32) -> f32 {
    let position = phase * INTEGRATED_TABLE_SIZE as f32;
    let index = position as usize;
    let fraction = position - index as f32;
    table[index] + fraction * (table[index + 1] - table[index])
}

#[derive(Clone, Copy, Debug)]
pub struct SineOscillator {
    frequency_hz: f32,
    pitch_ratio: f32,
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
            frequency_hz: 0.0,
            pitch_ratio: 1.0,
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
        self.frequency_hz = frequency_hz;
        self.update_rotation();
    }

    pub(crate) fn set_pitch_ratio(&mut self, ratio: f32) {
        if self.pitch_ratio != ratio {
            self.pitch_ratio = ratio;
            self.update_rotation();
        }
    }

    fn update_rotation(&mut self) {
        let angle = TAU * (self.frequency_hz * self.pitch_ratio).min(self.sample_rate * 0.5)
            / self.sample_rate;
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
    fn pitch_ratio_preserves_phase_and_respects_nyquist() {
        for method in [
            OscillatorMethod::PolyBlep,
            OscillatorMethod::IntegratedWavetable,
        ] {
            let mut osc = BandlimitedOscillator::new(48_000.0, method).unwrap();
            osc.set_frequency(440.0);
            for _ in 0..73 {
                osc.sample(0.0);
            }
            let phase = osc.phase;
            let history = osc.previous_integrated_saw;
            osc.set_pitch_ratio(crate::performance::ratio(2.0));
            assert_eq!(osc.phase, phase);
            assert_eq!(osc.previous_integrated_saw, history);
            assert!((osc.phase_increment() * 48_000.0 - 493.8833).abs() < 0.001);
            for _ in 0..1024 {
                assert!(osc.sample(0.0).is_finite());
            }
            osc.set_frequency(23_000.0);
            osc.set_pitch_ratio(crate::performance::ratio(2.5));
            assert_eq!(osc.phase_increment(), 0.5);
        }
        let mut sine = SineOscillator::new(48_000.0).unwrap();
        sine.set_frequency(440.0);
        for _ in 0..73 {
            sine.sample();
        }
        let phase = (sine.sine, sine.cosine);
        sine.set_pitch_ratio(crate::performance::ratio(-2.0));
        assert_eq!((sine.sine, sine.cosine), phase);
    }

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

    #[test]
    fn bandlimited_candidates_are_deterministic_finite_bounded_and_resettable() {
        for method in [
            OscillatorMethod::PolyBlep,
            OscillatorMethod::IntegratedWavetable,
        ] {
            for shape in [0.0, 0.5, 1.0] {
                let mut first = BandlimitedOscillator::new(48_000.0, method).unwrap();
                let mut second = BandlimitedOscillator::new(48_000.0, method).unwrap();
                first.set_frequency(4_186.01);
                second.set_frequency(4_186.01);
                let a: Vec<_> = (0..4_096).map(|_| first.sample(shape)).collect();
                let b: Vec<_> = (0..4_096).map(|_| second.sample(shape)).collect();
                assert_eq!(a, b, "method={method:?}, shape={shape}");
                assert!(
                    a.iter()
                        .all(|sample| sample.is_finite() && sample.abs() <= 1.5),
                    "method={method:?}, shape={shape}"
                );
                first.reset();
                second.reset();
                let reset_a: Vec<_> = (0..256).map(|_| first.sample(shape)).collect();
                let reset_b: Vec<_> = (0..256).map(|_| second.sample(shape)).collect();
                assert_eq!(reset_a, reset_b);
            }
        }
    }

    #[test]
    fn bandlimited_candidates_prepare_and_clamp_phase_increment() {
        for method in [
            OscillatorMethod::PolyBlep,
            OscillatorMethod::IntegratedWavetable,
        ] {
            let mut oscillator = BandlimitedOscillator::new(48_000.0, method).unwrap();
            oscillator.set_frequency(12_000.0);
            assert_eq!(oscillator.phase_increment(), 0.25);
            oscillator.set_frequency(f32::INFINITY);
            assert_eq!(oscillator.phase_increment(), 0.0);
            oscillator.set_frequency(96_000.0);
            assert_eq!(oscillator.phase_increment(), 0.5);
        }
        assert!(BandlimitedOscillator::new(0.0, OscillatorMethod::PolyBlep).is_err());
        assert!(
            BandlimitedOscillator::new(f32::NAN, OscillatorMethod::IntegratedWavetable).is_err()
        );
    }

    #[test]
    fn bandlimited_candidate_sample_paths_do_not_allocate() {
        for method in [
            OscillatorMethod::PolyBlep,
            OscillatorMethod::IntegratedWavetable,
        ] {
            let mut oscillator = BandlimitedOscillator::new(48_000.0, method).unwrap();
            oscillator.set_frequency(1_046.5);
            assert_no_alloc::assert_no_alloc(|| {
                let mut sum = 0.0;
                for _ in 0..4_096 {
                    sum += oscillator.sample(0.37);
                }
                std::hint::black_box(sum);
            });
        }
    }
}
