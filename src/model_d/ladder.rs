use super::ModelDError;

const OVERSAMPLE_FACTOR: usize = 4;
const CUTOFF_TABLE_SIZE: usize = 2_049;
const MIN_CUTOFF_HZ: f32 = 20.0;
const MAX_CUTOFF_HZ: f32 = 8_000.0;
const MAX_CUTOFF_RATIO: f32 = 0.2;
const MIN_DRIVE: f32 = 1.0;
const MAX_DRIVE: f32 = 4.0;
const MAX_RESONANCE_FEEDBACK: f32 = 3.5;
// Four equal one-pole stages reach -3 dB at
// `stage_cutoff * sqrt(2^(1/4) - 1)`.
const FOUR_POLE_CUTOFF_CORRECTION: f32 = 2.298_959;
const MAX_INPUT: f32 = 16.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LadderMode {
    Linear,
    Nonlinear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LadderConfig {
    /// Resonance control. Finite values are clamped to the inclusive 0 to 1 range.
    pub resonance: f32,
    /// Nonlinear-stage drive. Finite values are clamped to the inclusive 1 to 4 range.
    pub drive: f32,
    pub mode: LadderMode,
}

/// A prepared scalar four-stage low-pass ladder with fixed four-times oversampling.
///
/// `cutoff_normalized` maps exponentially from 20 Hz to the lower of 8 kHz
/// and one fifth of the host sample rate. The sample path uses only fixed-size
/// arithmetic and table interpolation.
#[derive(Clone, Debug)]
pub struct ModelDLadder {
    coefficient_table: [f32; CUTOFF_TABLE_SIZE],
    states: [f32; 4],
    resonance_feedback: f32,
    drive: f32,
    mode: LadderMode,
}

impl ModelDLadder {
    pub fn new(sample_rate: f32, config: LadderConfig) -> Result<Self, ModelDError> {
        let internal_sample_rate = sample_rate * OVERSAMPLE_FACTOR as f32;
        let maximum_cutoff = MAX_CUTOFF_HZ.min(MAX_CUTOFF_RATIO * sample_rate);
        if !sample_rate.is_finite()
            || sample_rate <= 0.0
            || !internal_sample_rate.is_finite()
            || maximum_cutoff <= MIN_CUTOFF_HZ
        {
            return Err(ModelDError::InvalidSampleRate);
        }
        if !config.resonance.is_finite() || !config.drive.is_finite() {
            return Err(ModelDError::InvalidConfig);
        }

        let cutoff_ratio = maximum_cutoff / MIN_CUTOFF_HZ;
        let mut coefficient_table = [0.0; CUTOFF_TABLE_SIZE];
        for (index, coefficient) in coefficient_table.iter_mut().enumerate() {
            let normalized = index as f32 / (CUTOFF_TABLE_SIZE - 1) as f32;
            let requested_cutoff = MIN_CUTOFF_HZ * cutoff_ratio.powf(normalized);
            let stage_cutoff = requested_cutoff * FOUR_POLE_CUTOFF_CORRECTION;
            *coefficient =
                1.0 - (-core::f32::consts::TAU * stage_cutoff / internal_sample_rate).exp();
        }

        Ok(Self {
            coefficient_table,
            states: [0.0; 4],
            resonance_feedback: config.resonance.clamp(0.0, 1.0) * MAX_RESONANCE_FEEDBACK,
            drive: config.drive.clamp(MIN_DRIVE, MAX_DRIVE),
            mode: config.mode,
        })
    }

    #[inline]
    pub fn sample(&mut self, input: f32, cutoff_normalized: f32) -> f32 {
        if !input.is_finite() || !cutoff_normalized.is_finite() {
            self.reset();
            return 0.0;
        }

        let table_position = cutoff_normalized.clamp(0.0, 1.0) * (CUTOFF_TABLE_SIZE - 1) as f32;
        let lower_index = (table_position as usize).min(CUTOFF_TABLE_SIZE - 2);
        let fraction = table_position - lower_index as f32;
        let lower = self.coefficient_table[lower_index];
        let coefficient = lower + fraction * (self.coefficient_table[lower_index + 1] - lower);
        let input = input.clamp(-MAX_INPUT, MAX_INPUT);

        let mut decimator_sum = 0.0;
        for _ in 0..OVERSAMPLE_FACTOR {
            let feedback_output = match self.mode {
                LadderMode::Linear => self.states[3],
                LadderMode::Nonlinear => bounded_odd(self.states[3]),
            };
            let differential = input - self.resonance_feedback * feedback_output;
            let mut stage_input = match self.mode {
                LadderMode::Linear => differential,
                LadderMode::Nonlinear => bounded_odd(self.drive * differential) / self.drive,
            };

            for state in &mut self.states {
                *state += coefficient * (stage_input - *state);
                stage_input = match self.mode {
                    LadderMode::Linear => *state,
                    LadderMode::Nonlinear => bounded_odd(*state),
                };
            }
            decimator_sum += stage_input;
        }

        let output = decimator_sum / OVERSAMPLE_FACTOR as f32;
        if output.is_finite() {
            output
        } else {
            self.reset();
            0.0
        }
    }

    pub fn reset(&mut self) {
        self.states = [0.0; 4];
    }
}

#[inline]
fn bounded_odd(input: f32) -> f32 {
    let bounded = input.clamp(-1.0, 1.0);
    bounded * (1.5 - 0.5 * bounded * bounded) / 1.5
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::{LadderConfig, LadderMode, ModelDLadder};

    const SAMPLE_RATE: f32 = 48_000.0;
    const MIN_CUTOFF_HZ: f32 = 20.0;
    const MAX_CUTOFF_HZ: f32 = 8_000.0;

    fn config(mode: LadderMode) -> LadderConfig {
        LadderConfig {
            resonance: 0.0,
            drive: 1.0,
            mode,
        }
    }

    fn normalized_cutoff(cutoff_hz: f32) -> f32 {
        (cutoff_hz / MIN_CUTOFF_HZ).ln() / (MAX_CUTOFF_HZ / MIN_CUTOFF_HZ).ln()
    }

    fn response(
        cutoff_hz: f32,
        frequency_hz: f32,
        resonance: f32,
        drive: f32,
        mode: LadderMode,
    ) -> f32 {
        let mut ladder = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                resonance,
                drive,
                mode,
            },
        )
        .unwrap();
        let cutoff = normalized_cutoff(cutoff_hz);
        let phase_step = core::f32::consts::TAU * frequency_hz / SAMPLE_RATE;
        let mut phase = 0.0_f32;
        for _ in 0..8_192 {
            ladder.sample(0.05 * phase.sin(), cutoff);
            phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
        }

        let mut input_sine = 0.0_f64;
        let mut input_cosine = 0.0_f64;
        let mut output_sine = 0.0_f64;
        let mut output_cosine = 0.0_f64;
        for _ in 0..16_384 {
            let input = 0.05 * phase.sin();
            let output = ladder.sample(input, cutoff);
            let sine = f64::from(phase.sin());
            let cosine = f64::from(phase.cos());
            input_sine += f64::from(input) * sine;
            input_cosine += f64::from(input) * cosine;
            output_sine += f64::from(output) * sine;
            output_cosine += f64::from(output) * cosine;
            phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
        }
        let input_magnitude = input_sine.hypot(input_cosine);
        output_sine.hypot(output_cosine) as f32 / input_magnitude as f32
    }

    fn measured_cutoff_hz(target_hz: f32) -> f32 {
        let threshold = core::f32::consts::FRAC_1_SQRT_2;
        let mut lower = 0.72 * target_hz;
        let mut upper = 1.28 * target_hz;
        for _ in 0..12 {
            let middle = 0.5 * (lower + upper);
            if response(target_hz, middle, 0.0, 1.0, LadderMode::Linear) > threshold {
                lower = middle;
            } else {
                upper = middle;
            }
        }
        0.5 * (lower + upper)
    }

    fn harmonic_magnitude(samples: &[f32], harmonic: usize, period: usize) -> f64 {
        let mut sine = 0.0;
        let mut cosine = 0.0;
        for (index, sample) in samples.iter().enumerate() {
            let phase =
                core::f64::consts::TAU * harmonic as f64 * (index % period) as f64 / period as f64;
            sine += f64::from(*sample) * phase.sin();
            cosine += f64::from(*sample) * phase.cos();
        }
        sine.hypot(cosine)
    }

    #[test]
    fn rejects_invalid_sample_rates_and_nonfinite_configuration() {
        assert!(ModelDLadder::new(0.0, config(LadderMode::Linear)).is_err());
        assert!(ModelDLadder::new(f32::NAN, config(LadderMode::Linear)).is_err());
        assert!(ModelDLadder::new(f32::from_bits(1), config(LadderMode::Linear)).is_err());

        for resonance in [f32::NAN, f32::INFINITY] {
            assert!(
                ModelDLadder::new(
                    SAMPLE_RATE,
                    LadderConfig {
                        resonance,
                        ..config(LadderMode::Nonlinear)
                    }
                )
                .is_err()
            );
        }
        for drive in [f32::NAN, f32::INFINITY] {
            assert!(
                ModelDLadder::new(
                    SAMPLE_RATE,
                    LadderConfig {
                        drive,
                        ..config(LadderMode::Nonlinear)
                    }
                )
                .is_err()
            );
        }
    }

    #[test]
    fn finite_out_of_range_configuration_is_clamped_during_preparation() {
        let mut below = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                resonance: -10.0,
                drive: -10.0,
                mode: LadderMode::Nonlinear,
            },
        )
        .unwrap();
        let mut minimum = ModelDLadder::new(SAMPLE_RATE, config(LadderMode::Nonlinear)).unwrap();
        let mut above = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                resonance: 10.0,
                drive: 10.0,
                mode: LadderMode::Nonlinear,
            },
        )
        .unwrap();
        let mut maximum = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                resonance: 1.0,
                drive: 4.0,
                mode: LadderMode::Nonlinear,
            },
        )
        .unwrap();

        for index in 0..1_024 {
            let input = 0.25 * (core::f32::consts::TAU * index as f32 / 97.0).sin();
            assert_eq!(below.sample(input, 0.4), minimum.sample(input, 0.4));
            assert_eq!(above.sample(input, 0.4), maximum.sample(input, 0.4));
        }
    }

    #[test]
    fn reset_restores_deterministic_state_and_sampling_allocates_nothing() {
        let mut ladder = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                resonance: 0.72,
                drive: 2.5,
                mode: LadderMode::Nonlinear,
            },
        )
        .unwrap();
        let mut expected = [0.0_f32; 257];
        assert_no_alloc(|| {
            for (index, output) in expected.iter_mut().enumerate() {
                let input = 0.4 * (core::f32::consts::TAU * index as f32 / 71.0).sin();
                *output = ladder.sample(input, index as f32 / 256.0);
                assert!(output.is_finite());
            }
        });
        ladder.reset();
        assert_no_alloc(|| {
            for (index, expected) in expected.into_iter().enumerate() {
                let input = 0.4 * (core::f32::consts::TAU * index as f32 / 71.0).sin();
                assert_eq!(ladder.sample(input, index as f32 / 256.0), expected);
            }
        });
    }

    #[test]
    fn nonfinite_sample_inputs_are_silenced_without_poisoning_state() {
        let mut ladder = ModelDLadder::new(SAMPLE_RATE, config(LadderMode::Nonlinear)).unwrap();
        assert_eq!(ladder.sample(f32::NAN, 0.5), 0.0);
        assert_eq!(ladder.sample(0.5, f32::NAN), 0.0);
        for _ in 0..256 {
            assert!(ladder.sample(0.1, 0.5).is_finite());
        }
    }

    #[test]
    fn impulse_response_decays_to_silence() {
        let mut ladder = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                resonance: 0.75,
                drive: 2.0,
                mode: LadderMode::Nonlinear,
            },
        )
        .unwrap();
        let cutoff = normalized_cutoff(500.0);
        let first = ladder.sample(1.0, cutoff);
        let mut peak = first.abs();
        let mut tail_peak = 0.0_f32;
        for index in 1..(SAMPLE_RATE as usize * 3) {
            let output = ladder.sample(0.0, cutoff);
            peak = peak.max(output.abs());
            if index >= SAMPLE_RATE as usize * 2 {
                tail_peak = tail_peak.max(output.abs());
            }
        }
        assert!(peak > 1.0e-5, "peak={peak}");
        assert!(tail_peak < 1.0e-5, "tail_peak={tail_peak}");
    }

    #[test]
    fn measured_cutoff_is_monotonic_and_within_eight_percent_at_calibration_points() {
        let targets = [125.0_f32, 500.0, 2_000.0];
        let mut previous = 0.0;
        for target in targets {
            let measured = measured_cutoff_hz(target);
            let error = (measured - target).abs() / target;
            assert!(measured > previous, "target={target}, measured={measured}");
            assert!(
                error <= 0.08,
                "target={target}, measured={measured}, error={error}"
            );
            previous = measured;
        }
    }

    #[test]
    fn stop_band_slope_is_twenty_to_twenty_eight_db_per_octave() {
        let lower = response(250.0, 2_000.0, 0.0, 1.0, LadderMode::Linear);
        let upper = response(250.0, 4_000.0, 0.0, 1.0, LadderMode::Linear);
        let slope_db = 20.0 * (lower / upper).log10();
        assert!(
            (20.0..=28.0).contains(&slope_db),
            "lower={lower}, upper={upper}, slope_db={slope_db}"
        );
    }

    #[test]
    fn resonance_increases_the_peak_near_cutoff() {
        let peak = |resonance| {
            let passband = response(1_000.0, 100.0, resonance, 1.0, LadderMode::Linear);
            [1_000.0_f32, 1_500.0, 2_000.0, 2_300.0, 2_500.0, 3_000.0]
                .into_iter()
                .map(|frequency| response(1_000.0, frequency, resonance, 1.0, LadderMode::Linear))
                .fold(0.0_f32, f32::max)
                / passband
        };
        let low = peak(0.0);
        let middle = peak(0.45);
        let high = peak(0.8);
        assert!(middle > low * 1.05, "low={low}, middle={middle}");
        assert!(high > middle * 1.05, "middle={middle}, high={high}");
    }

    #[test]
    fn nonlinear_mode_generates_more_harmonics_than_linear_mode() {
        const PERIOD: usize = 128;
        const WARMUP: usize = PERIOD * 32;
        const ANALYSIS: usize = PERIOD * 64;
        let cutoff = normalized_cutoff(4_000.0);
        let mut linear = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                drive: 4.0,
                ..config(LadderMode::Linear)
            },
        )
        .unwrap();
        let mut nonlinear = ModelDLadder::new(
            SAMPLE_RATE,
            LadderConfig {
                drive: 4.0,
                ..config(LadderMode::Nonlinear)
            },
        )
        .unwrap();
        for index in 0..WARMUP {
            let input =
                0.8 * (core::f32::consts::TAU * (index % PERIOD) as f32 / PERIOD as f32).sin();
            linear.sample(input, cutoff);
            nonlinear.sample(input, cutoff);
        }
        let mut linear_samples = [0.0_f32; ANALYSIS];
        let mut nonlinear_samples = [0.0_f32; ANALYSIS];
        for index in 0..ANALYSIS {
            let input =
                0.8 * (core::f32::consts::TAU * (index % PERIOD) as f32 / PERIOD as f32).sin();
            linear_samples[index] = linear.sample(input, cutoff);
            nonlinear_samples[index] = nonlinear.sample(input, cutoff);
        }
        let linear_third = harmonic_magnitude(&linear_samples, 3, PERIOD);
        let nonlinear_fundamental = harmonic_magnitude(&nonlinear_samples, 1, PERIOD);
        let nonlinear_third = harmonic_magnitude(&nonlinear_samples, 3, PERIOD);
        assert!(
            nonlinear_third > nonlinear_fundamental * 1.0e-4,
            "fundamental={nonlinear_fundamental}, third={nonlinear_third}"
        );
        assert!(
            nonlinear_third > linear_third * 100.0,
            "linear_third={linear_third}, nonlinear_third={nonlinear_third}"
        );
    }

    #[test]
    fn declared_cutoff_resonance_and_drive_range_remains_stable() {
        for mode in [LadderMode::Linear, LadderMode::Nonlinear] {
            for resonance in [0.0, 0.5, 1.0] {
                for drive in [1.0, 2.5, 4.0] {
                    let mut ladder = ModelDLadder::new(
                        SAMPLE_RATE,
                        LadderConfig {
                            resonance,
                            drive,
                            mode,
                        },
                    )
                    .unwrap();
                    for cutoff in [0.0, 0.125, 0.25, 0.5, 0.75, 1.0] {
                        for index in 0..2_048 {
                            let phase = core::f32::consts::TAU * index as f32 / 113.0;
                            let input = 0.45 * phase.sin() + 0.15 * (3.0 * phase).sin();
                            let output = ladder.sample(input, cutoff);
                            assert!(
                                output.is_finite() && output.abs() < 16.0,
                                "mode={mode:?}, resonance={resonance}, drive={drive}, cutoff={cutoff}, output={output}"
                            );
                        }
                    }
                }
            }
        }
    }
}
