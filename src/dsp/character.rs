use std::f32::consts::TAU;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CharacterMethod {
    Direct,
    Adaa1,
    Oversampled2x,
}

impl CharacterMethod {
    pub const ALL: [Self; 3] = [Self::Direct, Self::Adaa1, Self::Oversampled2x];
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
#[error("sample rate must be finite and positive")]
pub struct CharacterError;

#[derive(Clone, Copy, Debug)]
pub struct CharacterLayer {
    waveshaper: CharacterWaveshaper,
    lowpass: [f32; 4],
    lowpass_coefficient: f32,
    previous_generated: f32,
    dc_output: f32,
    dc_coefficient: f32,
    return_highpass_lowpass: [f32; 2],
    return_highpass_coefficient: f32,
    sample_rate: f32,
}

impl CharacterLayer {
    pub fn new(sample_rate: f32, method: CharacterMethod) -> Result<Self, CharacterError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(CharacterError);
        }
        let cutoff_hz = 6_000.0_f32.min(0.2 * sample_rate);
        let lowpass_coefficient = 1.0 - (-TAU * cutoff_hz / sample_rate).exp();
        let dc_coefficient = (-TAU * 10.0 / sample_rate).exp();
        let return_highpass_coefficient = 1.0 - (-TAU * 20.0 / sample_rate).exp();
        Ok(Self {
            waveshaper: CharacterWaveshaper::new(method),
            lowpass: [0.0; 4],
            lowpass_coefficient,
            previous_generated: 0.0,
            dc_output: 0.0,
            dc_coefficient,
            return_highpass_lowpass: [0.0; 2],
            return_highpass_coefficient,
            sample_rate,
        })
    }

    pub fn set_frequency(&mut self, frequency_hz: f32) {
        let frequency_hz = if frequency_hz.is_finite() {
            frequency_hz.clamp(0.0, 0.5 * self.sample_rate)
        } else {
            0.0
        };
        let cutoff_hz = (2.5 * frequency_hz).clamp(20.0, 6_000.0_f32.min(0.2 * self.sample_rate));
        self.return_highpass_coefficient = 1.0 - (-TAU * cutoff_hz / self.sample_rate).exp();
    }

    #[inline]
    pub fn sample(&mut self, dry: f32, edge: f32, couple: f32) -> f32 {
        let dry = finite_clamped(dry, -2.0, 2.0);
        let edge = finite_clamped(edge, 0.0, 1.0);
        let couple = finite_clamped(couple, 0.0, 1.0);
        let mut branch = dry;
        for state in &mut self.lowpass {
            *state += self.lowpass_coefficient * (branch - *state);
            branch = *state;
        }

        let generated = self.waveshaper.sample(branch, edge);

        self.dc_output = generated - self.previous_generated + self.dc_coefficient * self.dc_output;
        self.previous_generated = generated;
        let mut character = self.dc_output;
        for lowpass in &mut self.return_highpass_lowpass {
            *lowpass += self.return_highpass_coefficient * (character - *lowpass);
            character -= *lowpass;
        }
        let intensity = 1.0 - (1.0 - edge) * (1.0 - edge) * (1.0 - edge);
        let output = dry - 1.2 * couple * intensity * character;
        if output.is_finite() { output } else { 0.0 }
    }

    pub fn reset(&mut self) {
        self.lowpass = [0.0; 4];
        self.waveshaper.reset();
        self.previous_generated = 0.0;
        self.dc_output = 0.0;
        self.return_highpass_lowpass = [0.0; 2];
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CharacterWaveshaper {
    method: CharacterMethod,
    previous_branch: f32,
    previous_driven: f32,
}

impl CharacterWaveshaper {
    pub(crate) const fn new(method: CharacterMethod) -> Self {
        Self {
            method,
            previous_branch: 0.0,
            previous_driven: 0.0,
        }
    }

    #[inline]
    pub(crate) fn sample(&mut self, branch: f32, edge: f32) -> f32 {
        let edge = finite_clamped(edge, 0.0, 1.0);
        let drive = 1.0 + edge * edge;
        let driven = drive * branch;
        let generated = match self.method {
            CharacterMethod::Direct => soft_clip(driven) / drive - branch,
            CharacterMethod::Adaa1 => {
                let difference = driven - self.previous_driven;
                let clipped = if difference.abs() > 1.0e-5 {
                    (soft_clip_antiderivative(driven)
                        - soft_clip_antiderivative(self.previous_driven))
                        / difference
                } else {
                    soft_clip(0.5 * (driven + self.previous_driven))
                };
                clipped / drive - 0.5 * (branch + self.previous_branch)
            }
            CharacterMethod::Oversampled2x => {
                let midpoint = 0.5 * (branch + self.previous_branch);
                let midpoint_generated = soft_clip(drive * midpoint) / drive - midpoint;
                let current_generated = soft_clip(driven) / drive - branch;
                0.5 * (midpoint_generated + current_generated)
            }
        };
        self.previous_branch = branch;
        self.previous_driven = driven;
        generated
    }

    pub(crate) fn reset(&mut self) {
        self.previous_branch = 0.0;
        self.previous_driven = 0.0;
    }
}

#[inline]
fn finite_clamped(value: f32, minimum: f32, maximum: f32) -> f32 {
    if value.is_finite() {
        value.clamp(minimum, maximum)
    } else {
        0.0
    }
}

#[inline]
fn soft_clip(value: f32) -> f32 {
    if value > 1.0 {
        2.0 / 3.0
    } else if value < -1.0 {
        -2.0 / 3.0
    } else {
        value - value * value * value / 3.0
    }
}

#[inline]
fn soft_clip_antiderivative(value: f32) -> f32 {
    if value.abs() <= 1.0 {
        let squared = value * value;
        0.5 * squared - squared * squared / 12.0
    } else {
        (2.0 / 3.0) * value.abs() - 0.25
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_sample_rates() {
        assert!(CharacterLayer::new(0.0, CharacterMethod::Direct).is_err());
        assert!(CharacterLayer::new(f32::NAN, CharacterMethod::Adaa1).is_err());
    }

    #[test]
    fn cubic_soft_clip_and_antiderivative_are_continuous_and_bounded() {
        assert_eq!(soft_clip(0.0), 0.0);
        assert!((soft_clip(0.5) - (0.5 - 0.125 / 3.0)).abs() < 1.0e-7);
        assert!((soft_clip(1.0) - 2.0 / 3.0).abs() < 1.0e-7);
        assert!((soft_clip(2.0) - 2.0 / 3.0).abs() < 1.0e-7);
        assert!((soft_clip(-2.0) + 2.0 / 3.0).abs() < 1.0e-7);

        assert_eq!(soft_clip_antiderivative(0.0), 0.0);
        assert!((soft_clip_antiderivative(1.0) - 5.0 / 12.0).abs() < 1.0e-7);
        assert!((soft_clip_antiderivative(-1.0) - 5.0 / 12.0).abs() < 1.0e-7);
        assert!((soft_clip_antiderivative(2.0) - 13.0 / 12.0).abs() < 1.0e-7);
        assert!((soft_clip_antiderivative(-2.0) - 13.0 / 12.0).abs() < 1.0e-7);
    }

    #[test]
    fn zero_return_is_sample_exact_and_independent_of_edge() {
        for method in CharacterMethod::ALL {
            for edge in [0.0, 0.5, 1.0] {
                let mut layer = CharacterLayer::new(48_000.0, method).unwrap();
                for index in 0..4_096 {
                    let dry = ((index as f32 * 0.017).sin() * 0.8).clamp(-1.0, 1.0);
                    assert_eq!(layer.sample(dry, edge, 0.0), dry);
                }
            }
        }
    }

    #[test]
    fn methods_are_deterministic_finite_bounded_resettable_and_distinct() {
        let mut first_outputs = Vec::new();
        for method in CharacterMethod::ALL {
            let mut first = CharacterLayer::new(48_000.0, method).unwrap();
            let mut second = CharacterLayer::new(48_000.0, method).unwrap();
            let mut output = Vec::with_capacity(8_192);
            for index in 0..8_192 {
                let phase = (index as f32 * 440.0 / 48_000.0).fract();
                let dry = 2.0 * phase - 1.0;
                let a = first.sample(dry, 0.8, 1.0);
                let b = second.sample(dry, 0.8, 1.0);
                assert_eq!(a, b);
                assert!(
                    a.is_finite() && a.abs() <= 2.0,
                    "method={method:?} sample={a}"
                );
                output.push(a);
            }
            first.reset();
            second.reset();
            assert_eq!(first.sample(0.25, 0.8, 1.0), second.sample(0.25, 0.8, 1.0));
            first_outputs.push(output);
        }
        assert_ne!(first_outputs[0], first_outputs[1]);
        assert_ne!(first_outputs[1], first_outputs[2]);
    }

    #[test]
    fn dc_blocker_removes_the_generated_constant_component() {
        let mut layer = CharacterLayer::new(48_000.0, CharacterMethod::Adaa1).unwrap();
        let mut character = 0.0_f32;
        for _ in 0..240_000 {
            character = layer.sample(0.4, 1.0, 1.0) - 0.4;
        }
        assert!(character.abs() < 1.0e-5, "character={character}");
    }

    #[test]
    fn sample_paths_do_not_allocate() {
        for method in CharacterMethod::ALL {
            let mut layer = CharacterLayer::new(48_000.0, method).unwrap();
            assert_no_alloc::assert_no_alloc(|| {
                let mut sum = 0.0;
                for index in 0..8_192 {
                    let dry = (index as f32 * 0.013).sin();
                    sum += layer.sample(dry, 0.65, 0.7);
                }
                std::hint::black_box(sum);
            });
        }
    }

    #[test]
    fn note_tracked_return_keeps_the_dry_fundamental_and_adds_a_third() {
        let sample_rate = 48_000.0;
        let frequency = 440.0;
        let mut layer = CharacterLayer::new(sample_rate, CharacterMethod::Direct).unwrap();
        layer.set_frequency(frequency);
        let mut fundamental_sine = 0.0_f64;
        let mut fundamental_cosine = 0.0_f64;
        let mut third_sine = 0.0_f64;
        let mut third_cosine = 0.0_f64;
        let sample_count = 48_000;
        for index in 0..sample_count + 4_800 {
            let phase =
                std::f64::consts::TAU * frequency as f64 * index as f64 / sample_rate as f64;
            let dry = (0.8 * phase.sin()) as f32;
            let output = layer.sample(dry, 1.0, 1.0);
            if index >= 4_800 {
                fundamental_sine += f64::from(output) * phase.sin();
                fundamental_cosine += f64::from(output) * phase.cos();
                third_sine += f64::from(output) * (3.0 * phase).sin();
                third_cosine += f64::from(output) * (3.0 * phase).cos();
            }
        }
        let scale = 2.0 / sample_count as f64;
        let fundamental = scale * fundamental_sine.hypot(fundamental_cosine);
        let third = scale * third_sine.hypot(third_cosine);
        assert!(fundamental > 0.7, "fundamental={fundamental} third={third}");
        assert!(third > 0.05, "fundamental={fundamental} third={third}");
    }
}
