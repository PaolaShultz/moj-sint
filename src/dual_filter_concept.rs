//! Isolated dual-filter and dual-envelope control exercise.
//!
//! This module borrows only the broad workflow concept of two routable filters
//! with dedicated brightness and loudness contours. It is not a Korg model,
//! preset, or production Moj Sint synthesis model.

use crate::dsp::oscillator::{BandlimitedOscillator, OscillatorMethod};
use crate::research::fitted_residual_db;
use std::f32::consts::PI;
use thiserror::Error;

const CONTROL_COUNT: usize = 15;
const FILTER_TABLE_SIZE: usize = 2_049;
const ENVELOPE_TABLE_SIZE: usize = 2_049;
const MIN_CUTOFF_HZ: f32 = 20.0;
const MAX_CUTOFF_HZ: f32 = 18_000.0;
const CUTOFF_SPAN_SEMITONES: f32 = 117.765_37;
const DIRECT_CONTROL_COUNT: usize = 7;
const CONTROL_SMOOTHING_SECONDS: f32 = 0.010;
const TOPOLOGY_FADE_SECONDS: f32 = 0.005;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConceptControl {
    FilterACutoff,
    FilterAResonance,
    FilterAEnvelopeDepth,
    FilterBCutoff,
    FilterBResonance,
    FilterBEnvelopeDepth,
    Routing,
    FilterAttack,
    FilterDecay,
    FilterSustain,
    FilterRelease,
    AmpAttack,
    AmpDecay,
    AmpSustain,
    AmpRelease,
}

impl ConceptControl {
    pub const ALL: [Self; CONTROL_COUNT] = [
        Self::FilterACutoff,
        Self::FilterAResonance,
        Self::FilterAEnvelopeDepth,
        Self::FilterBCutoff,
        Self::FilterBResonance,
        Self::FilterBEnvelopeDepth,
        Self::Routing,
        Self::FilterAttack,
        Self::FilterDecay,
        Self::FilterSustain,
        Self::FilterRelease,
        Self::AmpAttack,
        Self::AmpDecay,
        Self::AmpSustain,
        Self::AmpRelease,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::FilterACutoff => "A CUTOFF",
            Self::FilterAResonance => "A RESONANCE",
            Self::FilterAEnvelopeDepth => "A ENV DEPTH",
            Self::FilterBCutoff => "B CUTOFF",
            Self::FilterBResonance => "B RESONANCE",
            Self::FilterBEnvelopeDepth => "B ENV DEPTH",
            Self::Routing => "STRUCTURE",
            Self::FilterAttack => "F ATTACK",
            Self::FilterDecay => "F DECAY",
            Self::FilterSustain => "F SUSTAIN",
            Self::FilterRelease => "F RELEASE",
            Self::AmpAttack => "A ATTACK",
            Self::AmpDecay => "A DECAY",
            Self::AmpSustain => "A SUSTAIN",
            Self::AmpRelease => "A RELEASE",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConceptControls([f32; CONTROL_COUNT]);

impl ConceptControls {
    pub const MIDPOINT: Self = Self([0.5; CONTROL_COUNT]);

    pub fn new(values: [f32; CONTROL_COUNT]) -> Result<Self, ConceptError> {
        if values
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
        {
            Ok(Self(values))
        } else {
            Err(ConceptError::InvalidControl)
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

    pub const fn get(self, control: ConceptControl) -> f32 {
        self.0[control.index()]
    }

    pub const fn values(self) -> [f32; CONTROL_COUNT] {
        self.0
    }

    pub fn set(&mut self, control: ConceptControl, value: f32) {
        if value.is_finite() {
            self.0[control.index()] = value.clamp(0.0, 1.0);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConceptTopology {
    Serial,
    Parallel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConceptVariant {
    SweetSerial,
    ParallelSplit,
    CounterMotion,
}

impl ConceptVariant {
    pub const ALL: [Self; 3] = [Self::SweetSerial, Self::ParallelSplit, Self::CounterMotion];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::SweetSerial => "sweet-serial",
            Self::ParallelSplit => "parallel-split",
            Self::CounterMotion => "counter-motion",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::SweetSerial => "Sweet serial gate",
            Self::ParallelSplit => "Parallel body and edge",
            Self::CounterMotion => "Counter-motion multimode",
        }
    }

    const fn default_topology(self) -> ConceptTopology {
        match self {
            Self::SweetSerial => ConceptTopology::Serial,
            Self::ParallelSplit | Self::CounterMotion => ConceptTopology::Parallel,
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ConceptError {
    #[error("sample rate must be finite and between 8,000 and 384,000 Hz")]
    InvalidSampleRate,
    #[error("control values must be finite and between zero and one")]
    InvalidControl,
    #[error("MIDI note must be between 0 and 127")]
    InvalidNote,
    #[error("gate and render durations must be finite, positive, and ordered")]
    InvalidDuration,
}

#[derive(Clone, Copy, Debug, Default)]
struct SvfState {
    integrator_1: f32,
    integrator_2: f32,
}

#[derive(Clone, Copy, Debug)]
struct SvfOutputs {
    low: f32,
    band: f32,
    high: f32,
}

#[derive(Clone, Copy, Debug)]
enum FilterResponse {
    Low,
    Band,
    LowBand,
    HighDry,
}

impl SvfState {
    #[inline]
    fn process(&mut self, input: f32, g: f32, resonance: f32) -> SvfOutputs {
        let q = 0.5 + 11.5 * resonance * resonance;
        let k = q.recip();
        let a1 = (1.0 + g * (g + k)).recip();
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = input - self.integrator_2;
        let band = a1 * self.integrator_1 + a2 * v3;
        let low = self.integrator_2 + a2 * self.integrator_1 + a3 * v3;
        self.integrator_1 = 2.0 * band - self.integrator_1;
        self.integrator_2 = 2.0 * low - self.integrator_2;
        let high = input - k * band - low;
        SvfOutputs { low, band, high }
    }
}

#[derive(Clone, Copy, Debug)]
struct EnvelopeConfig {
    attack_coefficient: f32,
    decay_coefficient: f32,
    sustain: f32,
    release_coefficient: f32,
}

impl EnvelopeConfig {
    fn from_controls(
        sample_rate: f32,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    ) -> Self {
        Self {
            attack_coefficient: envelope_coefficient(sample_rate, envelope_seconds(attack)),
            decay_coefficient: envelope_coefficient(sample_rate, envelope_seconds(decay)),
            sustain,
            release_coefficient: envelope_coefficient(sample_rate, envelope_seconds(release)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
struct CurvedEnvelope {
    config: EnvelopeConfig,
    stage: EnvelopeStage,
    level: f32,
}

impl CurvedEnvelope {
    fn new(config: EnvelopeConfig) -> Self {
        Self {
            config,
            stage: EnvelopeStage::Idle,
            level: 0.0,
        }
    }

    fn set_config(&mut self, config: EnvelopeConfig) {
        self.config = config;
    }

    fn note_on(&mut self) {
        self.stage = EnvelopeStage::Attack;
    }

    fn note_off(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Release;
        }
    }

    #[inline]
    fn advance(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => self.level = 0.0,
            EnvelopeStage::Attack => {
                self.level += (1.0 - self.level) * self.config.attack_coefficient;
                if self.level >= 0.999 {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                }
            }
            EnvelopeStage::Decay => {
                self.level += (self.config.sustain - self.level) * self.config.decay_coefficient;
                if (self.level - self.config.sustain).abs() <= 0.001 {
                    self.level = self.config.sustain;
                    self.stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => self.level = self.config.sustain,
            EnvelopeStage::Release => {
                self.level += (0.0 - self.level) * self.config.release_coefficient;
                if self.level <= 1.0e-5 {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                }
            }
        }
        self.level
    }

    fn is_idle(self) -> bool {
        self.stage == EnvelopeStage::Idle
    }

    fn reset(&mut self) {
        self.stage = EnvelopeStage::Idle;
        self.level = 0.0;
    }
}

#[derive(Debug)]
pub struct DualFilterConceptVoice {
    variant: ConceptVariant,
    active_topology: ConceptTopology,
    topology: ConceptTopology,
    topology_gain: f32,
    topology_fade_step: f32,
    controls: ConceptControls,
    direct_controls: [f32; DIRECT_CONTROL_COUNT],
    direct_control_coefficient: f32,
    cutoff_table: [f32; FILTER_TABLE_SIZE],
    envelope_table: [f32; ENVELOPE_TABLE_SIZE],
    oscillators: [BandlimitedOscillator; 4],
    filter_a: [SvfState; 2],
    filter_b: [SvfState; 2],
    filter_envelope: CurvedEnvelope,
    amp_envelope: CurvedEnvelope,
    velocity: f32,
}

impl DualFilterConceptVoice {
    pub fn new(
        sample_rate: f32,
        variant: ConceptVariant,
        controls: ConceptControls,
    ) -> Result<Self, ConceptError> {
        validate_sample_rate(sample_rate)?;
        let oscillator = || {
            BandlimitedOscillator::new(sample_rate, OscillatorMethod::PolyBlep)
                .expect("validated sample rate")
        };
        let filter_config = envelope_config(sample_rate, controls, true);
        let amp_config = envelope_config(sample_rate, controls, false);
        let maximum_cutoff = MAX_CUTOFF_HZ.min(sample_rate * 0.45);
        let cutoff_ratio = maximum_cutoff / MIN_CUTOFF_HZ;
        let mut cutoff_table = [0.0; FILTER_TABLE_SIZE];
        for (index, coefficient) in cutoff_table.iter_mut().enumerate() {
            let normalized = index as f32 / (FILTER_TABLE_SIZE - 1) as f32;
            let cutoff_hz = MIN_CUTOFF_HZ * cutoff_ratio.powf(normalized);
            *coefficient = (PI * cutoff_hz / sample_rate).tan();
        }
        let mut envelope_table = [0.0; ENVELOPE_TABLE_SIZE];
        for (index, coefficient) in envelope_table.iter_mut().enumerate() {
            let normalized = index as f32 / (ENVELOPE_TABLE_SIZE - 1) as f32;
            *coefficient = envelope_coefficient(sample_rate, envelope_seconds(normalized));
        }
        Ok(Self {
            variant,
            active_topology: variant.default_topology(),
            topology: variant.default_topology(),
            topology_gain: 1.0,
            topology_fade_step: (sample_rate * TOPOLOGY_FADE_SECONDS).recip(),
            controls,
            direct_controls: std::array::from_fn(|index| controls.0[index]),
            direct_control_coefficient: 1.0
                - (-1.0 / (sample_rate * CONTROL_SMOOTHING_SECONDS)).exp(),
            cutoff_table,
            envelope_table,
            oscillators: [oscillator(), oscillator(), oscillator(), oscillator()],
            filter_a: [SvfState::default(); 2],
            filter_b: [SvfState::default(); 2],
            filter_envelope: CurvedEnvelope::new(filter_config),
            amp_envelope: CurvedEnvelope::new(amp_config),
            velocity: 0.0,
        })
    }

    pub const fn topology(&self) -> ConceptTopology {
        self.topology
    }

    pub const fn controls(&self) -> ConceptControls {
        self.controls
    }

    pub fn toggle_topology(&mut self) {
        self.topology = match self.topology {
            ConceptTopology::Serial => ConceptTopology::Parallel,
            ConceptTopology::Parallel => ConceptTopology::Serial,
        };
    }

    pub fn set_topology_immediate(&mut self, topology: ConceptTopology) {
        self.active_topology = topology;
        self.topology = topology;
        self.topology_gain = 1.0;
    }

    pub fn set_control(&mut self, control: ConceptControl, value: f32) {
        self.controls.set(control, value);
        self.update_envelope_configs();
    }

    pub fn set_controls(&mut self, controls: ConceptControls) {
        self.controls = controls;
        self.update_envelope_configs();
    }

    fn update_envelope_configs(&mut self) {
        let config = |attack, decay, sustain, release| EnvelopeConfig {
            attack_coefficient: interpolate_table(&self.envelope_table, attack),
            decay_coefficient: interpolate_table(&self.envelope_table, decay),
            sustain,
            release_coefficient: interpolate_table(&self.envelope_table, release),
        };
        self.filter_envelope.set_config(config(
            self.controls.get(ConceptControl::FilterAttack),
            self.controls.get(ConceptControl::FilterDecay),
            self.controls.get(ConceptControl::FilterSustain),
            self.controls.get(ConceptControl::FilterRelease),
        ));
        self.amp_envelope.set_config(config(
            self.controls.get(ConceptControl::AmpAttack),
            self.controls.get(ConceptControl::AmpDecay),
            self.controls.get(ConceptControl::AmpSustain),
            self.controls.get(ConceptControl::AmpRelease),
        ));
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        let note = note.min(127);
        let frequency = midi_frequency(note);
        let detune = match self.variant {
            ConceptVariant::SweetSerial => 1.003,
            ConceptVariant::ParallelSplit => 0.997,
            ConceptVariant::CounterMotion => 1.006,
        };
        self.oscillators[0].set_frequency(frequency);
        self.oscillators[1].set_frequency(frequency * detune);
        self.oscillators[2].set_frequency(frequency * 2.0);
        self.oscillators[3].set_frequency(frequency * 1.5 / detune);
        self.velocity = if velocity.is_finite() {
            velocity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.filter_envelope.note_on();
        self.amp_envelope.note_on();
    }

    pub fn note_off(&mut self) {
        self.filter_envelope.note_off();
        self.amp_envelope.note_off();
    }

    pub fn is_idle(&self) -> bool {
        self.amp_envelope.is_idle()
    }

    pub fn reset(&mut self) {
        for oscillator in &mut self.oscillators {
            oscillator.reset();
        }
        self.filter_a = [SvfState::default(); 2];
        self.filter_b = [SvfState::default(); 2];
        self.filter_envelope.reset();
        self.amp_envelope.reset();
        self.velocity = 0.0;
        self.active_topology = self.topology;
        self.topology_gain = 1.0;
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        self.advance_direct_controls();
        let filter_envelope = self.filter_envelope.advance();
        let amp_envelope = self.amp_envelope.advance();
        let s0 = self.oscillators[0].sample(0.05);
        let s1 = self.oscillators[1].sample(0.35);
        let s2 = self.oscillators[2].sample(0.8);
        let s3 = self.oscillators[3].sample(1.0);
        let (source_a, source_b) = match self.variant {
            ConceptVariant::SweetSerial => {
                let source = 0.27 * (s0 + s1 + s2 + s3);
                (source, source)
            }
            ConceptVariant::ParallelSplit => (0.42 * (s0 + s1), 0.38 * (s2 + s3)),
            ConceptVariant::CounterMotion => (
                0.52 * s0 + 0.34 * s1 * s2,
                0.42 * (s2 - s3) + 0.18 * s0 * s3,
            ),
        };

        let a_depth =
            envelope_depth_semitones(self.direct_control(ConceptControl::FilterAEnvelopeDepth));
        let b_depth =
            envelope_depth_semitones(self.direct_control(ConceptControl::FilterBEnvelopeDepth));
        let a_cutoff = modulated_cutoff(
            self.direct_control(ConceptControl::FilterACutoff),
            a_depth,
            filter_envelope,
        );
        let b_cutoff = modulated_cutoff(
            self.direct_control(ConceptControl::FilterBCutoff),
            b_depth,
            filter_envelope,
        );
        let a_resonance = self.direct_control(ConceptControl::FilterAResonance);
        let b_resonance = self.direct_control(ConceptControl::FilterBResonance);
        let routing = self.direct_control(ConceptControl::Routing);

        let filtered = match self.active_topology {
            ConceptTopology::Serial => {
                let source = source_a + routing * (source_b - source_a);
                let through_a = self.process_filter_a(source, a_cutoff, a_resonance);
                let into_b = through_a + routing * (source_b - through_a);
                self.process_filter_b(into_b, b_cutoff, b_resonance)
            }
            ConceptTopology::Parallel => {
                let a = self.process_filter_a(source_a, a_cutoff, a_resonance);
                let b = self.process_filter_b(source_b, b_cutoff, b_resonance);
                let gain_a = 1.0 - 0.5 * routing;
                let gain_b = 0.5 + 0.5 * routing;
                (a * gain_a + b * gain_b) * 0.72
            }
        };

        if self.amp_envelope.is_idle() {
            return 0.0;
        }
        let topology_gain = self.advance_topology_transition();
        let vca = amp_envelope * amp_envelope;
        soft_clip(filtered * vca * self.velocity * 1.35) * 0.82 * topology_gain
    }

    #[inline]
    fn advance_direct_controls(&mut self) {
        for (index, current) in self.direct_controls.iter_mut().enumerate() {
            let target = self.controls.0[index];
            *current += (target - *current) * self.direct_control_coefficient;
        }
    }

    #[inline]
    fn direct_control(&self, control: ConceptControl) -> f32 {
        self.direct_controls[control.index()]
    }

    #[inline]
    fn advance_topology_transition(&mut self) -> f32 {
        if self.active_topology != self.topology {
            self.topology_gain = (self.topology_gain - self.topology_fade_step).max(0.0);
            if self.topology_gain == 0.0 {
                self.active_topology = self.topology;
            }
        } else if self.topology_gain < 1.0 {
            self.topology_gain = (self.topology_gain + self.topology_fade_step).min(1.0);
        }
        self.topology_gain
    }

    #[inline]
    fn process_filter_a(&mut self, input: f32, cutoff: f32, resonance: f32) -> f32 {
        let response = match self.variant {
            ConceptVariant::SweetSerial | ConceptVariant::ParallelSplit => FilterResponse::Low,
            ConceptVariant::CounterMotion => FilterResponse::LowBand,
        };
        let poles = match self.variant {
            ConceptVariant::SweetSerial => 2,
            ConceptVariant::ParallelSplit | ConceptVariant::CounterMotion => 1,
        };
        process_filter(
            &self.cutoff_table,
            &mut self.filter_a,
            input,
            cutoff,
            resonance,
            response,
            poles,
        )
    }

    #[inline]
    fn process_filter_b(&mut self, input: f32, cutoff: f32, resonance: f32) -> f32 {
        let response = match self.variant {
            ConceptVariant::SweetSerial => FilterResponse::Low,
            ConceptVariant::ParallelSplit => FilterResponse::Band,
            ConceptVariant::CounterMotion => FilterResponse::HighDry,
        };
        let poles = match self.variant {
            ConceptVariant::SweetSerial | ConceptVariant::CounterMotion => 1,
            ConceptVariant::ParallelSplit => 2,
        };
        process_filter(
            &self.cutoff_table,
            &mut self.filter_b,
            input,
            cutoff,
            resonance,
            response,
            poles,
        )
    }
}

#[inline]
fn process_filter(
    table: &[f32; FILTER_TABLE_SIZE],
    states: &mut [SvfState; 2],
    input: f32,
    cutoff: f32,
    resonance: f32,
    response: FilterResponse,
    poles: usize,
) -> f32 {
    let g = interpolate_table(table, cutoff);
    let driven = soft_clip(input * (1.0 + 1.25 * resonance));
    let first = select_response(states[0].process(driven, g, resonance), response);
    let output = if poles == 2 {
        select_response(states[1].process(first, g, resonance), response)
    } else {
        first
    };
    soft_clip(output)
}

#[inline]
fn select_response(outputs: SvfOutputs, response: FilterResponse) -> f32 {
    match response {
        FilterResponse::Low => outputs.low,
        FilterResponse::Band => outputs.band,
        FilterResponse::LowBand => 0.72 * outputs.low + 0.48 * outputs.band,
        FilterResponse::HighDry => 0.72 * outputs.high + 0.28 * (outputs.low + outputs.high),
    }
}

#[inline]
fn interpolate_table(table: &[f32; FILTER_TABLE_SIZE], normalized: f32) -> f32 {
    let position = normalized.clamp(0.0, 1.0) * (FILTER_TABLE_SIZE - 1) as f32;
    let index = (position as usize).min(FILTER_TABLE_SIZE - 2);
    let fraction = position - index as f32;
    table[index] + fraction * (table[index + 1] - table[index])
}

fn envelope_config(sample_rate: f32, controls: ConceptControls, filter: bool) -> EnvelopeConfig {
    let (attack, decay, sustain, release) = if filter {
        (
            ConceptControl::FilterAttack,
            ConceptControl::FilterDecay,
            ConceptControl::FilterSustain,
            ConceptControl::FilterRelease,
        )
    } else {
        (
            ConceptControl::AmpAttack,
            ConceptControl::AmpDecay,
            ConceptControl::AmpSustain,
            ConceptControl::AmpRelease,
        )
    };
    EnvelopeConfig::from_controls(
        sample_rate,
        controls.get(attack),
        controls.get(decay),
        controls.get(sustain),
        controls.get(release),
    )
}

fn envelope_seconds(normalized: f32) -> f32 {
    const MIN_SECONDS: f32 = 0.002;
    const MAX_SECONDS: f32 = 8.0;
    MIN_SECONDS * (MAX_SECONDS / MIN_SECONDS).powf(normalized)
}

fn envelope_coefficient(sample_rate: f32, seconds: f32) -> f32 {
    1.0 - (-6.907_755 / (sample_rate * seconds)).exp()
}

fn envelope_depth_semitones(normalized: f32) -> f32 {
    -48.0 + 120.0 * normalized
}

fn modulated_cutoff(base: f32, depth_semitones: f32, envelope: f32) -> f32 {
    (base + envelope * depth_semitones / CUTOFF_SPAN_SEMITONES).clamp(0.0, 1.0)
}

#[inline]
fn soft_clip(input: f32) -> f32 {
    let input = input.clamp(-3.0, 3.0);
    let squared = input * input;
    input * (27.0 + squared) / (27.0 + 9.0 * squared)
}

fn validate_sample_rate(sample_rate: f32) -> Result<(), ConceptError> {
    if sample_rate.is_finite() && (8_000.0..=384_000.0).contains(&sample_rate) {
        Ok(())
    } else {
        Err(ConceptError::InvalidSampleRate)
    }
}

pub fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

pub fn render_concept(
    variant: ConceptVariant,
    controls: ConceptControls,
    sample_rate: u32,
    note: u8,
    gate_seconds: f32,
    total_seconds: f32,
) -> Result<Vec<f32>, ConceptError> {
    validate_sample_rate(sample_rate as f32)?;
    if note > 127 {
        return Err(ConceptError::InvalidNote);
    }
    if !gate_seconds.is_finite()
        || !total_seconds.is_finite()
        || gate_seconds <= 0.0
        || total_seconds <= gate_seconds
        || total_seconds > 60.0
    {
        return Err(ConceptError::InvalidDuration);
    }
    let sample_count = (total_seconds * sample_rate as f32).round() as usize;
    let note_off = (gate_seconds * sample_rate as f32).round() as usize;
    let mut voice = DualFilterConceptVoice::new(sample_rate as f32, variant, controls)?;
    voice.note_on(note, 0.9);
    let mut samples = Vec::with_capacity(sample_count);
    for frame in 0..sample_count {
        if frame == note_off {
            voice.note_off();
        }
        samples.push(voice.sample());
    }
    Ok(samples)
}

pub fn measure_high_rate_residual(
    variant: ConceptVariant,
    controls: ConceptControls,
    note: u8,
    frames: usize,
) -> Result<f64, ConceptError> {
    if frames == 0 || note > 127 {
        return Err(ConceptError::InvalidNote);
    }
    let seconds = (frames as f32 / 48_000.0).max(0.01);
    let target = render_concept(variant, controls, 48_000, note, seconds * 0.8, seconds)?;
    let reference = render_concept(variant, controls, 192_000, note, seconds * 0.8, seconds)?;
    let reduced: Vec<f32> = reference.chunks_exact(4).map(|chunk| chunk[0]).collect();
    Ok(fitted_residual_db(&target, &reduced))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_inputs_fail_before_rendering() {
        assert!(
            DualFilterConceptVoice::new(
                0.0,
                ConceptVariant::SweetSerial,
                ConceptControls::MIDPOINT,
            )
            .is_err()
        );
        assert!(ConceptControls::new([f32::NAN; CONTROL_COUNT]).is_err());
        assert!(
            render_concept(
                ConceptVariant::SweetSerial,
                ConceptControls::MIDPOINT,
                48_000,
                60,
                1.0,
                0.5,
            )
            .is_err()
        );
        assert!(
            measure_high_rate_residual(
                ConceptVariant::SweetSerial,
                ConceptControls::MIDPOINT,
                60,
                0,
            )
            .is_err()
        );
    }
}
