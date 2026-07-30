use super::ModelDError;

const OVERSAMPLE_FACTOR: usize = 4;
const CUTOFF_TABLE_SIZE: usize = 2_049;
const DECIMATOR_TAPS: usize = 63;
const DECIMATOR_CUTOFF: f32 = 0.078_125;
const MIN_CUTOFF_HZ: f32 = 20.0;
const MAX_CUTOFF_HZ: f32 = 8_000.0;
const MAX_CUTOFF_RATIO: f32 = 0.2;
const MIN_DRIVE: f32 = 1.0;
const MAX_DRIVE: f32 = 4.0;
const MAX_RESONANCE_FEEDBACK: f32 = 3.5;
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
    coefficient_table: [CutoffCoefficients; CUTOFF_TABLE_SIZE],
    decimator_coefficients: [f32; DECIMATOR_TAPS],
    decimator_history: [f32; DECIMATOR_TAPS],
    decimator_index: usize,
    states: [f32; 4],
    resonance_tuning: f32,
    resonance_feedback: f32,
    drive: f32,
    character: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct CutoffCoefficients {
    flat: f32,
    resonant: f32,
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

        let decimator_coefficients = design_decimator();
        let cutoff_ratio = maximum_cutoff / MIN_CUTOFF_HZ;
        let mut coefficient_table = [CutoffCoefficients::default(); CUTOFF_TABLE_SIZE];
        for (index, coefficients) in coefficient_table.iter_mut().enumerate() {
            let normalized = index as f32 / (CUTOFF_TABLE_SIZE - 1) as f32;
            let requested_cutoff = MIN_CUTOFF_HZ * cutoff_ratio.powf(normalized);
            let omega = core::f32::consts::TAU * requested_cutoff / internal_sample_rate;
            coefficients.flat =
                calibrated_flat_coefficient(omega, &decimator_coefficients, OVERSAMPLE_FACTOR);
            coefficients.resonant = resonant_coefficient(omega);
        }

        let resonance = config.resonance.clamp(0.0, 1.0);
        let resonance_tuning = if resonance > 0.0 {
            resonance / (0.1 + 0.9 * resonance)
        } else {
            0.0
        };
        Ok(Self {
            coefficient_table,
            decimator_coefficients,
            decimator_history: [0.0; DECIMATOR_TAPS],
            decimator_index: 0,
            states: [0.0; 4],
            resonance_tuning,
            resonance_feedback: resonance * MAX_RESONANCE_FEEDBACK,
            drive: config.drive.clamp(MIN_DRIVE, MAX_DRIVE),
            character: match config.mode {
                LadderMode::Linear => 0.0,
                LadderMode::Nonlinear => 1.0,
            },
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
        let upper = self.coefficient_table[lower_index + 1];
        let flat = lower.flat + fraction * (upper.flat - lower.flat);
        let resonant = lower.resonant + fraction * (upper.resonant - lower.resonant);
        let coefficient = flat + self.resonance_tuning * (resonant - flat);
        let input = input.clamp(-MAX_INPUT, MAX_INPUT);

        for _ in 0..OVERSAMPLE_FACTOR {
            let nonlinear_feedback = bounded_odd(self.states[3]);
            let feedback_output =
                self.states[3] + self.character * (nonlinear_feedback - self.states[3]);
            let differential = input - self.resonance_feedback * feedback_output;
            let nonlinear_input = bounded_odd(self.drive * differential) / self.drive;
            let mut stage_input = differential + self.character * (nonlinear_input - differential);

            for state in &mut self.states {
                *state += coefficient * (stage_input - *state);
                let nonlinear_stage = bounded_odd(*state);
                stage_input = *state + self.character * (nonlinear_stage - *state);
            }
            self.decimator_history[self.decimator_index] = stage_input;
            self.decimator_index += 1;
            if self.decimator_index == DECIMATOR_TAPS {
                self.decimator_index = 0;
            }
        }

        let output = self.decimated_output();
        if output.is_finite() {
            output
        } else {
            self.reset();
            0.0
        }
    }

    pub fn reset(&mut self) {
        self.states = [0.0; 4];
        self.decimator_history = [0.0; DECIMATOR_TAPS];
        self.decimator_index = 0;
    }

    pub fn set_resonance(&mut self, resonance: f32) {
        let resonance = resonance.clamp(0.0, 1.0);
        self.resonance_tuning = if resonance > 0.0 {
            resonance / (0.1 + 0.9 * resonance)
        } else {
            0.0
        };
        self.resonance_feedback = resonance * MAX_RESONANCE_FEEDBACK;
    }

    pub fn set_character(&mut self, amount: f32, drive: f32) {
        self.character = amount.clamp(0.0, 1.0);
        self.drive = drive.clamp(MIN_DRIVE, MAX_DRIVE);
    }

    #[inline]
    fn decimated_output(&self) -> f32 {
        let mut output = 0.0;
        for tap in 0..DECIMATOR_TAPS {
            let history_index = if self.decimator_index > tap {
                self.decimator_index - tap - 1
            } else {
                DECIMATOR_TAPS + self.decimator_index - tap - 1
            };
            output += self.decimator_coefficients[tap] * self.decimator_history[history_index];
        }
        output
    }
}

fn design_decimator() -> [f32; DECIMATOR_TAPS] {
    let mut coefficients = [0.0; DECIMATOR_TAPS];
    let order = (DECIMATOR_TAPS - 1) as f32;
    let center = 0.5 * order;
    let mut sum = 0.0;
    for (index, coefficient) in coefficients.iter_mut().enumerate() {
        let offset = index as f32 - center;
        let sinc = if offset == 0.0 {
            2.0 * DECIMATOR_CUTOFF
        } else {
            (core::f32::consts::TAU * DECIMATOR_CUTOFF * offset).sin()
                / (core::f32::consts::PI * offset)
        };
        let window_phase = core::f32::consts::TAU * index as f32 / order;
        let window = 0.42 - 0.5 * window_phase.cos() + 0.08 * (2.0 * window_phase).cos();
        *coefficient = sinc * window;
        sum += *coefficient;
    }
    for coefficient in &mut coefficients {
        *coefficient /= sum;
    }
    coefficients
}

fn calibrated_flat_coefficient(
    omega: f32,
    decimator: &[f32; DECIMATOR_TAPS],
    hold_samples: usize,
) -> f32 {
    let half_omega = 0.5 * omega;
    let hold_gain =
        (hold_samples as f32 * half_omega).sin() / (hold_samples as f32 * half_omega.sin());
    let decimator_gain = fir_magnitude(decimator, omega);
    let external_gain = (hold_gain * decimator_gain).max(f32::EPSILON);
    let cascade_target = (core::f32::consts::FRAC_1_SQRT_2 / external_gain).min(0.999_999);
    let stage_target = cascade_target.sqrt().sqrt();
    let cosine = omega.cos();
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..32 {
        let coefficient = 0.5 * (lower + upper);
        let memory = 1.0 - coefficient;
        let denominator = (1.0 + memory * memory - 2.0 * memory * cosine).sqrt();
        let magnitude = coefficient / denominator;
        if magnitude < stage_target {
            lower = coefficient;
        } else {
            upper = coefficient;
        }
    }
    0.5 * (lower + upper)
}

fn resonant_coefficient(omega: f32) -> f32 {
    (1.0 - 1.0 / (omega.sin() + omega.cos())).clamp(0.0, 1.0)
}

fn fir_magnitude(coefficients: &[f32; DECIMATOR_TAPS], omega: f32) -> f32 {
    let mut real = 0.0;
    let mut imaginary = 0.0;
    for (index, coefficient) in coefficients.iter().enumerate() {
        let phase = omega * index as f32;
        real += *coefficient * phase.cos();
        imaginary -= *coefficient * phase.sin();
    }
    real.hypot(imaginary)
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

    fn nonlinear_probe_components(sample_rate: f32, frequencies: [f32; 3]) -> [f64; 3] {
        let mut ladder = ModelDLadder::new(
            sample_rate,
            LadderConfig {
                resonance: 0.0,
                drive: 4.0,
                mode: LadderMode::Nonlinear,
            },
        )
        .unwrap();
        let cutoff = normalized_cutoff(8_000.0);
        let tone_hz = 9_000.0;
        let phase_step = core::f32::consts::TAU * tone_hz / sample_rate;
        let warmup_samples = (sample_rate / SAMPLE_RATE * 8_192.0) as usize;
        let analysis_samples = (sample_rate / SAMPLE_RATE * 16_384.0) as usize;
        let mut phase = 0.0_f32;
        for _ in 0..warmup_samples {
            ladder.sample(0.95 * phase.sin(), cutoff);
            phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
        }

        let mut sine = [0.0_f64; 3];
        let mut cosine = [0.0_f64; 3];
        for index in 0..analysis_samples {
            let output = f64::from(ladder.sample(0.95 * phase.sin(), cutoff));
            for probe in 0..frequencies.len() {
                let probe_phase =
                    core::f64::consts::TAU * f64::from(frequencies[probe]) * index as f64
                        / f64::from(sample_rate);
                sine[probe] += output * probe_phase.sin();
                cosine[probe] += output * probe_phase.cos();
            }
            phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
        }
        let scale = 2.0 / analysis_samples as f64;
        [
            scale * sine[0].hypot(cosine[0]),
            scale * sine[1].hypot(cosine[1]),
            scale * sine[2].hypot(cosine[2]),
        ]
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
        let targets = [125.0_f32, 500.0, 2_000.0, 8_000.0];
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
    fn resonance_peak_location_stays_near_the_requested_cutoff() {
        for resonance in [0.45, 0.8] {
            let mut peak_frequency = 0.0;
            let mut peak_gain = 0.0;
            for step in 0..=18 {
                let frequency = 600.0 + step as f32 * 50.0;
                let gain = response(1_000.0, frequency, resonance, 1.0, LadderMode::Linear);
                if gain > peak_gain {
                    peak_gain = gain;
                    peak_frequency = frequency;
                }
            }
            assert!(
                (800.0..=1_250.0).contains(&peak_frequency),
                "resonance={resonance}, peak_frequency={peak_frequency}, peak_gain={peak_gain}"
            );
        }
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
    fn fixed_decimator_suppresses_nonlinear_foldback_against_high_rate_reference() {
        let low_rate = nonlinear_probe_components(SAMPLE_RATE, [9_000.0, 21_000.0, 21_000.0]);
        let high_rate =
            nonlinear_probe_components(4.0 * SAMPLE_RATE, [9_000.0, 21_000.0, 27_000.0]);
        assert!(
            high_rate[2] > high_rate[0] * 1.0e-3,
            "reference excitation produced no measurable third harmonic: {high_rate:?}"
        );
        let alias_excess = (low_rate[1] - high_rate[1]).max(1.0e-12);
        let alias_db = 20.0 * (alias_excess / low_rate[0]).log10();
        assert!(
            alias_db <= -40.0,
            "low_rate={low_rate:?}, high_rate={high_rate:?}, alias_db={alias_db}"
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
