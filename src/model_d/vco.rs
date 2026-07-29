#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::{ModelDVco, ModelDWaveform, VcoConfig};

    const SAMPLE_RATES: [f32; 3] = [44_100.0, 48_000.0, 96_000.0];
    const NOTES: [u8; 3] = [36, 60, 84];

    fn config(waveform: ModelDWaveform) -> VcoConfig {
        VcoConfig {
            semitone_offset: 0,
            cents_offset: 0.0,
            drift_cents: 0.0,
            drift_hz: 0.25,
            asymmetry: 0.0,
            level_offset: 0.0,
            reset_phase: 0.0,
            waveform,
        }
    }

    fn midi_hz(note: u8) -> f32 {
        440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
    }

    fn cents_ratio(cents: f32) -> f32 {
        2.0_f32.powf(cents / 1_200.0)
    }

    fn estimate_triangle_hz(vco: &mut ModelDVco, sample_rate: f32) -> f32 {
        let mut previous = vco.sample();
        let mut crossings = [0.0; 8];
        let mut count = 0;
        for index in 1..(sample_rate as usize * 2) {
            let current = vco.sample();
            if previous <= 0.0 && current > 0.0 {
                let fraction = (-previous / (current - previous)).clamp(0.0, 1.0);
                crossings[count] = index as f32 - 1.0 + fraction;
                count += 1;
                if count == crossings.len() {
                    break;
                }
            }
            previous = current;
        }
        assert!(count >= 3, "insufficient crossings: {count}");
        let periods = count - 1;
        sample_rate * periods as f32 / (crossings[periods] - crossings[0])
    }

    #[test]
    fn rejects_invalid_sample_rates_and_configuration() {
        assert!(ModelDVco::new(0.0, config(ModelDWaveform::Saw)).is_err());
        assert!(ModelDVco::new(f32::NAN, config(ModelDWaveform::Saw)).is_err());
        assert!(ModelDVco::new(f32::from_bits(1), config(ModelDWaveform::Saw)).is_err());

        let mut invalid = config(ModelDWaveform::Saw);
        invalid.cents_offset = 2.51;
        assert!(ModelDVco::new(48_000.0, invalid).is_err());

        let mut invalid = config(ModelDWaveform::Saw);
        invalid.drift_cents = 1.51;
        assert!(ModelDVco::new(48_000.0, invalid).is_err());

        let mut invalid = config(ModelDWaveform::Saw);
        invalid.reset_phase = f32::NAN;
        assert!(ModelDVco::new(48_000.0, invalid).is_err());
    }

    #[test]
    fn reset_restores_the_same_deterministic_waveform() {
        let mut first = ModelDVco::new(48_000.0, config(ModelDWaveform::Saw)).unwrap();
        let mut second = ModelDVco::new(48_000.0, config(ModelDWaveform::Saw)).unwrap();
        first.set_note(60);
        second.set_note(60);
        let expected: Vec<_> = (0..2_048).map(|_| first.sample()).collect();
        let actual: Vec<_> = (0..2_048).map(|_| second.sample()).collect();
        assert_eq!(actual, expected);
        first.reset();
        assert_eq!(
            (0..512).map(|_| first.sample()).collect::<Vec<_>>(),
            expected[..512]
        );
    }

    #[test]
    fn static_offset_and_drift_are_limited_at_preparation() {
        let mut vco = ModelDVco::new(
            48_000.0,
            VcoConfig {
                cents_offset: -2.5,
                drift_cents: 1.5,
                ..config(ModelDWaveform::Triangle)
            },
        )
        .unwrap();
        vco.set_note(60);
        assert!(vco.static_cents.abs() <= 2.5);
        assert!(vco.drift_cents.abs() <= 1.5);
        assert!((vco.static_ratio - cents_ratio(-2.5)).abs() < 1.0e-6);
    }

    #[test]
    fn drift_excursion_stays_within_its_declared_bound_over_long_low_rate_runs() {
        for (sample_rate, drift_hz) in [(44_100.0, 0.01), (48_000.0, 0.25)] {
            let mut vco = ModelDVco::new(
                sample_rate,
                VcoConfig {
                    drift_cents: 1.5,
                    drift_hz,
                    reset_phase: 0.25,
                    ..config(ModelDWaveform::Triangle)
                },
            )
            .unwrap();
            vco.set_note(60);
            let nominal_increment = vco.base_increment * vco.static_ratio;
            let mut minimum_cents = f32::INFINITY;
            let mut maximum_cents = f32::NEG_INFINITY;
            for _ in 0..(sample_rate as usize * 60 * 10) {
                let observed_cents = 1_200.0 * (vco.phase_increment() / nominal_increment).log2();
                minimum_cents = minimum_cents.min(observed_cents);
                maximum_cents = maximum_cents.max(observed_cents);
                vco.sample();
            }
            assert!(
                minimum_cents >= -1.5 && maximum_cents <= 1.5,
                "sample_rate={sample_rate}, drift_hz={drift_hz}, minimum_cents={minimum_cents}, maximum_cents={maximum_cents}"
            );
        }
    }

    #[test]
    fn negative_maximum_drift_never_exceeds_its_declared_positive_bound() {
        let sample_rate = 48_000.0;
        let mut vco = ModelDVco::new(
            sample_rate,
            VcoConfig {
                drift_cents: -1.5,
                drift_hz: 0.25,
                reset_phase: 0.75,
                ..config(ModelDWaveform::Triangle)
            },
        )
        .unwrap();
        vco.set_note(60);
        let nominal_increment = vco.base_increment * vco.static_ratio;
        for _ in 0..(sample_rate as usize * 60 * 10) {
            let observed_cents = 1_200.0 * (vco.phase_increment() / nominal_increment).log2();
            assert!(
                observed_cents.abs() <= 1.5,
                "observed_cents={observed_cents}"
            );
            vco.sample();
        }
    }

    #[test]
    fn zero_drift_preserves_the_exact_prepared_pitch() {
        let mut vco = ModelDVco::new(
            48_000.0,
            VcoConfig {
                drift_cents: 0.0,
                drift_hz: 0.25,
                reset_phase: 0.75,
                ..config(ModelDWaveform::Triangle)
            },
        )
        .unwrap();
        vco.set_note(60);
        let nominal_increment = vco.base_increment * vco.static_ratio;
        for _ in 0..4_096 {
            assert_eq!(vco.phase_increment(), nominal_increment);
            vco.sample();
        }
    }

    #[test]
    fn triangle_mean_pitch_tracks_midi_notes_across_supported_sample_rates() {
        for sample_rate in SAMPLE_RATES {
            for note in NOTES {
                let mut vco =
                    ModelDVco::new(sample_rate, config(ModelDWaveform::Triangle)).unwrap();
                vco.set_note(note);
                let measured_hz = estimate_triangle_hz(&mut vco, sample_rate);
                let error_cents = 1_200.0 * (measured_hz / midi_hz(note)).log2();
                assert!(
                    error_cents.abs() <= 5.0,
                    "sample_rate={sample_rate}, note={note}, measured_hz={measured_hz}, error_cents={error_cents}"
                );
            }
        }
    }

    #[test]
    fn pulse_width_is_clamped_to_the_declared_range() {
        for waveform in [
            ModelDWaveform::Rectangle,
            ModelDWaveform::WidePulse,
            ModelDWaveform::NarrowPulse,
        ] {
            for asymmetry in [-1.0, 0.0, 1.0] {
                let vco = ModelDVco::new(
                    48_000.0,
                    VcoConfig {
                        asymmetry,
                        ..config(waveform)
                    },
                )
                .unwrap();
                assert!(
                    (0.10..=0.90).contains(&vco.pulse_width),
                    "waveform={waveform:?}, asymmetry={asymmetry}, pulse_width={}",
                    vco.pulse_width
                );
            }
        }
    }

    #[test]
    fn sampling_is_finite_bounded_and_allocation_free() {
        for waveform in [
            ModelDWaveform::Triangle,
            ModelDWaveform::Saw,
            ModelDWaveform::Rectangle,
            ModelDWaveform::WidePulse,
            ModelDWaveform::NarrowPulse,
        ] {
            let mut vco = ModelDVco::new(
                48_000.0,
                VcoConfig {
                    cents_offset: 2.5,
                    drift_cents: 1.5,
                    drift_hz: 0.25,
                    asymmetry: 1.0,
                    level_offset: 0.25,
                    reset_phase: 0.37,
                    waveform,
                    ..config(waveform)
                },
            )
            .unwrap();
            vco.set_note(84);
            assert_no_alloc(|| {
                for _ in 0..4_096 {
                    let sample = vco.sample();
                    assert!(sample.is_finite() && sample.abs() <= 1.5, "{sample}");
                }
            });
        }
    }
}
use std::f32::consts::TAU;

use super::ModelDError;

const MAX_STATIC_CENTS: f32 = 2.5;
const MAX_DRIFT_CENTS: f32 = 1.5;
const MAX_DRIFT_HZ: f32 = 2.0;
const MAX_LEVEL_OFFSET: f32 = 0.5;
const MIN_PULSE_WIDTH: f32 = 0.10;
const MAX_PULSE_WIDTH: f32 = 0.90;
const MAX_PHASE_INCREMENT: f32 = 0.49;
const DRIFT_NORMALIZE_PERIOD: u16 = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDWaveform {
    Triangle,
    Saw,
    Rectangle,
    WidePulse,
    NarrowPulse,
}

#[derive(Clone, Copy, Debug)]
pub struct VcoConfig {
    pub semitone_offset: i8,
    pub cents_offset: f32,
    pub drift_cents: f32,
    pub drift_hz: f32,
    pub asymmetry: f32,
    pub level_offset: f32,
    pub reset_phase: f32,
    pub waveform: ModelDWaveform,
}

/// A prepared, independently phased oscillator for the isolated Model D study.
///
/// The configuration and note boundary prepare every exponential or trigonometric
/// value. `sample` only advances scalar recurrence state and evaluates the selected
/// waveform.
#[derive(Clone, Copy, Debug)]
pub struct ModelDVco {
    waveform: ModelDWaveform,
    sample_rate: f32,
    semitone_offset: i8,
    phase: f32,
    reset_phase: f32,
    base_increment: f32,
    static_ratio: f32,
    #[cfg(test)]
    static_cents: f32,
    #[cfg(test)]
    drift_cents: f32,
    drift_sine: f32,
    drift_cosine: f32,
    reset_drift_sine: f32,
    reset_drift_cosine: f32,
    drift_rotation_sine: f32,
    drift_rotation_cosine: f32,
    drift_samples_since_normalize: u16,
    drift_polarity: f32,
    drift_up_scale: f32,
    drift_down_scale: f32,
    triangle_peak: f32,
    pulse_width: f32,
    level: f32,
}

impl ModelDVco {
    pub fn new(sample_rate: f32, config: VcoConfig) -> Result<Self, ModelDError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(ModelDError::InvalidSampleRate);
        }
        if !valid_config(config) {
            return Err(ModelDError::InvalidConfig);
        }

        let reset_phase = normalize_reset_phase(config.reset_phase);
        let drift_angle = TAU * reset_phase;
        let (reset_drift_sine, reset_drift_cosine) = drift_angle.sin_cos();
        let rotation_angle = TAU * config.drift_hz / sample_rate;
        if !rotation_angle.is_finite() {
            return Err(ModelDError::InvalidSampleRate);
        }
        let (drift_rotation_sine, drift_rotation_cosine) = rotation_angle.sin_cos();
        let static_ratio = cents_ratio(config.cents_offset);
        let drift_magnitude = config.drift_cents.abs();
        let (drift_up_scale, drift_down_scale) = if drift_magnitude == 0.0 {
            (0.0, 0.0)
        } else {
            (
                previous_positive_f32(cents_ratio(drift_magnitude)) - 1.0,
                next_positive_f32(cents_ratio(-drift_magnitude)) - 1.0,
            )
        };
        let drift_polarity = if config.drift_cents < 0.0 { -1.0 } else { 1.0 };
        let asymmetry = config.asymmetry.clamp(-1.0, 1.0);
        let base_pulse_width = match config.waveform {
            ModelDWaveform::Rectangle => 0.50,
            ModelDWaveform::WidePulse => 0.65,
            ModelDWaveform::NarrowPulse => 0.35,
            ModelDWaveform::Triangle | ModelDWaveform::Saw => 0.50,
        };

        Ok(Self {
            waveform: config.waveform,
            sample_rate,
            semitone_offset: config.semitone_offset,
            phase: reset_phase,
            reset_phase,
            base_increment: 0.0,
            static_ratio,
            #[cfg(test)]
            static_cents: config.cents_offset,
            #[cfg(test)]
            drift_cents: config.drift_cents,
            drift_sine: reset_drift_sine,
            drift_cosine: reset_drift_cosine,
            reset_drift_sine,
            reset_drift_cosine,
            drift_rotation_sine,
            drift_rotation_cosine,
            drift_samples_since_normalize: 0,
            drift_polarity,
            drift_up_scale,
            drift_down_scale,
            triangle_peak: (0.50 + 0.20 * asymmetry).clamp(0.20, 0.80),
            pulse_width: (base_pulse_width + 0.25 * asymmetry)
                .clamp(MIN_PULSE_WIDTH, MAX_PULSE_WIDTH),
            level: 1.0 + config.level_offset,
        })
    }

    pub fn set_note(&mut self, note: u8) {
        let semitones_from_a4 = note as f32 + self.semitone_offset() as f32 - 69.0;
        let frequency_hz = 440.0 * 2.0_f32.powf(semitones_from_a4 / 12.0);
        let maximum_drift_ratio = (1.0 + self.drift_up_scale).max(1.0 + self.drift_down_scale);
        let maximum_base_increment =
            MAX_PHASE_INCREMENT / (self.static_ratio * maximum_drift_ratio);
        self.base_increment = (frequency_hz / self.sample_rate).clamp(0.0, maximum_base_increment);
        self.reset();
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        if self.base_increment <= 0.0 {
            return 0.0;
        }

        let waveform = match self.waveform {
            ModelDWaveform::Triangle => triangle(self.phase, self.triangle_peak),
            ModelDWaveform::Saw => {
                let phase = skew_phase(self.phase, self.triangle_peak);
                2.0 * phase - 1.0 - poly_blep(self.phase, self.phase_increment())
            }
            ModelDWaveform::Rectangle | ModelDWaveform::WidePulse | ModelDWaveform::NarrowPulse => {
                let phase_increment = self.phase_increment();
                let falling_phase = wrap_phase_once(self.phase - self.pulse_width);
                let raw = if self.phase < self.pulse_width {
                    1.0
                } else {
                    -1.0
                };
                raw + poly_blep(self.phase, phase_increment)
                    - poly_blep(falling_phase, phase_increment)
            }
        };
        self.advance_phase();
        waveform * self.level
    }

    pub fn reset(&mut self) {
        self.phase = self.reset_phase;
        self.drift_sine = self.reset_drift_sine;
        self.drift_cosine = self.reset_drift_cosine;
        self.drift_samples_since_normalize = 0;
    }

    #[inline]
    fn semitone_offset(&self) -> i8 {
        self.semitone_offset
    }

    #[inline]
    fn phase_increment(&self) -> f32 {
        let bounded_drift_signal = self.drift_sine.clamp(-1.0, 1.0) * self.drift_polarity;
        let drift_ratio = if bounded_drift_signal >= 0.0 {
            1.0 + bounded_drift_signal * self.drift_up_scale
        } else {
            1.0 - bounded_drift_signal * self.drift_down_scale
        };
        self.base_increment * self.static_ratio * drift_ratio
    }

    #[inline]
    fn advance_phase(&mut self) {
        self.phase += self.phase_increment();
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let drift_sine = self.drift_sine * self.drift_rotation_cosine
            + self.drift_cosine * self.drift_rotation_sine;
        let drift_cosine = self.drift_cosine * self.drift_rotation_cosine
            - self.drift_sine * self.drift_rotation_sine;
        self.drift_sine = drift_sine;
        self.drift_cosine = drift_cosine;
        self.drift_samples_since_normalize += 1;
        if self.drift_samples_since_normalize == DRIFT_NORMALIZE_PERIOD {
            let scale = (self.drift_sine * self.drift_sine + self.drift_cosine * self.drift_cosine)
                .sqrt()
                .recip();
            self.drift_sine *= scale;
            self.drift_cosine *= scale;
            self.drift_samples_since_normalize = 0;
        }
    }
}

fn valid_config(config: VcoConfig) -> bool {
    config.cents_offset.is_finite()
        && config.cents_offset.abs() <= MAX_STATIC_CENTS
        && config.drift_cents.is_finite()
        && config.drift_cents.abs() <= MAX_DRIFT_CENTS
        && config.drift_hz.is_finite()
        && (0.0..=MAX_DRIFT_HZ).contains(&config.drift_hz)
        && config.asymmetry.is_finite()
        && (-1.0..=1.0).contains(&config.asymmetry)
        && config.level_offset.is_finite()
        && config.level_offset.abs() <= MAX_LEVEL_OFFSET
        && config.reset_phase.is_finite()
}

#[inline]
fn cents_ratio(cents: f32) -> f32 {
    2.0_f32.powf(cents / 1_200.0)
}

#[inline]
fn next_positive_f32(value: f32) -> f32 {
    debug_assert!(value.is_finite() && value > 0.0);
    f32::from_bits(value.to_bits() + 1)
}

#[inline]
fn previous_positive_f32(value: f32) -> f32 {
    debug_assert!(value.is_finite() && value > 0.0);
    f32::from_bits(value.to_bits() - 1)
}

#[inline]
fn normalize_reset_phase(phase: f32) -> f32 {
    phase - phase.floor()
}

#[inline]
fn wrap_phase_once(mut phase: f32) -> f32 {
    if phase < 0.0 {
        phase += 1.0;
    } else if phase >= 1.0 {
        phase -= 1.0;
    }
    phase
}

#[inline]
fn skew_phase(phase: f32, peak: f32) -> f32 {
    if phase < peak {
        0.5 * phase / peak
    } else {
        0.5 + 0.5 * (phase - peak) / (1.0 - peak)
    }
}

#[inline]
fn triangle(phase: f32, peak: f32) -> f32 {
    if phase < peak {
        2.0 * phase / peak - 1.0
    } else {
        1.0 - 2.0 * (phase - peak) / (1.0 - peak)
    }
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
