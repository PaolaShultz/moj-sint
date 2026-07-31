use std::{ops::Deref, sync::Arc};

use thiserror::Error;

use super::envelope::{Envelope, EnvelopeError, EnvelopeSpec};

pub const SINE_TABLE_SIZE: usize = 4_096;
pub type SharedSineTable = Arc<[f32; SINE_TABLE_SIZE]>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrequencyMode {
    Ratio(f32),
    Fixed(f32),
}

impl FrequencyMode {
    pub fn ratio(ratio: f32) -> Result<Self, OperatorError> {
        let mode = Self::Ratio(ratio);
        mode.validate()?;
        Ok(mode)
    }

    pub fn fixed(frequency_hz: f32) -> Result<Self, OperatorError> {
        let mode = Self::Fixed(frequency_hz);
        mode.validate()?;
        Ok(mode)
    }

    fn validate(self) -> Result<(), OperatorError> {
        let value = match self {
            Self::Ratio(value) | Self::Fixed(value) => value,
        };
        if !value.is_finite() || value <= 0.0 || value > 20_000.0 {
            return Err(OperatorError::InvalidFrequency);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyboardScaling {
    pub break_note: u8,
    pub left_db_per_octave: f32,
    pub right_db_per_octave: f32,
}

impl KeyboardScaling {
    pub fn new(
        break_note: u8,
        left_db_per_octave: f32,
        right_db_per_octave: f32,
    ) -> Result<Self, OperatorError> {
        let scaling = Self {
            break_note,
            left_db_per_octave,
            right_db_per_octave,
        };
        scaling.validate()?;
        Ok(scaling)
    }

    fn validate(self) -> Result<(), OperatorError> {
        if self.break_note > 127 {
            return Err(OperatorError::InvalidBreakNote);
        }
        if !valid_slope(self.left_db_per_octave) || !valid_slope(self.right_db_per_octave) {
            return Err(OperatorError::InvalidScaling);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperatorSpec {
    pub frequency: FrequencyMode,
    pub output_level: f32,
    pub envelope: EnvelopeSpec,
    pub velocity_sensitivity: f32,
    pub scaling: KeyboardScaling,
    pub detune_cents: f32,
}

impl OperatorSpec {
    pub fn new(
        frequency: FrequencyMode,
        output_level: f32,
        envelope: EnvelopeSpec,
        velocity_sensitivity: f32,
        scaling: KeyboardScaling,
        detune_cents: f32,
    ) -> Result<Self, OperatorError> {
        let spec = Self {
            frequency,
            output_level,
            envelope,
            velocity_sensitivity,
            scaling,
            detune_cents,
        };
        spec.validate()?;
        Ok(spec)
    }

    fn validate(self) -> Result<(), OperatorError> {
        self.frequency.validate()?;
        if !valid_unit(self.output_level) {
            return Err(OperatorError::InvalidOutputLevel);
        }
        self.envelope.validate()?;
        if !valid_unit(self.velocity_sensitivity) {
            return Err(OperatorError::InvalidVelocitySensitivity);
        }
        self.scaling.validate()?;
        if !self.detune_cents.is_finite() || !(-50.0..=50.0).contains(&self.detune_cents) {
            return Err(OperatorError::InvalidDetune);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum OperatorError {
    #[error("invalid ratio or fixed frequency")]
    InvalidFrequency,
    #[error("invalid output level")]
    InvalidOutputLevel,
    #[error("invalid velocity sensitivity")]
    InvalidVelocitySensitivity,
    #[error("invalid keyboard scaling slope")]
    InvalidScaling,
    #[error("invalid keyboard scaling break note")]
    InvalidBreakNote,
    #[error("invalid detune")]
    InvalidDetune,
    #[error("invalid sample rate")]
    InvalidSampleRate,
    #[error("invalid MIDI note")]
    InvalidNote,
    #[error("invalid velocity")]
    InvalidVelocity,
    #[error("prepared frequency is not finite")]
    InvalidPreparedFrequency,
    #[error("invalid sine table")]
    InvalidSineTable,
    #[error(transparent)]
    Envelope(#[from] EnvelopeError),
}

#[derive(Clone, Debug)]
pub struct SineTable(SharedSineTable);

impl SineTable {
    pub fn new() -> Self {
        Self(Arc::new(std::array::from_fn(|index| {
            (std::f32::consts::TAU * index as f32 / SINE_TABLE_SIZE as f32).sin()
        })))
    }

    #[inline]
    pub fn lookup(table: &[f32; SINE_TABLE_SIZE], cycles: f32) -> f32 {
        if !cycles.is_finite() {
            return 0.0;
        }
        let position = cycles.rem_euclid(1.0) * SINE_TABLE_SIZE as f32;
        let index = position as usize;
        let fraction = position - index as f32;
        let next = (index + 1) & (SINE_TABLE_SIZE - 1);
        table[index] + (table[next] - table[index]) * fraction
    }
}

impl Default for SineTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for SineTable {
    type Target = [f32; SINE_TABLE_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SineTable> for SharedSineTable {
    fn from(table: SineTable) -> Self {
        table.0
    }
}

#[derive(Clone, Debug)]
pub struct PreparedOperator {
    phase: f32,
    initial_phase: f32,
    increment: f32,
    frequency_hz: f32,
    prepared_level: f32,
    envelope: Envelope,
    sine_table: SharedSineTable,
}

impl PreparedOperator {
    pub fn new<T>(
        spec: OperatorSpec,
        sample_rate: f32,
        note: u8,
        velocity: f32,
        sine_table: T,
    ) -> Result<Self, OperatorError>
    where
        T: Into<SharedSineTable>,
    {
        spec.validate()?;
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(OperatorError::InvalidSampleRate);
        }
        if note > 127 {
            return Err(OperatorError::InvalidNote);
        }
        if !valid_unit(velocity) {
            return Err(OperatorError::InvalidVelocity);
        }
        let sine_table = sine_table.into();
        if sine_table
            .iter()
            .any(|sample| !sample.is_finite() || sample.abs() > 1.0)
        {
            return Err(OperatorError::InvalidSineTable);
        }

        let base_frequency = match spec.frequency {
            FrequencyMode::Ratio(ratio) => midi_note_hz(note) * ratio,
            FrequencyMode::Fixed(frequency_hz) => frequency_hz,
        };
        let frequency_hz = base_frequency * 2.0_f32.powf(spec.detune_cents / 1_200.0);
        let increment = frequency_hz / sample_rate;
        if !frequency_hz.is_finite() || !increment.is_finite() {
            return Err(OperatorError::InvalidPreparedFrequency);
        }

        let key_db = key_scaling_db(note, spec.scaling);
        let key_gain = 10.0_f32.powf(key_db / 20.0);
        let velocity_gain = 1.0 - spec.velocity_sensitivity * (1.0 - velocity);
        let prepared_level = (spec.output_level * key_gain * velocity_gain).clamp(0.0, 1.0);
        if !prepared_level.is_finite() {
            return Err(OperatorError::InvalidOutputLevel);
        }

        Ok(Self {
            phase: 0.0,
            initial_phase: 0.0,
            increment,
            frequency_hz,
            prepared_level,
            envelope: Envelope::new(spec.envelope, sample_rate)?,
            sine_table,
        })
    }

    #[inline]
    pub fn sample(&mut self, phase_modulation_cycles: f32, pitch_multiplier: f32) -> f32 {
        let modulation = if phase_modulation_cycles.is_finite() {
            phase_modulation_cycles
        } else {
            0.0
        };
        let pitch_multiplier = if pitch_multiplier.is_finite() {
            pitch_multiplier.clamp(0.0, 16.0)
        } else {
            0.0
        };

        let sine = SineTable::lookup(&self.sine_table, self.phase + modulation);
        self.phase = (self.phase + self.increment * pitch_multiplier).rem_euclid(1.0);
        let output = sine * self.envelope.advance() * self.prepared_level;
        if output.is_finite() { output } else { 0.0 }
    }

    pub fn note_on(&mut self) {
        self.envelope.note_on();
    }

    pub fn note_off(&mut self) {
        self.envelope.note_off();
    }

    pub fn reset(&mut self) {
        self.phase = self.initial_phase;
        self.envelope.reset();
    }

    pub const fn frequency_hz(&self) -> f32 {
        self.frequency_hz
    }

    pub const fn prepared_level(&self) -> f32 {
        self.prepared_level
    }

    pub const fn phase(&self) -> f32 {
        self.phase
    }

    pub const fn envelope_level(&self) -> f32 {
        self.envelope.level()
    }

    pub const fn is_idle(&self) -> bool {
        self.envelope.is_idle()
    }
}

fn midi_note_hz(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

fn key_scaling_db(note: u8, scaling: KeyboardScaling) -> f32 {
    if note < scaling.break_note {
        scaling.left_db_per_octave * f32::from(scaling.break_note - note) / 12.0
    } else {
        scaling.right_db_per_octave * f32::from(note - scaling.break_note) / 12.0
    }
}

fn valid_unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_slope(value: f32) -> bool {
    value.is_finite() && (-24.0..=24.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    fn assert_close(actual: f32, expected: f32, tolerance: f32) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} != {expected} within {tolerance}"
        );
    }

    fn envelope() -> EnvelopeSpec {
        EnvelopeSpec::new([0.001; 4], [1.0, 1.0, 1.0, 0.0]).unwrap()
    }

    fn spec(frequency: FrequencyMode) -> OperatorSpec {
        OperatorSpec::new(
            frequency,
            0.8,
            envelope(),
            0.5,
            KeyboardScaling::new(69, -6.0, -6.0).unwrap(),
            0.0,
        )
        .unwrap()
    }

    #[test]
    fn ratio_tracks_note_and_fixed_frequency_ignores_it() {
        let table = SineTable::new();
        let ratio = PreparedOperator::new(
            spec(FrequencyMode::ratio(2.0).unwrap()),
            48_000.0,
            69,
            1.0,
            table.clone(),
        )
        .unwrap();
        assert_close(ratio.frequency_hz(), 880.0, 1.0e-3);

        let fixed_spec = spec(FrequencyMode::fixed(777.0).unwrap());
        let low = PreparedOperator::new(fixed_spec, 48_000.0, 24, 1.0, table.clone()).unwrap();
        let high = PreparedOperator::new(fixed_spec, 48_000.0, 108, 1.0, table).unwrap();
        assert_close(low.frequency_hz(), 777.0, 1.0e-3);
        assert_close(high.frequency_hz(), 777.0, 1.0e-3);
    }

    #[test]
    fn key_scaling_and_velocity_only_change_prepared_level() {
        let table = SineTable::new();
        let base_spec = OperatorSpec::new(
            FrequencyMode::ratio(1.0).unwrap(),
            0.8,
            envelope(),
            1.0,
            KeyboardScaling::new(69, -6.0, -6.0).unwrap(),
            0.0,
        )
        .unwrap();

        let center = PreparedOperator::new(base_spec, 48_000.0, 69, 1.0, table.clone()).unwrap();
        let left = PreparedOperator::new(base_spec, 48_000.0, 57, 1.0, table.clone()).unwrap();
        let right = PreparedOperator::new(base_spec, 48_000.0, 81, 1.0, table.clone()).unwrap();
        let quiet = PreparedOperator::new(base_spec, 48_000.0, 69, 0.25, table.clone()).unwrap();
        let unscaled_spec = OperatorSpec::new(
            FrequencyMode::ratio(1.0).unwrap(),
            0.8,
            envelope(),
            1.0,
            KeyboardScaling::new(69, 0.0, 0.0).unwrap(),
            0.0,
        )
        .unwrap();
        let unscaled = PreparedOperator::new(unscaled_spec, 48_000.0, 81, 1.0, table).unwrap();

        assert!(left.prepared_level() < center.prepared_level());
        assert!(right.prepared_level() < center.prepared_level());
        assert!(right.prepared_level() < unscaled.prepared_level());
        assert!(quiet.prepared_level() < center.prepared_level());
        assert_eq!(quiet.frequency_hz(), center.frequency_hz());
        assert_eq!(right.frequency_hz(), unscaled.frequency_hz());
        assert_close(left.frequency_hz(), 220.0, 1.0e-3);
        assert_close(center.frequency_hz(), 440.0, 1.0e-3);
        assert_close(right.frequency_hz(), 880.0, 1.0e-3);
        assert!(
            [left, center, right, quiet, unscaled]
                .iter()
                .all(|operator| (0.0..=1.0).contains(&operator.prepared_level()))
        );
    }

    #[test]
    fn detune_is_applied_in_cents() {
        let table = SineTable::new();
        for cents in [-50.0, 50.0] {
            let detuned_spec = OperatorSpec::new(
                FrequencyMode::fixed(440.0).unwrap(),
                1.0,
                envelope(),
                0.0,
                KeyboardScaling::new(69, 0.0, 0.0).unwrap(),
                cents,
            )
            .unwrap();
            let operator =
                PreparedOperator::new(detuned_spec, 48_000.0, 69, 1.0, table.clone()).unwrap();
            assert_close(
                operator.frequency_hz(),
                440.0 * 2.0_f32.powf(cents / 1_200.0),
                1.0e-3,
            );
        }
    }

    #[test]
    fn every_invalid_or_non_finite_field_is_rejected() {
        for value in [0.0, -1.0, 20_001.0, f32::NAN, f32::INFINITY] {
            assert!(FrequencyMode::ratio(value).is_err());
            assert!(FrequencyMode::fixed(value).is_err());
        }
        for slopes in [
            (-24.01, 0.0),
            (24.01, 0.0),
            (f32::NAN, 0.0),
            (0.0, f32::INFINITY),
        ] {
            assert!(KeyboardScaling::new(69, slopes.0, slopes.1).is_err());
        }
        assert!(KeyboardScaling::new(128, 0.0, 0.0).is_err());

        let scaling = KeyboardScaling::new(69, 0.0, 0.0).unwrap();
        for output_level in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            assert!(
                OperatorSpec::new(
                    FrequencyMode::Ratio(1.0),
                    output_level,
                    envelope(),
                    0.0,
                    scaling,
                    0.0,
                )
                .is_err()
            );
        }
        for sensitivity in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            assert!(
                OperatorSpec::new(
                    FrequencyMode::Ratio(1.0),
                    1.0,
                    envelope(),
                    sensitivity,
                    scaling,
                    0.0,
                )
                .is_err()
            );
        }
        for detune in [-50.01, 50.01, f32::NAN, f32::INFINITY] {
            assert!(
                OperatorSpec::new(
                    FrequencyMode::Ratio(1.0),
                    1.0,
                    envelope(),
                    0.0,
                    scaling,
                    detune,
                )
                .is_err()
            );
        }

        let table = SineTable::new();
        let valid_spec = spec(FrequencyMode::Ratio(1.0));
        for sample_rate in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(
                PreparedOperator::new(valid_spec, sample_rate, 69, 1.0, table.clone()).is_err()
            );
        }
        assert!(PreparedOperator::new(valid_spec, 48_000.0, 128, 1.0, table.clone()).is_err());
        for velocity in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            assert!(
                PreparedOperator::new(valid_spec, 48_000.0, 69, velocity, table.clone()).is_err()
            );
        }

        let bypassed_frequency = OperatorSpec {
            frequency: FrequencyMode::Fixed(f32::NAN),
            ..valid_spec
        };
        assert!(
            PreparedOperator::new(bypassed_frequency, 48_000.0, 69, 1.0, table.clone()).is_err()
        );
        let bypassed_envelope = OperatorSpec {
            envelope: EnvelopeSpec {
                seconds: [0.0; 4],
                levels: [2.0; 4],
            },
            ..valid_spec
        };
        assert!(PreparedOperator::new(bypassed_envelope, 48_000.0, 69, 1.0, table).is_err());

        let invalid_table = Arc::new([f32::NAN; SINE_TABLE_SIZE]);
        assert!(PreparedOperator::new(valid_spec, 48_000.0, 69, 1.0, invalid_table).is_err());
    }

    #[test]
    fn interpolated_sine_is_periodic_finite_and_close_at_boundaries() {
        let table = SineTable::new();
        for cycles in [-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0] {
            let actual = SineTable::lookup(&table, cycles);
            assert!(actual.is_finite());
            assert_close(actual, (cycles * std::f32::consts::TAU).sin(), 2.0e-6);
            assert_eq!(actual, SineTable::lookup(&table, cycles + 2.0));
        }
        for cycles in [-1.0e-6, 1.0e-6, 0.249_999, 0.250_001, 0.999_999, 1.000_001] {
            assert_close(
                SineTable::lookup(&table, cycles),
                (cycles * std::f32::consts::TAU).sin(),
                3.0e-6,
            );
        }
        assert_eq!(SineTable::lookup(&table, f32::NAN), 0.0);
    }

    #[test]
    fn sampling_is_bounded_resettable_and_allocation_free() {
        let table = SineTable::new();
        let mut operator =
            PreparedOperator::new(spec(FrequencyMode::Ratio(1.5)), 48_000.0, 69, 0.8, table)
                .unwrap();
        operator.note_on();
        let first = operator.sample(0.125, 1.0);
        assert!(first.is_finite() && first.abs() <= 1.0);
        operator.reset();
        operator.note_on();
        assert_no_alloc(|| {
            for index in 0..48_000 {
                let modulation = (index % 101) as f32 * 0.001 - 0.05;
                let sample = operator.sample(modulation, 1.01);
                assert!(sample.is_finite() && sample.abs() <= 1.0);
            }
        });

        operator.reset();
        operator.note_on();
        assert_eq!(operator.sample(0.125, 1.0), first);
        assert!(operator.sample(f32::NAN, f32::INFINITY).is_finite());
        operator.note_off();
    }
}
