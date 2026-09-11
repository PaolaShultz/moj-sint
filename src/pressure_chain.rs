//! Original pressure-coupled subtractive chain, shared by the live voice and offline lab.
//!
//! The broad source -> resonant filter -> VCA -> colour causality is informed
//! by classic bass-line instruments. The oscillator, filter cells, pressure
//! memory, routings, ranges, and control mappings are independently authored.
//! This is not a TD-3 or TB-303 model. The live adapter owns monophonic MIDI articulation.

use crate::envelope::{Adsr, AdsrConfig};
use crate::research::fitted_residual_db;
use std::f32::consts::TAU;
use thiserror::Error;

const CONTROL_COUNT: usize = 8;
const TABLE_SIZE: usize = 2_049;
const CONTROL_SMOOTH_SECONDS: f32 = 0.010;
const GLIDE_SECONDS: f32 = 0.060;
const PRESSURE_RELAX_SECONDS: f32 = 0.420;
const MIN_CUTOFF_HZ: f32 = 24.0;
const MAX_CUTOFF_HZ: f32 = 17_000.0;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PressureChainControl {
    Source,
    Shape,
    Cutoff,
    Resonance,
    Sweep,
    Decay,
    Pressure,
    Bite,
}

impl PressureChainControl {
    pub const ALL: [Self; CONTROL_COUNT] = [
        Self::Source,
        Self::Shape,
        Self::Cutoff,
        Self::Resonance,
        Self::Sweep,
        Self::Decay,
        Self::Pressure,
        Self::Bite,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "SOURCE",
            Self::Shape => "SHAPE",
            Self::Cutoff => "CUTOFF",
            Self::Resonance => "RESONANCE",
            Self::Sweep => "SWEEP",
            Self::Decay => "DECAY",
            Self::Pressure => "PRESSURE",
            Self::Bite => "BITE",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureChainControls([f32; CONTROL_COUNT]);

impl PressureChainControls {
    pub const START: Self = Self([0.35, 0.62, 0.32, 0.68, 0.72, 0.38, 0.70, 0.42]);

    pub fn new(values: [f32; CONTROL_COUNT]) -> Result<Self, PressureChainError> {
        if values
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        {
            Ok(Self(values))
        } else {
            Err(PressureChainError::InvalidControl)
        }
    }

    pub fn clamped(values: [f32; CONTROL_COUNT]) -> Self {
        Self(values.map(|value| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.5
            }
        }))
    }

    pub const fn get(self, control: PressureChainControl) -> f32 {
        self.0[control.index()]
    }

    pub fn set(&mut self, control: PressureChainControl, value: f32) {
        if value.is_finite() {
            self.0[control.index()] = value.clamp(0.0, 1.0);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureChainTopology {
    DeepCascade,
    BodyTap,
    CrossFeed,
}

impl PressureChainTopology {
    pub const ALL: [Self; 3] = [Self::DeepCascade, Self::BodyTap, Self::CrossFeed];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::DeepCascade => "deep-cascade",
            Self::BodyTap => "body-tap",
            Self::CrossFeed => "cross-feed",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::DeepCascade => "Deep Cascade",
            Self::BodyTap => "Body Tap",
            Self::CrossFeed => "Cross Feed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PressureArticulation {
    /// Refresh both contours and start at the new pitch.
    Trigger,
    /// Preserve contours while gliding to a held note.
    Slide,
    /// Refresh both contours from the current amp level while retaining pitch glide.
    RetriggerSlide,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum PressureChainError {
    #[error("sample rate must be finite and between 8,000 and 384,000 Hz")]
    InvalidSampleRate,
    #[error("control values must be finite and between zero and one")]
    InvalidControl,
    #[error("MIDI note must be between 0 and 127")]
    InvalidNote,
    #[error("velocity must be finite and between zero and one")]
    InvalidVelocity,
    #[error("gate and render durations must be finite, positive, and ordered")]
    InvalidDuration,
    #[error("ADSR configuration is invalid")]
    InvalidEnvelope,
}

#[derive(Clone, Copy, Debug, Default)]
struct FallingRampPulse {
    phase: f32,
    increment: f32,
}

impl FallingRampPulse {
    fn set_frequency(&mut self, frequency_hz: f32, sample_rate: f32) {
        self.increment = (frequency_hz / sample_rate).clamp(0.0, 0.45);
    }

    #[inline]
    fn sample(&mut self, blend: f32, width: f32) -> f32 {
        if self.increment <= 0.0 {
            return 0.0;
        }
        let phase = self.phase;
        let ramp = -(2.0 * phase - 1.0 - poly_blep(phase, self.increment));
        let pulse_phase = wrap_phase(phase - width);
        let pulse = (if phase < width { 1.0 } else { -1.0 }) + poly_blep(phase, self.increment)
            - poly_blep(pulse_phase, self.increment);
        self.phase = wrap_phase(phase + self.increment);
        ramp + blend * (pulse - ramp)
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }
}

/// One complete fixed-state monophonic experiment voice.
#[derive(Debug)]
pub struct PressureChainVoice {
    sample_rate: f32,
    topology: PressureChainTopology,
    controls: PressureChainControls,
    smoothed: [f32; CONTROL_COUNT],
    control_coefficient: f32,
    cutoff_table: [f32; TABLE_SIZE],
    decay_table: [f32; TABLE_SIZE],
    oscillator: FallingRampPulse,
    cascade: [f32; 4],
    body: [f32; 2],
    previous_stage_two: f32,
    cross_delay: f32,
    colour_low: f32,
    dc_input: f32,
    dc_output: f32,
    output_low: f32,
    output_coefficient: f32,
    amp: Adsr,
    velocity: f32,
    filter_envelope: f32,
    pressure: f32,
    pressure_decay: f32,
    current_frequency_hz: f32,
    performance_pitch: f32,
    target_frequency_hz: f32,
    glide_coefficient: f32,
}

impl PressureChainVoice {
    pub fn new(
        sample_rate: f32,
        topology: PressureChainTopology,
        controls: PressureChainControls,
        adsr: AdsrConfig,
    ) -> Result<Self, PressureChainError> {
        validate_sample_rate(sample_rate)?;
        let amp = Adsr::new(sample_rate, adsr).map_err(|_| PressureChainError::InvalidEnvelope)?;
        let maximum_cutoff = MAX_CUTOFF_HZ.min(sample_rate * 0.42);
        let cutoff_ratio = maximum_cutoff / MIN_CUTOFF_HZ;
        let mut cutoff_table = [0.0; TABLE_SIZE];
        for (index, coefficient) in cutoff_table.iter_mut().enumerate() {
            let normalized = index as f32 / (TABLE_SIZE - 1) as f32;
            let cutoff_hz = MIN_CUTOFF_HZ * cutoff_ratio.powf(normalized);
            *coefficient = 1.0 - (-TAU * cutoff_hz / sample_rate).exp();
        }
        let mut decay_table = [0.0; TABLE_SIZE];
        for (index, multiplier) in decay_table.iter_mut().enumerate() {
            let normalized = index as f32 / (TABLE_SIZE - 1) as f32;
            let seconds = 0.025 * (64.0_f32).powf(normalized);
            *multiplier = (-1.0 / (sample_rate * seconds)).exp();
        }
        Ok(Self {
            sample_rate,
            topology,
            controls,
            smoothed: controls.0,
            control_coefficient: 1.0 - (-1.0 / (sample_rate * CONTROL_SMOOTH_SECONDS)).exp(),
            cutoff_table,
            decay_table,
            oscillator: FallingRampPulse::default(),
            cascade: [0.0; 4],
            body: [0.0; 2],
            previous_stage_two: 0.0,
            cross_delay: 0.0,
            colour_low: 0.0,
            dc_input: 0.0,
            dc_output: 0.0,
            output_low: 0.0,
            output_coefficient: 1.0 - (-TAU * 4_000.0 / sample_rate).exp(),
            amp,
            velocity: 0.0,
            filter_envelope: 0.0,
            pressure: 0.0,
            pressure_decay: (-1.0 / (sample_rate * PRESSURE_RELAX_SECONDS)).exp(),
            current_frequency_hz: 0.0,
            performance_pitch: 1.0,
            target_frequency_hz: 0.0,
            glide_coefficient: 1.0 - (-1.0 / (sample_rate * GLIDE_SECONDS)).exp(),
        })
    }

    pub const fn topology(&self) -> PressureChainTopology {
        self.topology
    }

    pub const fn controls(&self) -> PressureChainControls {
        self.controls
    }

    pub fn set_control(&mut self, control: PressureChainControl, value: f32) {
        self.controls.set(control, value);
    }

    pub fn set_controls(&mut self, controls: PressureChainControls) {
        self.controls = controls;
    }

    pub fn set_adsr(&mut self, adsr: AdsrConfig) {
        self.amp.set_config(adsr);
    }

    pub fn note_on(&mut self, note: u8, velocity: f32, articulation: PressureArticulation) {
        let note = note.min(127);
        let velocity = if velocity.is_finite() {
            velocity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.target_frequency_hz = midi_frequency(note);
        let pressure_control = self.controls.get(PressureChainControl::Pressure);
        let strike = ((velocity - 0.35) / 0.65).clamp(0.0, 1.0) * pressure_control;
        self.pressure = (self.pressure * 0.80 + strike * 0.72).min(1.0);
        self.velocity = velocity;
        let was_idle = self.amp.is_idle();
        if articulation == PressureArticulation::Trigger || was_idle {
            self.current_frequency_hz = self.target_frequency_hz;
            self.oscillator.set_frequency(
                self.current_frequency_hz * self.performance_pitch,
                self.sample_rate,
            );
        }
        if articulation != PressureArticulation::Slide || was_idle {
            self.filter_envelope = 1.0;
            self.amp.note_on();
        }
    }

    pub fn note_off(&mut self) {
        self.amp.note_off();
    }

    pub fn is_idle(&self) -> bool {
        self.amp.is_idle()
    }

    pub const fn pressure_level(&self) -> f32 {
        self.pressure
    }

    pub const fn amp_level(&self) -> f32 {
        self.amp.level()
    }

    pub(crate) fn set_pitch_ratio(&mut self, ratio: f32) {
        self.performance_pitch = ratio;
    }

    pub const fn current_frequency_hz(&self) -> f32 {
        self.current_frequency_hz
    }

    pub const fn target_frequency_hz(&self) -> f32 {
        self.target_frequency_hz
    }

    pub fn reset(&mut self) {
        self.oscillator.reset();
        self.cascade = [0.0; 4];
        self.body = [0.0; 2];
        self.previous_stage_two = 0.0;
        self.cross_delay = 0.0;
        self.colour_low = 0.0;
        self.dc_input = 0.0;
        self.dc_output = 0.0;
        self.output_low = 0.0;
        self.amp.reset();
        self.velocity = 0.0;
        self.filter_envelope = 0.0;
        self.pressure = 0.0;
        self.current_frequency_hz = 0.0;
        self.target_frequency_hz = 0.0;
        self.oscillator.increment = 0.0;
    }

    #[inline]
    pub fn sample(&mut self) -> [f32; 2] {
        self.advance_controls();
        self.pressure *= self.pressure_decay;
        let amp = self.amp.advance();
        if self.amp.is_idle() {
            return [0.0; 2];
        }

        self.current_frequency_hz +=
            (self.target_frequency_hz - self.current_frequency_hz) * self.glide_coefficient;
        self.oscillator.set_frequency(
            self.current_frequency_hz * self.performance_pitch,
            self.sample_rate,
        );

        let source_blend = self.value(PressureChainControl::Source);
        let shape = self.value(PressureChainControl::Shape);
        let pressure_amount = self.value(PressureChainControl::Pressure);
        let pulse_width =
            (0.16 + 0.64 * shape + 0.10 * self.pressure * pressure_amount).clamp(0.08, 0.90);
        let mut source = self.oscillator.sample(source_blend, pulse_width) * (0.72 + 0.18 * shape);

        let envelope_decay =
            interpolate(&self.decay_table, self.value(PressureChainControl::Decay));
        self.filter_envelope *= envelope_decay;
        if self.filter_envelope < 1.0e-6 {
            self.filter_envelope = 0.0;
        }
        let sweep = self.value(PressureChainControl::Sweep);
        let cutoff = (self.value(PressureChainControl::Cutoff)
            + self.filter_envelope * sweep * 0.58
            + self.pressure * pressure_amount * 0.24)
            .clamp(0.0, 1.0);
        let coefficient = interpolate(&self.cutoff_table, cutoff);
        let resonance = self.value(PressureChainControl::Resonance);
        // This causal cascade has an explicit sample of feedback delay. Keep
        // its loop gain below the Nyquist-period alternation region; colour
        // comes from the distributed cells and cross path, not instability.
        let feedback = 1.55 * resonance * resonance;

        if self.topology == PressureChainTopology::CrossFeed {
            source += self.cross_delay * (0.20 + 0.62 * resonance);
        }
        let driven = bounded_curve(source - feedback * self.cascade[3]);
        let stage_drive = 1.0 + 0.55 * resonance + 0.35 * self.pressure;
        let mut input = driven;
        for state in &mut self.cascade {
            let target = bounded_curve(input * stage_drive);
            *state += coefficient * (target - *state);
            input = *state;
        }

        let stage_two_difference = self.cascade[1] - self.previous_stage_two;
        self.previous_stage_two = self.cascade[1];
        self.cross_delay = bounded_curve(stage_two_difference * 5.0);

        let filtered = match self.topology {
            PressureChainTopology::DeepCascade => 0.88 * self.cascade[3] + 0.12 * self.cascade[2],
            PressureChainTopology::BodyTap => {
                self.body[0] += coefficient * 0.72 * (source - self.body[0]);
                self.body[1] += coefficient * 0.72 * (self.body[0] - self.body[1]);
                0.68 * self.body[1] + 0.52 * self.cascade[3]
            }
            PressureChainTopology::CrossFeed => {
                0.76 * self.cascade[3] + 0.32 * self.cascade[1] + 0.18 * self.cross_delay
            }
        };

        let bite = self.value(PressureChainControl::Bite);
        self.colour_low += 0.11 * (filtered - self.colour_low);
        let edge = filtered - self.colour_low;
        let colour_input = filtered
            + edge * bite * (1.2 + 1.6 * self.pressure)
            + self.cross_delay
                * bite
                * if self.topology == PressureChainTopology::CrossFeed {
                    0.28
                } else {
                    0.0
                };
        let coloured = bounded_curve(colour_input * (1.0 + 5.5 * bite));
        let accented_gain = 1.0 + self.pressure * pressure_amount * 0.48;
        let output = dc_block(
            coloured * amp * self.velocity * accented_gain * 0.86,
            &mut self.dc_input,
            &mut self.dc_output,
        );
        self.output_low += self.output_coefficient * (output - self.output_low);
        let output = (bounded_curve(self.output_low * 1.08) * 0.94).clamp(-0.94, 0.94);
        [output, output]
    }

    #[inline]
    fn advance_controls(&mut self) {
        for (current, target) in self.smoothed.iter_mut().zip(self.controls.0) {
            *current += (target - *current) * self.control_coefficient;
        }
    }

    #[inline]
    fn value(&self, control: PressureChainControl) -> f32 {
        self.smoothed[control.index()]
    }
}

#[inline]
fn poly_blep(phase: f32, increment: f32) -> f32 {
    if phase < increment {
        let x = phase / increment;
        x + x - x * x - 1.0
    } else if phase > 1.0 - increment {
        let x = (phase - 1.0) / increment;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[inline]
fn wrap_phase(mut phase: f32) -> f32 {
    if phase >= 1.0 {
        phase -= 1.0;
    } else if phase < 0.0 {
        phase += 1.0;
    }
    phase
}

#[inline]
fn bounded_curve(input: f32) -> f32 {
    let input = input.clamp(-3.0, 3.0);
    let squared = input * input;
    input * (27.0 + squared) / (27.0 + 9.0 * squared)
}

#[inline]
fn dc_block(input: f32, previous_input: &mut f32, previous_output: &mut f32) -> f32 {
    let output = input - *previous_input + 0.995 * *previous_output;
    *previous_input = input;
    *previous_output = output;
    output
}

#[inline]
fn interpolate(table: &[f32; TABLE_SIZE], normalized: f32) -> f32 {
    let position = normalized.clamp(0.0, 1.0) * (TABLE_SIZE - 1) as f32;
    let index = (position as usize).min(TABLE_SIZE - 2);
    let fraction = position - index as f32;
    table[index] + fraction * (table[index + 1] - table[index])
}

fn validate_sample_rate(sample_rate: f32) -> Result<(), PressureChainError> {
    if sample_rate.is_finite() && (8_000.0..=384_000.0).contains(&sample_rate) {
        Ok(())
    } else {
        Err(PressureChainError::InvalidSampleRate)
    }
}

fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureRenderSpec {
    pub adsr: AdsrConfig,
    pub sample_rate: u32,
    pub note: u8,
    pub velocity: f32,
    pub gate_seconds: f32,
    pub total_seconds: f32,
}

pub fn render_note(
    topology: PressureChainTopology,
    controls: PressureChainControls,
    spec: PressureRenderSpec,
) -> Result<Vec<[f32; 2]>, PressureChainError> {
    validate_sample_rate(spec.sample_rate as f32)?;
    if spec.note > 127 {
        return Err(PressureChainError::InvalidNote);
    }
    if !spec.velocity.is_finite() || !(0.0..=1.0).contains(&spec.velocity) {
        return Err(PressureChainError::InvalidVelocity);
    }
    if !spec.gate_seconds.is_finite()
        || !spec.total_seconds.is_finite()
        || spec.gate_seconds <= 0.0
        || spec.total_seconds <= spec.gate_seconds
        || spec.total_seconds > 60.0
    {
        return Err(PressureChainError::InvalidDuration);
    }
    let frames = (spec.total_seconds * spec.sample_rate as f32).round() as usize;
    let note_off = (spec.gate_seconds * spec.sample_rate as f32).round() as usize;
    let mut voice =
        PressureChainVoice::new(spec.sample_rate as f32, topology, controls, spec.adsr)?;
    voice.note_on(spec.note, spec.velocity, PressureArticulation::Trigger);
    let mut samples = Vec::with_capacity(frames);
    for frame in 0..frames {
        if frame == note_off {
            voice.note_off();
        }
        samples.push(voice.sample());
    }
    Ok(samples)
}

pub fn measure_high_rate_residual(
    topology: PressureChainTopology,
    controls: PressureChainControls,
    note: u8,
    frames: usize,
) -> Result<f64, PressureChainError> {
    if note > 127 || frames == 0 {
        return Err(PressureChainError::InvalidNote);
    }
    let adsr = AdsrConfig::new(0.002, 0.080, 0.8, 0.080)
        .map_err(|_| PressureChainError::InvalidEnvelope)?;
    let mut target = PressureChainVoice::new(48_000.0, topology, controls, adsr)?;
    let mut reference = PressureChainVoice::new(384_000.0, topology, controls, adsr)?;
    target.note_on(note, 0.95, PressureArticulation::Trigger);
    reference.note_on(note, 0.95, PressureArticulation::Trigger);
    let mut target_samples = Vec::with_capacity(frames);
    let mut reference_samples = Vec::with_capacity(frames);
    for _ in 0..frames {
        target_samples.push(target.sample()[0]);
        let mut high = 0.0;
        for _ in 0..8 {
            high = reference.sample()[0];
        }
        reference_samples.push(high);
    }
    Ok(fitted_residual_db(&target_samples, &reference_samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrigger_slide_refreshes_both_contours_without_resetting_pitch_or_phase() {
        for topology in PressureChainTopology::ALL {
            let mut voice = PressureChainVoice::new(
                48_000.0,
                topology,
                PressureChainControls::START,
                AdsrConfig::new(0.010, 0.020, 0.2, 0.050).unwrap(),
            )
            .unwrap();
            voice.note_on(40, 0.8, PressureArticulation::Trigger);
            for note in [52, 52] {
                for _ in 0..2_400 {
                    let sample = voice.sample();
                    assert!(sample.iter().all(|s| s.is_finite() && s.abs() <= 0.94));
                }
                let frequency = voice.current_frequency_hz;
                let phase = voice.oscillator.phase;
                let level = voice.amp.level();
                assert!((level - 0.2).abs() < 1e-6);
                assert!(voice.filter_envelope < 1.0);

                voice.note_on(note, 0.8, PressureArticulation::RetriggerSlide);
                assert_eq!(voice.filter_envelope, 1.0);
                assert_eq!(voice.amp.level(), level);
                assert_eq!(voice.oscillator.phase, phase);
                assert_eq!(voice.current_frequency_hz, frequency);
                voice.sample();
                assert!(voice.amp.level() > level);
                assert!(voice.current_frequency_hz >= frequency);
                assert!(voice.current_frequency_hz < voice.target_frequency_hz);
            }
            let filter = voice.filter_envelope;
            let amp = voice.amp;
            voice.note_on(40, 0.8, PressureArticulation::Slide);
            assert_eq!(voice.filter_envelope, filter);
            let mut expected_amp = amp;
            voice.sample();
            assert_eq!(voice.amp.level(), expected_amp.advance());
        }
    }
}
