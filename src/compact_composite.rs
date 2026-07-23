//! Isolated one-to-three-mechanism composite research machines.
//!
//! This module is intentionally unreachable from the production `Engine`.

use std::sync::Arc;

use thiserror::Error;

pub const OUTPUT_CEILING: f32 = 0.999;
const TABLE_SIZE: usize = 2_048;
const MAX_MECHANISMS: usize = 3;
const MAX_EVENTS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum CompactCandidate {
    DeepRotor,
    PhaseForge,
    EvolvingLattice,
    PedalMonolith,
    D2Braid,
    ClippedThump,
    ResonantThump,
    SyntheticKick,
    StruckComb,
    ContextRelay,
}

impl CompactCandidate {
    pub const ALL: [Self; 10] = [
        Self::DeepRotor,
        Self::PhaseForge,
        Self::EvolvingLattice,
        Self::PedalMonolith,
        Self::D2Braid,
        Self::ClippedThump,
        Self::ResonantThump,
        Self::SyntheticKick,
        Self::StruckComb,
        Self::ContextRelay,
    ];

    pub fn spec(self) -> &'static CandidateSpec {
        &RETAINED_SPECS[self as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExperimentRole {
    SustainedLow,
    PlayableMid,
    Evolving,
    Pedal,
    Bass,
    BassThump,
    SyntheticKick,
    Struck,
    MusicalContext,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriveMethod {
    Linear,
    CubicSaturation,
    RationalSaturation,
    HardClip,
}

impl DriveMethod {
    pub fn slug(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::CubicSaturation => "cubic-saturation",
            Self::RationalSaturation => "rational-saturation",
            Self::HardClip => "hard-clip",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeSpec {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
    pub gate_ms: f32,
    pub pitch_drop_semitones: f32,
    pub pitch_drop_ms: f32,
}

impl EnvelopeSpec {
    pub fn is_finite_and_bounded(self) -> bool {
        self.attack_ms.is_finite()
            && self.attack_ms >= 0.0
            && self.decay_ms.is_finite()
            && self.decay_ms >= 0.0
            && self.sustain.is_finite()
            && (0.0..=1.0).contains(&self.sustain)
            && self.release_ms.is_finite()
            && self.release_ms >= 0.0
            && self.gate_ms.is_finite()
            && self.gate_ms > 0.0
            && self.pitch_drop_semitones.is_finite()
            && self.pitch_drop_semitones >= 0.0
            && self.pitch_drop_ms.is_finite()
            && self.pitch_drop_ms >= 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateSpec {
    pub candidate: CompactCandidate,
    pub slug: &'static str,
    pub role: ExperimentRole,
    pub mechanism_count: usize,
    pub mechanism_names: [&'static str; MAX_MECHANISMS],
    pub source_gains: [f32; MAX_MECHANISMS],
    pub stereo_widths: [f32; MAX_MECHANISMS],
    pub drive_method: DriveMethod,
    pub drive: f32,
    pub dc_block_hz: f32,
    pub output_gain: f32,
    pub output_ceiling: f32,
    pub envelope: EnvelopeSpec,
    pub primary_note: u8,
    pub tone_min: f32,
    pub tone_max: f32,
    pub duration_seconds: f32,
}

const SUSTAIN: EnvelopeSpec = EnvelopeSpec {
    attack_ms: 12.0,
    decay_ms: 180.0,
    sustain: 0.86,
    release_ms: 220.0,
    gate_ms: 3_700.0,
    pitch_drop_semitones: 0.0,
    pitch_drop_ms: 0.0,
};
const EVOLVING: EnvelopeSpec = EnvelopeSpec {
    attack_ms: 18.0,
    decay_ms: 240.0,
    sustain: 0.88,
    release_ms: 300.0,
    gate_ms: 5_650.0,
    pitch_drop_semitones: 0.0,
    pitch_drop_ms: 0.0,
};
const THUMP: EnvelopeSpec = EnvelopeSpec {
    attack_ms: 0.5,
    decay_ms: 95.0,
    sustain: 0.32,
    release_ms: 260.0,
    gate_ms: 210.0,
    pitch_drop_semitones: 19.0,
    pitch_drop_ms: 95.0,
};
const BODY_THUMP: EnvelopeSpec = EnvelopeSpec {
    attack_ms: 1.2,
    decay_ms: 140.0,
    sustain: 0.42,
    release_ms: 340.0,
    gate_ms: 270.0,
    pitch_drop_semitones: 7.0,
    pitch_drop_ms: 130.0,
};
const KICK: EnvelopeSpec = EnvelopeSpec {
    attack_ms: 0.25,
    decay_ms: 72.0,
    sustain: 0.18,
    release_ms: 290.0,
    gate_ms: 160.0,
    pitch_drop_semitones: 28.0,
    pitch_drop_ms: 78.0,
};
const STRUCK: EnvelopeSpec = EnvelopeSpec {
    attack_ms: 0.4,
    decay_ms: 120.0,
    sustain: 0.28,
    release_ms: 1_150.0,
    gate_ms: 190.0,
    pitch_drop_semitones: 0.0,
    pitch_drop_ms: 0.0,
};
const CONTEXT: EnvelopeSpec = EnvelopeSpec {
    attack_ms: 4.0,
    decay_ms: 90.0,
    sustain: 0.72,
    release_ms: 180.0,
    gate_ms: 760.0,
    pitch_drop_semitones: 0.0,
    pitch_drop_ms: 0.0,
};

static RETAINED_SPECS: [CandidateSpec; 10] = [
    CandidateSpec {
        candidate: CompactCandidate::DeepRotor,
        slug: "deep-rotor",
        role: ExperimentRole::SustainedLow,
        mechanism_count: 2,
        mechanism_names: ["table-sub-oscillator", "fixed-register-rotor", ""],
        source_gains: [0.94, 0.44, 0.0],
        stereo_widths: [0.0, 0.08, 0.0],
        drive_method: DriveMethod::RationalSaturation,
        drive: 3.4,
        dc_block_hz: 12.0,
        output_gain: 0.92,
        output_ceiling: OUTPUT_CEILING,
        envelope: SUSTAIN,
        primary_note: 26,
        tone_min: 0.18,
        tone_max: 0.82,
        duration_seconds: 4.0,
    },
    CandidateSpec {
        candidate: CompactCandidate::PhaseForge,
        slug: "phase-forge",
        role: ExperimentRole::PlayableMid,
        mechanism_count: 1,
        mechanism_names: ["nonlinear-phase-interaction", "", ""],
        source_gains: [1.0, 0.0, 0.0],
        stereo_widths: [0.04, 0.0, 0.0],
        drive_method: DriveMethod::CubicSaturation,
        drive: 2.9,
        dc_block_hz: 12.0,
        output_gain: 0.96,
        output_ceiling: OUTPUT_CEILING,
        envelope: SUSTAIN,
        primary_note: 50,
        tone_min: 0.15,
        tone_max: 0.78,
        duration_seconds: 4.0,
    },
    CandidateSpec {
        candidate: CompactCandidate::EvolvingLattice,
        slug: "evolving-lattice",
        role: ExperimentRole::Evolving,
        mechanism_count: 2,
        mechanism_names: ["authored-spectral-bank", "fixed-register-rotor", ""],
        source_gains: [0.82, 0.42, 0.0],
        stereo_widths: [0.18, 0.06, 0.0],
        drive_method: DriveMethod::RationalSaturation,
        drive: 3.7,
        dc_block_hz: 12.0,
        output_gain: 0.9,
        output_ceiling: OUTPUT_CEILING,
        envelope: EVOLVING,
        primary_note: 38,
        tone_min: 0.12,
        tone_max: 0.88,
        duration_seconds: 6.0,
    },
    CandidateSpec {
        candidate: CompactCandidate::PedalMonolith,
        slug: "pedal-monolith",
        role: ExperimentRole::Pedal,
        mechanism_count: 3,
        mechanism_names: [
            "table-sub-oscillator",
            "damped-resonant-body",
            "fixed-register-rotor",
        ],
        source_gains: [0.82, 0.72, 0.46],
        stereo_widths: [0.0, 0.08, 0.04],
        drive_method: DriveMethod::RationalSaturation,
        drive: 3.6,
        dc_block_hz: 12.0,
        output_gain: 0.92,
        output_ceiling: OUTPUT_CEILING,
        envelope: SUSTAIN,
        primary_note: 26,
        tone_min: 0.16,
        tone_max: 0.74,
        duration_seconds: 4.0,
    },
    CandidateSpec {
        candidate: CompactCandidate::D2Braid,
        slug: "d2-braid",
        role: ExperimentRole::Bass,
        mechanism_count: 2,
        mechanism_names: ["nonlinear-phase-interaction", "table-sub-oscillator", ""],
        source_gains: [0.72, 0.62, 0.0],
        stereo_widths: [0.06, 0.0, 0.0],
        drive_method: DriveMethod::CubicSaturation,
        drive: 3.2,
        dc_block_hz: 12.0,
        output_gain: 0.96,
        output_ceiling: OUTPUT_CEILING,
        envelope: SUSTAIN,
        primary_note: 38,
        tone_min: 0.2,
        tone_max: 0.76,
        duration_seconds: 4.0,
    },
    CandidateSpec {
        candidate: CompactCandidate::ClippedThump,
        slug: "clipped-thump",
        role: ExperimentRole::BassThump,
        mechanism_count: 1,
        mechanism_names: ["pitch-drop-table-oscillator", "", ""],
        source_gains: [1.0, 0.0, 0.0],
        stereo_widths: [0.0, 0.0, 0.0],
        drive_method: DriveMethod::HardClip,
        drive: 8.0,
        dc_block_hz: 12.0,
        output_gain: 1.0,
        output_ceiling: OUTPUT_CEILING,
        envelope: THUMP,
        primary_note: 26,
        tone_min: 0.2,
        tone_max: 0.8,
        duration_seconds: 1.25,
    },
    CandidateSpec {
        candidate: CompactCandidate::ResonantThump,
        slug: "resonant-thump",
        role: ExperimentRole::BassThump,
        mechanism_count: 2,
        mechanism_names: ["damped-resonant-body", "table-sub-oscillator", ""],
        source_gains: [0.74, 0.72, 0.0],
        stereo_widths: [0.06, 0.0, 0.0],
        drive_method: DriveMethod::RationalSaturation,
        drive: 6.0,
        dc_block_hz: 12.0,
        output_gain: 0.94,
        output_ceiling: OUTPUT_CEILING,
        envelope: BODY_THUMP,
        primary_note: 26,
        tone_min: 0.18,
        tone_max: 0.76,
        duration_seconds: 1.5,
    },
    CandidateSpec {
        candidate: CompactCandidate::SyntheticKick,
        slug: "synthetic-kick",
        role: ExperimentRole::SyntheticKick,
        mechanism_count: 3,
        mechanism_names: [
            "pitch-drop-table-oscillator",
            "noise-transient",
            "damped-resonant-body",
        ],
        source_gains: [1.0, 1.35, 0.46],
        stereo_widths: [0.0, 0.03, 0.02],
        drive_method: DriveMethod::HardClip,
        drive: 9.5,
        dc_block_hz: 12.0,
        output_gain: 1.0,
        output_ceiling: OUTPUT_CEILING,
        envelope: KICK,
        primary_note: 26,
        tone_min: 0.16,
        tone_max: 0.7,
        duration_seconds: 1.25,
    },
    CandidateSpec {
        candidate: CompactCandidate::StruckComb,
        slug: "struck-comb",
        role: ExperimentRole::Struck,
        mechanism_count: 2,
        mechanism_names: ["noise-transient", "excited-feedback-comb", ""],
        source_gains: [0.42, 0.92, 0.0],
        stereo_widths: [0.08, 0.16, 0.0],
        drive_method: DriveMethod::CubicSaturation,
        drive: 4.2,
        dc_block_hz: 12.0,
        output_gain: 0.94,
        output_ceiling: OUTPUT_CEILING,
        envelope: STRUCK,
        primary_note: 50,
        tone_min: 0.18,
        tone_max: 0.82,
        duration_seconds: 3.0,
    },
    CandidateSpec {
        candidate: CompactCandidate::ContextRelay,
        slug: "context-relay",
        role: ExperimentRole::MusicalContext,
        mechanism_count: 3,
        mechanism_names: [
            "table-sub-oscillator",
            "authored-spectral-bank",
            "fixed-register-rotor",
        ],
        source_gains: [0.67, 0.62, 0.62],
        stereo_widths: [0.0, 0.16, 0.07],
        drive_method: DriveMethod::RationalSaturation,
        drive: 3.9,
        dc_block_hz: 12.0,
        output_gain: 0.92,
        output_ceiling: OUTPUT_CEILING,
        envelope: CONTEXT,
        primary_note: 38,
        tone_min: 0.16,
        tone_max: 0.84,
        duration_seconds: 8.0,
    },
];

pub fn retained_specs() -> &'static [CandidateSpec] {
    &RETAINED_SPECS
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputPolicy {
    pub drive_method: DriveMethod,
    pub drive: f32,
    pub output_gain: f32,
    pub output_ceiling: f32,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum CompactError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error("tone must be finite and between zero and one")]
    InvalidTone,
    #[error("mute index is outside the mechanism list")]
    InvalidMute,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum EnvelopeStage {
    #[default]
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
struct PreparedEnvelope {
    attack: u32,
    decay: u32,
    sustain: f32,
    release: u32,
    stage: EnvelopeStage,
    stage_frame: u32,
    level: f32,
    release_start: f32,
}

impl PreparedEnvelope {
    fn new(sample_rate: u32, spec: EnvelopeSpec) -> Self {
        let frames = |milliseconds: f32| {
            ((milliseconds * sample_rate as f32 / 1_000.0).round() as u32).max(1)
        };
        Self {
            attack: frames(spec.attack_ms),
            decay: frames(spec.decay_ms),
            sustain: spec.sustain,
            release: frames(spec.release_ms),
            stage: EnvelopeStage::Idle,
            stage_frame: 0,
            level: 0.0,
            release_start: 0.0,
        }
    }

    fn note_on(&mut self) {
        self.stage = EnvelopeStage::Attack;
        self.stage_frame = 0;
        self.level = 0.0;
    }

    fn note_off(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Release;
            self.stage_frame = 0;
            self.release_start = self.level;
        }
    }

    #[inline]
    fn sample(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Idle => self.level = 0.0,
            EnvelopeStage::Attack => {
                self.stage_frame += 1;
                self.level = self.stage_frame as f32 / self.attack as f32;
                if self.stage_frame >= self.attack {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                    self.stage_frame = 0;
                }
            }
            EnvelopeStage::Decay => {
                self.stage_frame += 1;
                let position = self.stage_frame as f32 / self.decay as f32;
                self.level = 1.0 - (1.0 - self.sustain) * position;
                if self.stage_frame >= self.decay {
                    self.level = self.sustain;
                    self.stage = EnvelopeStage::Sustain;
                    self.stage_frame = 0;
                }
            }
            EnvelopeStage::Sustain => self.level = self.sustain,
            EnvelopeStage::Release => {
                self.stage_frame += 1;
                self.level =
                    self.release_start * (1.0 - self.stage_frame as f32 / self.release as f32);
                if self.stage_frame >= self.release {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                    self.stage_frame = 0;
                }
            }
        }
        self.level = self.level.clamp(0.0, 1.0);
        self.level
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct PreparedEvent {
    frame: usize,
    note: u8,
    frequency: f32,
    note_on: bool,
}

#[derive(Clone, Debug)]
enum MechanismState {
    Oscillator {
        phase: f32,
        base_increment: f32,
        shape: f32,
    },
    PhaseInteraction {
        carrier_phase: f32,
        mod_phase: f32,
        carrier_increment: f32,
        mod_increment: f32,
        index: f32,
    },
    Spectral {
        phases: [f32; 5],
        increments: [f32; 5],
        base_increment: f32,
        tone: f32,
    },
    Register {
        phase: u32,
        increment: u32,
        state: u32,
        low: f32,
        dc: f32,
    },
    Noise {
        state: u32,
        level: f32,
        decay: f32,
    },
    Resonator {
        state: u32,
        y1: f32,
        y2: f32,
        coefficient: f32,
        radius_squared: f32,
        excitation_frames: u32,
        frame: u32,
    },
    Comb {
        state: u32,
        delay: Box<[f32]>,
        index: usize,
        last: f32,
        feedback: f32,
        excitation_frames: u32,
        frame: u32,
    },
}

impl MechanismState {
    fn set_frequency(&mut self, frequency: f32, sample_rate: u32) {
        let increment = frequency * TABLE_SIZE as f32 / sample_rate as f32;
        match self {
            Self::Oscillator { base_increment, .. } => *base_increment = increment,
            Self::PhaseInteraction {
                carrier_increment,
                mod_increment,
                ..
            } => {
                *carrier_increment = increment;
                *mod_increment = 2.013 * increment;
            }
            Self::Spectral {
                increments,
                base_increment,
                ..
            } => {
                *base_increment = increment;
                for (index, item) in increments.iter_mut().enumerate() {
                    *item = increment * (index + 1) as f32;
                }
            }
            Self::Register {
                increment: word, ..
            } => {
                *word = ((frequency / sample_rate as f32) * u32::MAX as f32)
                    .round()
                    .max(1.0) as u32;
            }
            Self::Noise { .. } | Self::Resonator { .. } | Self::Comb { .. } => {}
        }
    }

    #[inline]
    fn sample(&mut self, table: &[f32], tone: f32, pitch_multiplier: f32, evolution: f32) -> f32 {
        match self {
            Self::Oscillator {
                phase,
                base_increment,
                shape,
            } => {
                let sine = table_sample(table, *phase);
                let harmonic = table_sample(table, 2.0 * *phase);
                *phase = wrap_phase(*phase + *base_increment * pitch_multiplier);
                let shaped = (*shape + 0.45 * tone).clamp(0.0, 0.92);
                (1.0 - 0.42 * shaped) * sine + 0.42 * shaped * harmonic
            }
            Self::PhaseInteraction {
                carrier_phase,
                mod_phase,
                carrier_increment,
                mod_increment,
                index,
            } => {
                let modulator = table_sample(table, *mod_phase);
                let shaped = modulator - (0.18 + 0.24 * tone) * modulator * modulator * modulator;
                let value = table_sample(
                    table,
                    *carrier_phase + (*index + 2.2 * tone) * shaped * 160.0,
                );
                *carrier_phase = wrap_phase(*carrier_phase + *carrier_increment * pitch_multiplier);
                *mod_phase = wrap_phase(*mod_phase + *mod_increment * pitch_multiplier);
                value
            }
            Self::Spectral {
                phases,
                increments,
                tone: base_tone,
                ..
            } => {
                let path = (0.65 * *base_tone + 0.35 * evolution).clamp(0.0, 1.0);
                let dark = [1.0, 0.32, 0.18, 0.08, 0.04];
                let bright = [0.82, 0.24, 0.46, 0.19, 0.31];
                let mut value = 0.0;
                let mut weight = 0.0;
                for index in 0..5 {
                    let amplitude = dark[index] + path * (bright[index] - dark[index]);
                    value += amplitude * table_sample(table, phases[index]);
                    weight += amplitude;
                    phases[index] =
                        wrap_phase(phases[index] + increments[index] * pitch_multiplier);
                }
                value / weight.max(1.0)
            }
            Self::Register {
                phase,
                increment,
                state,
                low,
                dc,
            } => {
                let previous = *phase;
                *phase = phase.wrapping_add(*increment);
                let carry = *phase < previous;
                *state = state.rotate_left(5).wrapping_add(*phase ^ 0x9e37_79b9)
                    ^ if carry { 0xa511_e9b3 } else { 0x63d8_3595 };
                let raw =
                    (((*state >> (17 - (tone * 5.0) as u32)) & 0x7fff) as f32 / 16_383.5) - 1.0;
                *low += (0.08 + 0.16 * (1.0 - tone)) * (raw - *low);
                *dc += 0.002 * (*low - *dc);
                *low - *dc
            }
            Self::Noise {
                state,
                level,
                decay,
            } => {
                let raw = signed_noise(state);
                let result = raw * *level;
                *level *= *decay;
                result
            }
            Self::Resonator {
                state,
                y1,
                y2,
                coefficient,
                radius_squared,
                excitation_frames,
                frame,
            } => {
                let excitation = if *frame < *excitation_frames {
                    0.16 * signed_noise(state)
                } else {
                    0.0
                };
                let output = excitation + *coefficient * *y1 - *radius_squared * *y2;
                *y2 = *y1;
                *y1 = output.clamp(-4.0, 4.0);
                *frame += 1;
                0.42 * *y1
            }
            Self::Comb {
                state,
                delay,
                index,
                last,
                feedback,
                excitation_frames,
                frame,
            } => {
                let excitation = if *frame < *excitation_frames {
                    0.28 * signed_noise(state)
                } else {
                    0.0
                };
                let delayed = delay[*index];
                *last += 0.34 * (delayed - *last);
                delay[*index] = (excitation + *feedback * *last).clamp(-2.0, 2.0);
                *index += 1;
                if *index >= delay.len() {
                    *index = 0;
                }
                *frame += 1;
                delayed
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompactMachine {
    candidate: CompactCandidate,
    sample_rate: u32,
    note: u8,
    tone: f32,
    frame: usize,
    duration_frames: usize,
    table: Arc<[f32]>,
    mechanisms: Vec<MechanismState>,
    envelope: PreparedEnvelope,
    events: [PreparedEvent; MAX_EVENTS],
    event_count: usize,
    next_event: usize,
    muted_mechanism: Option<usize>,
    pitch_multiplier: f32,
    pitch_start: f32,
    pitch_step: f32,
    pitch_frames: u32,
    pitch_frames_remaining: u32,
    mix_dc: [f32; 2],
    output_dc: [f32; 2],
    mix_dc_alpha: f32,
}

impl CompactMachine {
    pub fn new(
        candidate: CompactCandidate,
        sample_rate: u32,
        note: u8,
        tone: f32,
        muted_mechanism: Option<usize>,
    ) -> Result<Self, CompactError> {
        if sample_rate == 0 {
            return Err(CompactError::InvalidSampleRate);
        }
        if !tone.is_finite() || !(0.0..=1.0).contains(&tone) {
            return Err(CompactError::InvalidTone);
        }
        let spec = candidate.spec();
        if muted_mechanism.is_some_and(|index| index >= spec.mechanism_count) {
            return Err(CompactError::InvalidMute);
        }
        let table = build_sine_table();
        let frequency = midi_frequency(note);
        let mechanisms = build_mechanisms(candidate, sample_rate, frequency, tone);
        debug_assert_eq!(mechanisms.len(), spec.mechanism_count);
        let duration_frames = (spec.duration_seconds * sample_rate as f32).round() as usize;
        let (events, event_count) = build_events(candidate, sample_rate, note, duration_frames);
        let pitch_multiplier = 2.0_f32.powf(spec.envelope.pitch_drop_semitones / 12.0);
        let pitch_frames =
            (spec.envelope.pitch_drop_ms * sample_rate as f32 / 1_000.0).round() as u32;
        let pitch_step = if pitch_frames > 0 {
            (pitch_multiplier - 1.0) / pitch_frames as f32
        } else {
            0.0
        };
        Ok(Self {
            candidate,
            sample_rate,
            note,
            tone,
            frame: 0,
            duration_frames,
            table,
            mechanisms,
            envelope: PreparedEnvelope::new(sample_rate, spec.envelope),
            events,
            event_count,
            next_event: 0,
            muted_mechanism,
            pitch_multiplier,
            pitch_start: pitch_multiplier,
            pitch_step,
            pitch_frames,
            pitch_frames_remaining: pitch_frames,
            mix_dc: [0.0; 2],
            output_dc: [0.0; 2],
            mix_dc_alpha: 1.0
                - (-std::f32::consts::TAU * spec.dc_block_hz / sample_rate as f32).exp(),
        })
    }

    pub fn mechanism_count(&self) -> usize {
        self.mechanisms.len()
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }

    pub fn current_pitch_multiplier(&self) -> f32 {
        self.pitch_multiplier
    }

    pub fn event_frames(&self) -> Vec<usize> {
        self.events[..self.event_count]
            .iter()
            .map(|event| event.frame)
            .collect()
    }

    #[inline]
    pub fn sample(&mut self) -> [f32; 2] {
        while self.next_event < self.event_count && self.events[self.next_event].frame == self.frame
        {
            let event = self.events[self.next_event];
            if event.note_on {
                self.envelope.note_on();
                for mechanism in &mut self.mechanisms {
                    mechanism.set_frequency(event.frequency, self.sample_rate);
                }
                self.note = event.note;
                self.pitch_multiplier = self.pitch_start;
                self.pitch_frames_remaining = self.pitch_frames;
            } else {
                self.envelope.note_off();
            }
            self.next_event += 1;
        }
        let envelope = self.envelope.sample();
        let evolution = triangle(self.frame as f32 / self.duration_frames.max(1) as f32);
        let mut left = 0.0;
        let mut right = 0.0;
        let spec = self.candidate.spec();
        for (index, mechanism) in self.mechanisms.iter_mut().enumerate() {
            if self.muted_mechanism == Some(index) {
                continue;
            }
            let value = mechanism.sample(&self.table, self.tone, self.pitch_multiplier, evolution)
                * spec.source_gains[index]
                * envelope;
            let width = spec.stereo_widths[index];
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            left += value * (1.0 + sign * width);
            right += value * (1.0 - sign * width);
        }
        if self.pitch_frames_remaining > 0 {
            self.pitch_frames_remaining -= 1;
            if self.pitch_frames_remaining == 0 {
                self.pitch_multiplier = 1.0;
            } else {
                self.pitch_multiplier = (self.pitch_multiplier - self.pitch_step).max(1.0);
            }
        }
        self.frame = self.frame.saturating_add(1);
        self.mix_dc[0] += self.mix_dc_alpha * (left - self.mix_dc[0]);
        self.mix_dc[1] += self.mix_dc_alpha * (right - self.mix_dc[1]);
        let driven = apply_output_stage(left - self.mix_dc[0], right - self.mix_dc[1], spec);
        let written_ceiling = spec.output_ceiling - 1.0e-6;
        let emitted = [
            (driven[0] - self.output_dc[0]).clamp(-written_ceiling, written_ceiling),
            (driven[1] - self.output_dc[1]).clamp(-written_ceiling, written_ceiling),
        ];
        self.output_dc[0] += self.mix_dc_alpha * emitted[0];
        self.output_dc[1] += self.mix_dc_alpha * emitted[1];
        emitted
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompactMetrics {
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub crest_factor: f64,
    pub maximum_jump: f64,
    pub low_rms: f64,
    pub correlation: f64,
    pub side_to_mid: f64,
    pub low_side_to_mid: f64,
    pub mono_rms: f64,
    pub mono_to_stereo_db: f64,
    pub perceptual_loudness_db: f64,
    pub clipped_proportion: f64,
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompactRender {
    pub samples: Vec<f32>,
    pub metrics: CompactMetrics,
    pub policy: OutputPolicy,
    pub event_frames: Vec<usize>,
}

pub fn render_candidate(
    candidate: CompactCandidate,
    sample_rate: u32,
    note: u8,
    tone: f32,
    muted_mechanism: Option<usize>,
) -> Result<CompactRender, CompactError> {
    let mut machine = CompactMachine::new(candidate, sample_rate, note, tone, muted_mechanism)?;
    let event_frames = machine.event_frames();
    let mut samples = Vec::with_capacity(2 * machine.duration_frames());
    for _ in 0..machine.duration_frames() {
        let frame = machine.sample();
        samples.extend(frame);
    }
    let metrics = measure(&samples, sample_rate);
    let spec = candidate.spec();
    Ok(CompactRender {
        samples,
        metrics,
        policy: OutputPolicy {
            drive_method: spec.drive_method,
            drive: spec.drive,
            output_gain: spec.output_gain,
            output_ceiling: spec.output_ceiling,
        },
        event_frames,
    })
}

pub fn measure(samples: &[f32], sample_rate: u32) -> CompactMetrics {
    if samples.is_empty() || samples.len() % 2 != 0 || sample_rate == 0 {
        return CompactMetrics {
            peak: 0.0,
            rms: 0.0,
            dc: 0.0,
            crest_factor: 0.0,
            maximum_jump: 0.0,
            low_rms: 0.0,
            correlation: 0.0,
            side_to_mid: 0.0,
            low_side_to_mid: 0.0,
            mono_rms: 0.0,
            mono_to_stereo_db: 0.0,
            perceptual_loudness_db: f64::NEG_INFINITY,
            clipped_proportion: 0.0,
            sample_hash: 0,
            finite: false,
        };
    }
    let frames = samples.len() / 2;
    let mut energy = 0.0;
    let mut mid_energy = 0.0;
    let mut side_energy = 0.0;
    let mut left_energy = 0.0;
    let mut right_energy = 0.0;
    let mut cross = 0.0;
    let mut peak = 0.0_f64;
    let mut dc = 0.0;
    let mut clipped = 0;
    let mut maximum_jump = 0.0_f64;
    let mut previous = [0.0_f64; 2];
    let low_alpha = 1.0 - (-std::f64::consts::TAU * 160.0 / f64::from(sample_rate)).exp();
    let high_pass_alpha = 1.0 - (-std::f64::consts::TAU * 60.0 / f64::from(sample_rate)).exp();
    let mut low_mid = 0.0;
    let mut low_side = 0.0;
    let mut low_mid_energy = 0.0;
    let mut low_side_energy = 0.0;
    let mut perceptual_low = 0.0;
    let mut previous_weighted = 0.0;
    let mut perceptual_energy = 0.0;
    let mut finite = true;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for frame in samples.chunks_exact(2) {
        let left = f64::from(frame[0]);
        let right = f64::from(frame[1]);
        finite &= left.is_finite() && right.is_finite();
        peak = peak.max(left.abs()).max(right.abs());
        energy += left * left + right * right;
        left_energy += left * left;
        right_energy += right * right;
        cross += left * right;
        let mid = 0.5 * (left + right);
        let side = 0.5 * (left - right);
        mid_energy += mid * mid;
        side_energy += side * side;
        low_mid += low_alpha * (mid - low_mid);
        low_side += low_alpha * (side - low_side);
        low_mid_energy += low_mid * low_mid;
        low_side_energy += low_side * low_side;
        perceptual_low += high_pass_alpha * (mid - perceptual_low);
        let high_passed = mid - perceptual_low;
        let weighted = high_passed + 0.45 * (high_passed - previous_weighted);
        previous_weighted = high_passed;
        perceptual_energy += weighted * weighted;
        dc += mid;
        maximum_jump = maximum_jump
            .max((left - previous[0]).abs())
            .max((right - previous[1]).abs());
        previous = [left, right];
        clipped += usize::from(left.abs() >= f64::from(OUTPUT_CEILING) - 2.0e-6);
        clipped += usize::from(right.abs() >= f64::from(OUTPUT_CEILING) - 2.0e-6);
        for sample in frame {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let rms = (energy / samples.len() as f64).sqrt();
    let mono_rms = (mid_energy / frames as f64).sqrt();
    CompactMetrics {
        peak,
        rms,
        dc: dc / frames as f64,
        crest_factor: peak / rms.max(1.0e-24),
        maximum_jump,
        low_rms: (low_mid_energy / frames as f64).sqrt(),
        correlation: cross / (left_energy * right_energy).sqrt().max(1.0e-24),
        side_to_mid: side_energy / mid_energy.max(1.0e-24),
        low_side_to_mid: low_side_energy / low_mid_energy.max(1.0e-24),
        mono_rms,
        mono_to_stereo_db: amplitude_db(mono_rms / rms.max(1.0e-24)),
        perceptual_loudness_db: 10.0 * (perceptual_energy / frames as f64).max(1.0e-24).log10(),
        clipped_proportion: clipped as f64 / samples.len() as f64,
        sample_hash: hash,
        finite,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariantRow {
    pub note: u8,
    pub tone: f32,
    pub metrics: CompactMetrics,
    pub fundamental_db: f64,
    pub mono_fundamental_loss_db: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariantRetention {
    pub tone_rows: Vec<VariantRow>,
    pub pitch_rows: Vec<VariantRow>,
    pub tone_rms_db_span: f64,
    pub tone_loudness_db_span: f64,
}

pub fn measure_variant_retention(
    candidate: CompactCandidate,
    sample_rate: u32,
) -> Result<VariantRetention, CompactError> {
    let spec = candidate.spec();
    let middle_tone = 0.5 * (spec.tone_min + spec.tone_max);
    let mut tone_rows = Vec::with_capacity(3);
    for tone in [spec.tone_min, middle_tone, spec.tone_max] {
        tone_rows.push(variant_row(
            candidate,
            sample_rate,
            spec.primary_note,
            tone,
        )?);
    }
    let mut pitch_rows = Vec::with_capacity(3);
    for note in [26, 38, 50] {
        pitch_rows.push(variant_row(candidate, sample_rate, note, middle_tone)?);
    }
    let rms_levels: Vec<_> = tone_rows
        .iter()
        .map(|row| amplitude_db(row.metrics.rms))
        .collect();
    let loudness: Vec<_> = tone_rows
        .iter()
        .map(|row| row.metrics.perceptual_loudness_db)
        .collect();
    Ok(VariantRetention {
        tone_rms_db_span: value_span(&rms_levels),
        tone_loudness_db_span: value_span(&loudness),
        tone_rows,
        pitch_rows,
    })
}

fn variant_row(
    candidate: CompactCandidate,
    sample_rate: u32,
    note: u8,
    tone: f32,
) -> Result<VariantRow, CompactError> {
    let render = render_candidate(candidate, sample_rate, note, tone, None)?;
    let mono = fold_to_mono(&render.samples);
    let duration = render.samples.len() as f64 / (2.0 * f64::from(sample_rate));
    let (active_start, active_end) = match candidate.spec().role {
        ExperimentRole::BassThump | ExperimentRole::SyntheticKick | ExperimentRole::Struck => {
            (0.0, duration.min(0.4))
        }
        _ => (0.25, (duration - 0.25).max(0.26)),
    };
    let start =
        ((active_start * f64::from(sample_rate)).round() as usize).min(render.samples.len() / 2);
    let end =
        ((active_end * f64::from(sample_rate)).round() as usize).min(render.samples.len() / 2);
    let metrics = measure(
        &render.samples[2 * start..2 * end.max(start + 1)],
        sample_rate,
    );
    let fundamental = projection_db(&render.samples, sample_rate, midi_frequency(note));
    let mono_fundamental = projection_db(&mono, sample_rate, midi_frequency(note));
    Ok(VariantRow {
        note,
        tone,
        metrics,
        fundamental_db: fundamental,
        mono_fundamental_loss_db: (fundamental - mono_fundamental).max(0.0),
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventMetrics {
    pub window_rms: [f64; 5],
    pub window_peak: [f64; 5],
    pub momentary_loudness_db: f64,
    pub onset_to_body_energy: f64,
    pub low_band_transient_rms: f64,
    pub fundamental_start_db: f64,
    pub fundamental_body_db: f64,
    pub decay_60db_ms: f64,
    pub tail_ms: f64,
    pub clipped_proportion: f64,
    pub maximum_jump: f64,
    pub dc: f64,
    pub return_to_zero: f64,
}

pub fn measure_event(samples: &[f32], sample_rate: u32, note: u8) -> EventMetrics {
    let windows = [10_u32, 25, 50, 100, 250];
    let mut window_rms = [0.0; 5];
    let mut window_peak = [0.0; 5];
    for (index, milliseconds) in windows.into_iter().enumerate() {
        let frames = ((milliseconds as usize * sample_rate as usize) / 1_000)
            .max(1)
            .min(samples.len() / 2);
        let window = &samples[..2 * frames];
        window_rms[index] = stereo_rms(window);
        window_peak[index] = window
            .iter()
            .map(|sample| f64::from(sample.abs()))
            .fold(0.0, f64::max);
    }
    let complete = measure(samples, sample_rate);
    let transient_frames = (sample_rate as usize / 4).min(samples.len() / 2);
    let transient = measure(&samples[..2 * transient_frames], sample_rate);
    let start_begin = sample_rate as usize / 200;
    let start_end = (sample_rate as usize / 40).min(samples.len() / 2);
    let body_begin = (sample_rate as usize / 10).min(samples.len() / 2);
    let body_end = (sample_rate as usize / 4).min(samples.len() / 2);
    let frequency = midi_frequency(note);
    let fundamental_start_db = projection_db(
        &samples[2 * start_begin..2 * start_end.max(start_begin + 1)],
        sample_rate,
        frequency,
    );
    let fundamental_body_db = projection_db(
        &samples[2 * body_begin..2 * body_end.max(body_begin + 1)],
        sample_rate,
        frequency,
    );
    let peak = complete.peak.max(1.0e-12);
    let mut last_above_60 = 0;
    let mut last_audible = 0;
    for (index, frame) in samples.chunks_exact(2).enumerate() {
        let magnitude = f64::from(frame[0].abs().max(frame[1].abs()));
        if magnitude >= peak * 0.001 {
            last_above_60 = index;
        }
        if magnitude >= 1.0e-4 {
            last_audible = index;
        }
    }
    let tail_frames = (sample_rate as usize / 20).min(samples.len() / 2);
    let return_to_zero = samples[samples.len() - 2 * tail_frames..]
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max) as f64;
    EventMetrics {
        window_rms,
        window_peak,
        momentary_loudness_db: transient.perceptual_loudness_db,
        onset_to_body_energy: window_rms[2].powi(2) / window_rms[4].powi(2).max(1.0e-24),
        low_band_transient_rms: transient.low_rms,
        fundamental_start_db,
        fundamental_body_db,
        decay_60db_ms: 1_000.0 * last_above_60 as f64 / f64::from(sample_rate),
        tail_ms: 1_000.0 * last_audible as f64 / f64::from(sample_rate),
        clipped_proportion: complete.clipped_proportion,
        maximum_jump: complete.maximum_jump,
        dc: complete.dc,
        return_to_zero,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AblationEvidence {
    pub mechanism_index: usize,
    pub mechanism_name: &'static str,
    pub difference_db: f64,
    pub early_difference_db: f64,
    pub loudness_change_db: f64,
    pub complete_hash: u64,
    pub muted_hash: u64,
}

pub fn measure_ablations(
    candidate: CompactCandidate,
    sample_rate: u32,
) -> Result<Vec<AblationEvidence>, CompactError> {
    let spec = candidate.spec();
    let tone = 0.5 * (spec.tone_min + spec.tone_max);
    let complete = render_candidate(candidate, sample_rate, spec.primary_note, tone, None)?;
    let mut rows = Vec::with_capacity(spec.mechanism_count);
    for index in 0..spec.mechanism_count {
        let muted = render_candidate(candidate, sample_rate, spec.primary_note, tone, Some(index))?;
        let difference = complete
            .samples
            .iter()
            .zip(&muted.samples)
            .map(|(complete, muted)| {
                let value = f64::from(*complete) - f64::from(*muted);
                value * value
            })
            .sum::<f64>();
        let difference_rms = (difference / complete.samples.len().max(1) as f64).sqrt();
        let early_samples = (sample_rate as usize / 4 * 2)
            .min(complete.samples.len())
            .min(muted.samples.len());
        let early_difference = complete.samples[..early_samples]
            .iter()
            .zip(&muted.samples[..early_samples])
            .map(|(complete, muted)| {
                let value = f64::from(*complete) - f64::from(*muted);
                value * value
            })
            .sum::<f64>();
        let early_difference_rms = (early_difference / early_samples.max(1) as f64).sqrt();
        let early_complete_rms = stereo_rms(&complete.samples[..early_samples]);
        rows.push(AblationEvidence {
            mechanism_index: index,
            mechanism_name: spec.mechanism_names[index],
            difference_db: amplitude_db(difference_rms / complete.metrics.rms.max(1.0e-24)),
            early_difference_db: amplitude_db(
                early_difference_rms / early_complete_rms.max(1.0e-24),
            ),
            loudness_change_db: muted.metrics.perceptual_loudness_db
                - complete.metrics.perceptual_loudness_db,
            complete_hash: complete.metrics.sample_hash,
            muted_hash: muted.metrics.sample_hash,
        });
    }
    Ok(rows)
}

pub fn measure_high_rate_residual(
    candidate: CompactCandidate,
    note: u8,
    tone: f32,
    sample_rate: u32,
    frames: usize,
) -> Result<f64, CompactError> {
    const FACTOR: u32 = 4;
    let target = render_candidate(candidate, sample_rate, note, tone, None)?;
    let reference = render_candidate(candidate, sample_rate * FACTOR, note, tone, None)?;
    let count = frames
        .min(target.samples.len() / 2)
        .min(reference.samples.len() / (2 * FACTOR as usize));
    let mut target_mid = Vec::with_capacity(count);
    let mut reference_mid = Vec::with_capacity(count);
    for frame in 0..count {
        target_mid.push(0.5 * (target.samples[2 * frame] + target.samples[2 * frame + 1]));
        let mut sum = 0.0;
        for high in 0..FACTOR as usize {
            let index = 2 * (frame * FACTOR as usize + high);
            sum += 0.5 * (reference.samples[index] + reference.samples[index + 1]);
        }
        reference_mid.push(sum / FACTOR as f32);
    }
    Ok(fitted_residual_db(&target_mid, &reference_mid))
}

pub fn fold_to_mono(samples: &[f32]) -> Vec<f32> {
    let mut mono = samples.to_vec();
    for frame in mono.chunks_exact_mut(2) {
        let mid = 0.5 * (frame[0] + frame[1]);
        frame[0] = mid;
        frame[1] = mid;
    }
    mono
}

pub fn projection_db(samples: &[f32], sample_rate: u32, frequency: f32) -> f64 {
    if samples.is_empty() || sample_rate == 0 {
        return -240.0;
    }
    let frames = samples.len() / 2;
    let mut real = 0.0;
    let mut imaginary = 0.0;
    for (index, frame) in samples.chunks_exact(2).enumerate() {
        let phase =
            std::f64::consts::TAU * f64::from(frequency) * index as f64 / f64::from(sample_rate);
        let mid = 0.5 * f64::from(frame[0] + frame[1]);
        real += mid * phase.cos();
        imaginary -= mid * phase.sin();
    }
    amplitude_db(2.0 * (real * real + imaginary * imaginary).sqrt() / frames.max(1) as f64)
}

fn fitted_residual_db(target: &[f32], reference: &[f32]) -> f64 {
    let mut dot = 0.0;
    let mut reference_energy = 0.0;
    let mut target_energy = 0.0;
    for (&target, &reference) in target.iter().zip(reference) {
        dot += f64::from(target) * f64::from(reference);
        reference_energy += f64::from(reference).powi(2);
        target_energy += f64::from(target).powi(2);
    }
    let gain = dot / reference_energy.max(1.0e-24);
    let residual = target
        .iter()
        .zip(reference)
        .map(|(&target, &reference)| (f64::from(target) - gain * f64::from(reference)).powi(2))
        .sum::<f64>();
    10.0 * (residual / target_energy.max(1.0e-24)).max(1.0e-24).log10()
}

fn stereo_rms(samples: &[f32]) -> f64 {
    (samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / samples.len().max(1) as f64)
        .sqrt()
}

fn value_span(values: &[f64]) -> f64 {
    let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    maximum - minimum
}

fn build_sine_table() -> Arc<[f32]> {
    (0..TABLE_SIZE)
        .map(|index| (std::f32::consts::TAU * index as f32 / TABLE_SIZE as f32).sin())
        .collect::<Vec<_>>()
        .into()
}

fn build_mechanisms(
    candidate: CompactCandidate,
    sample_rate: u32,
    frequency: f32,
    tone: f32,
) -> Vec<MechanismState> {
    let oscillator = |shape| MechanismState::Oscillator {
        phase: 0.0,
        base_increment: frequency * TABLE_SIZE as f32 / sample_rate as f32,
        shape,
    };
    let phase = || MechanismState::PhaseInteraction {
        carrier_phase: 0.0,
        mod_phase: 371.0,
        carrier_increment: frequency * TABLE_SIZE as f32 / sample_rate as f32,
        mod_increment: 2.013 * frequency * TABLE_SIZE as f32 / sample_rate as f32,
        index: 0.7,
    };
    let spectral = || {
        let base = frequency * TABLE_SIZE as f32 / sample_rate as f32;
        MechanismState::Spectral {
            phases: [0.0, 181.0, 419.0, 733.0, 1_101.0],
            increments: std::array::from_fn(|index| base * (index + 1) as f32),
            base_increment: base,
            tone,
        }
    };
    let register = || MechanismState::Register {
        phase: 0,
        increment: ((frequency / sample_rate as f32) * u32::MAX as f32)
            .round()
            .max(1.0) as u32,
        state: 0x6d2b_79f5,
        low: 0.0,
        dc: 0.0,
    };
    let noise = |seed: u32, milliseconds: f32| MechanismState::Noise {
        state: seed,
        level: 1.0,
        decay: 0.001_f32.powf(1.0 / (milliseconds * sample_rate as f32 / 1_000.0).max(1.0)),
    };
    let resonator = |seed: u32| {
        let radius = 0.9975 - 0.0012 * tone;
        MechanismState::Resonator {
            state: seed,
            y1: 0.0,
            y2: 0.0,
            coefficient: 2.0
                * radius
                * (std::f32::consts::TAU * frequency / sample_rate as f32).cos(),
            radius_squared: radius * radius,
            excitation_frames: (0.012 * sample_rate as f32).round() as u32,
            frame: 0,
        }
    };
    let comb = |seed: u32| {
        let length = (sample_rate as f32 / frequency).round().clamp(8.0, 4_096.0) as usize;
        MechanismState::Comb {
            state: seed,
            delay: vec![0.0; length].into_boxed_slice(),
            index: 0,
            last: 0.0,
            feedback: 0.965 - 0.025 * tone,
            excitation_frames: (0.016 * sample_rate as f32).round() as u32,
            frame: 0,
        }
    };
    match candidate {
        CompactCandidate::DeepRotor => vec![oscillator(0.1), register()],
        CompactCandidate::PhaseForge => vec![phase()],
        CompactCandidate::EvolvingLattice => vec![spectral(), register()],
        CompactCandidate::PedalMonolith => {
            vec![oscillator(0.05), resonator(0xa511_0026), register()]
        }
        CompactCandidate::D2Braid => vec![phase(), oscillator(0.12)],
        CompactCandidate::ClippedThump => vec![oscillator(0.02)],
        CompactCandidate::ResonantThump => {
            vec![resonator(0xb0d1_0026), oscillator(0.04)]
        }
        CompactCandidate::SyntheticKick => vec![
            oscillator(0.0),
            noise(0xc1c1_0026, 28.0),
            resonator(0xcafe_0026),
        ],
        CompactCandidate::StruckComb => {
            vec![noise(0x51e1_0050, 35.0), comb(0xc0ab_0050)]
        }
        CompactCandidate::ContextRelay => vec![oscillator(0.08), spectral(), register()],
    }
}

fn build_events(
    candidate: CompactCandidate,
    sample_rate: u32,
    note: u8,
    duration_frames: usize,
) -> ([PreparedEvent; MAX_EVENTS], usize) {
    let mut events = [PreparedEvent::default(); MAX_EVENTS];
    if candidate == CompactCandidate::ContextRelay {
        let intervals = [0_i16, 7, 12, 10, 0, 15, 7, 12];
        let mut count = 0;
        for (segment, interval) in intervals.into_iter().enumerate() {
            let event_note = (i16::from(note) + interval).clamp(0, 127) as u8;
            let start = segment * sample_rate as usize;
            let stop = start
                + (candidate.spec().envelope.gate_ms * sample_rate as f32 / 1_000.0).round()
                    as usize;
            events[count] = PreparedEvent {
                frame: start,
                note: event_note,
                frequency: midi_frequency(event_note),
                note_on: true,
            };
            count += 1;
            events[count] = PreparedEvent {
                frame: stop.min(duration_frames.saturating_sub(1)),
                note: event_note,
                frequency: midi_frequency(event_note),
                note_on: false,
            };
            count += 1;
        }
        (events, count)
    } else {
        events[0] = PreparedEvent {
            frame: 0,
            note,
            frequency: midi_frequency(note),
            note_on: true,
        };
        events[1] = PreparedEvent {
            frame: ((candidate.spec().envelope.gate_ms * sample_rate as f32 / 1_000.0).round()
                as usize)
                .min(duration_frames.saturating_sub(1)),
            note,
            frequency: midi_frequency(note),
            note_on: false,
        };
        (events, 2)
    }
}

#[inline]
fn table_sample(table: &[f32], phase: f32) -> f32 {
    let wrapped = wrap_phase(phase);
    let index = wrapped as usize;
    let fraction = wrapped - index as f32;
    let next = if index + 1 == TABLE_SIZE {
        0
    } else {
        index + 1
    };
    table[index] + fraction * (table[next] - table[index])
}

#[inline]
fn wrap_phase(phase: f32) -> f32 {
    let mut wrapped = phase;
    while wrapped >= TABLE_SIZE as f32 {
        wrapped -= TABLE_SIZE as f32;
    }
    while wrapped < 0.0 {
        wrapped += TABLE_SIZE as f32;
    }
    wrapped
}

#[inline]
fn signed_noise(state: &mut u32) -> f32 {
    let mut value = *state;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    *state = value;
    (value as f32 / u32::MAX as f32) * 2.0 - 1.0
}

#[inline]
fn triangle(position: f32) -> f32 {
    1.0 - (2.0 * position.fract() - 1.0).abs()
}

#[inline]
fn apply_output_stage(left: f32, right: f32, spec: &CandidateSpec) -> [f32; 2] {
    let process = |sample: f32| {
        let driven = sample * spec.drive;
        let shaped = match spec.drive_method {
            DriveMethod::Linear => driven,
            DriveMethod::CubicSaturation => {
                let bounded = driven.clamp(-1.0, 1.0);
                1.5 * bounded - 0.5 * bounded * bounded * bounded
            }
            DriveMethod::RationalSaturation => 2.0 * driven / (1.0 + driven.abs()),
            DriveMethod::HardClip => driven.clamp(-1.0, 1.0),
        };
        let written_ceiling = spec.output_ceiling - 1.0e-6;
        (shaped * spec.output_gain).clamp(-written_ceiling, written_ceiling)
    };
    [process(left), process(right)]
}

pub fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

fn amplitude_db(amplitude: f64) -> f64 {
    20.0 * amplitude.max(1.0e-12).log10()
}
