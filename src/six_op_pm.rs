// Independently authored offline listening inventory: no factory voice or
// SysEx data, no compatibility claim, and no production engine or host path.

use thiserror::Error;

pub mod measurements;
pub use measurements::{
    AliasRow, EvidenceStatus, GateEvidence, MeasurementError, PairRmsComparison, PatchEvidence,
    PitchRow, RenderMetrics, SpectralRow, SpectralStage, measure_listening_gate,
};

use crate::dsp::six_op_pm::algorithm::CLASSIC_ALGORITHMS;
use crate::dsp::six_op_pm::operator::SharedSineTable;
use crate::dsp::six_op_pm::{
    EnvelopeSpec, FrequencyMode, KeyboardScaling, LfoSpec, OperatorSpec, PitchEnvelopeSpec,
    SineTable, SixOpPatch, SixOpVoice, VoiceError,
};

const REFERENCE_SAMPLE_RATE: u32 = 48_000;
type OperatorTuple = (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32);

const BELL_EVENTS: [ScoreEvent; 6] = [
    ScoreEvent::on(0, 60, 110),
    ScoreEvent::off(42_240, 60),
    ScoreEvent::on(57_600, 72, 94),
    ScoreEvent::off(98_400, 72),
    ScoreEvent::on(117_600, 48, 122),
    ScoreEvent::off(160_800, 48),
];
const MALLET_EVENTS: [ScoreEvent; 8] = [
    ScoreEvent::on(0, 60, 100),
    ScoreEvent::off(36_000, 60),
    ScoreEvent::on(55_200, 60, 114),
    ScoreEvent::on(55_200, 64, 98),
    ScoreEvent::on(55_200, 67, 105),
    ScoreEvent::off(110_400, 60),
    ScoreEvent::off(110_400, 64),
    ScoreEvent::off(110_400, 67),
];
const BRASS_EVENTS: [ScoreEvent; 8] = [
    ScoreEvent::on(0, 36, 116),
    ScoreEvent::off(26_400, 36),
    ScoreEvent::on(34_560, 43, 96),
    ScoreEvent::off(60_000, 43),
    ScoreEvent::on(68_160, 48, 108),
    ScoreEvent::off(91_200, 48),
    ScoreEvent::on(96_000, 36, 124),
    ScoreEvent::off(124_800, 36),
];

const BELL_OPERATORS: [OperatorTuple; 6] = [
    (1.0, 0.54, 0.003, 0.18, 1.8, 2.4, 1.0, 0.62, 0.18, 0.25, 0.0),
    (
        1.414, 0.68, 0.002, 0.09, 0.55, 1.1, 1.0, 0.48, 0.05, 0.55, 0.0,
    ),
    (1.0, 0.40, 0.002, 0.24, 2.4, 2.8, 1.0, 0.70, 0.22, 0.18, 0.0),
    (
        2.0, 0.66, 0.001, 0.08, 0.70, 1.2, 1.0, 0.42, 0.03, 0.50, 0.0,
    ),
    (
        1.414, 0.62, 0.001, 0.06, 0.42, 0.9, 1.0, 0.35, 0.02, 0.45, 0.0,
    ),
    (
        2.71, 0.52, 0.001, 0.04, 0.28, 0.7, 1.0, 0.24, 0.0, 0.40, 0.0,
    ),
];
const FRACTURED_OPERATORS: [OperatorTuple; 6] = [
    (
        1.0, 0.74, 0.002, 0.12, 1.2, 1.7, 1.0, 0.50, 0.12, 0.30, -3.0,
    ),
    (
        1.731, 0.72, 0.001, 0.05, 0.38, 0.8, 1.0, 0.30, 0.01, 0.65, 2.0,
    ),
    (0.5, 0.54, 0.003, 0.20, 1.7, 2.1, 1.0, 0.58, 0.16, 0.20, 4.0),
    (
        2.347, 0.70, 0.001, 0.04, 0.32, 0.7, 1.0, 0.28, 0.01, 0.60, -2.0,
    ),
    (
        1.219, 0.68, 0.001, 0.035, 0.24, 0.55, 1.0, 0.20, 0.0, 0.55, 3.0,
    ),
    (
        3.913, 0.58, 0.001, 0.025, 0.18, 0.45, 1.0, 0.12, 0.0, 0.50, -4.0,
    ),
];
const MALLET_OPERATORS: [OperatorTuple; 6] = [
    (1.0, 0.56, 0.004, 0.16, 1.1, 1.5, 1.0, 0.72, 0.28, 0.45, 0.0),
    (
        1.0, 0.52, 0.001, 0.055, 0.34, 0.65, 1.0, 0.32, 0.01, 0.75, 0.0,
    ),
    (
        1.0, 0.33, 0.003, 0.20, 1.25, 1.6, 1.0, 0.68, 0.24, 0.35, -2.0,
    ),
    (
        14.0, 0.22, 0.001, 0.025, 0.16, 0.35, 1.0, 0.16, 0.0, 0.85, 0.0,
    ),
    (
        2.0, 0.22, 0.002, 0.12, 0.80, 1.1, 1.0, 0.48, 0.12, 0.30, 2.0,
    ),
    (3.0, 0.34, 0.001, 0.04, 0.24, 0.5, 1.0, 0.22, 0.0, 0.70, 0.0),
];
const GLASS_OPERATORS: [OperatorTuple; 6] = [
    (
        1.0, 0.66, 0.003, 0.11, 0.72, 1.0, 1.0, 0.62, 0.18, 0.55, 0.0,
    ),
    (
        2.71, 0.58, 0.001, 0.035, 0.22, 0.42, 1.0, 0.18, 0.0, 0.80, -2.0,
    ),
    (
        0.5, 0.38, 0.002, 0.16, 0.90, 1.3, 1.0, 0.52, 0.14, 0.45, 3.0,
    ),
    (
        7.07, 0.40, 0.001, 0.022, 0.14, 0.30, 1.0, 0.12, 0.0, 0.80, 0.0,
    ),
    (
        1.5, 0.32, 0.002, 0.09, 0.58, 0.85, 1.0, 0.38, 0.08, 0.50, -3.0,
    ),
    (
        4.13, 0.46, 0.001, 0.03, 0.20, 0.40, 1.0, 0.15, 0.0, 0.75, 2.0,
    ),
];
const BRASS_OPERATORS: [OperatorTuple; 6] = [
    (
        1.0, 0.30, 0.012, 0.13, 0.50, 0.24, 1.0, 0.84, 0.70, 0.20, 2.0,
    ),
    (
        1.0, 0.16, 0.010, 0.10, 0.34, 0.20, 1.0, 0.72, 0.52, 0.35, 0.0,
    ),
    (
        1.0, 0.34, 0.008, 0.08, 0.28, 0.18, 1.0, 0.64, 0.44, 0.30, 0.0,
    ),
    (
        2.0, 0.15, 0.006, 0.09, 0.30, 0.20, 1.0, 0.65, 0.42, 0.30, 0.0,
    ),
    (
        1.0, 0.38, 0.005, 0.07, 0.24, 0.16, 1.0, 0.58, 0.36, 0.28, -2.0,
    ),
    (
        3.0, 0.30, 0.004, 0.06, 0.20, 0.14, 1.0, 0.52, 0.30, 0.24, 2.0,
    ),
];
const MECHANICAL_OPERATORS: [OperatorTuple; 6] = [
    (
        1.0, 0.72, 0.002, 0.055, 0.22, 0.16, 1.0, 0.54, 0.08, 0.25, 0.0,
    ),
    (
        1.25, 0.66, 0.001, 0.035, 0.14, 0.11, 1.0, 0.32, 0.01, 0.55, 2.0,
    ),
    (
        2.01, 0.54, 0.001, 0.025, 0.10, 0.09, 1.0, 0.22, 0.0, 0.50, -2.0,
    ),
    (
        1.5, 0.62, 0.001, 0.030, 0.12, 0.10, 1.0, 0.28, 0.01, 0.50, 1.0,
    ),
    (
        2.75, 0.54, 0.001, 0.022, 0.09, 0.08, 1.0, 0.20, 0.0, 0.45, -1.0,
    ),
    (
        5.03, 0.44, 0.001, 0.018, 0.07, 0.07, 1.0, 0.14, 0.0, 0.40, 3.0,
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListeningRole {
    Reference,
    Original,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListeningPatch {
    BellMetal,
    FracturedMetal,
    ElectricPianoMallet,
    GlassWood,
    BrassBass,
    MechanicalStab,
}

impl ListeningPatch {
    pub const ALL: [Self; 6] = [
        Self::BellMetal,
        Self::FracturedMetal,
        Self::ElectricPianoMallet,
        Self::GlassWood,
        Self::BrassBass,
        Self::MechanicalStab,
    ];

    pub const fn pair(self) -> u8 {
        match self {
            Self::BellMetal | Self::FracturedMetal => 0,
            Self::ElectricPianoMallet | Self::GlassWood => 1,
            Self::BrassBass | Self::MechanicalStab => 2,
        }
    }

    pub const fn filename(self) -> &'static str {
        match self {
            Self::BellMetal => "01_bell-metal_reference.wav",
            Self::FracturedMetal => "02_fractured-metal_original.wav",
            Self::ElectricPianoMallet => "03_electric-piano-mallet_reference.wav",
            Self::GlassWood => "04_glass-wood_original.wav",
            Self::BrassBass => "05_brass-bass_reference.wav",
            Self::MechanicalStab => "06_mechanical-stab_original.wav",
        }
    }

    pub const fn role(self) -> ListeningRole {
        match self {
            Self::BellMetal | Self::ElectricPianoMallet | Self::BrassBass => {
                ListeningRole::Reference
            }
            Self::FracturedMetal | Self::GlassWood | Self::MechanicalStab => {
                ListeningRole::Original
            }
        }
    }

    pub const fn category(self) -> &'static str {
        match self {
            Self::BellMetal | Self::FracturedMetal => "bell/metal",
            Self::ElectricPianoMallet | Self::GlassWood => "electric piano/mallet",
            Self::BrassBass | Self::MechanicalStab => "brass/bass",
        }
    }

    pub const fn pair_name(self) -> &'static str {
        match self {
            Self::BellMetal | Self::FracturedMetal => "inharmonic stack",
            Self::ElectricPianoMallet | Self::GlassWood => "parallel mallet",
            Self::BrassBass | Self::MechanicalStab => "feedback branch",
        }
    }

    pub const fn pair_gain(self) -> f32 {
        match self {
            Self::BellMetal | Self::FracturedMetal => 0.25,
            Self::ElectricPianoMallet | Self::GlassWood => 0.23,
            Self::BrassBass | Self::MechanicalStab => 0.20,
        }
    }

    pub const fn score(self) -> Score {
        match self {
            Self::BellMetal | Self::FracturedMetal => Score::authored(230_400, &BELL_EVENTS),
            Self::ElectricPianoMallet | Self::GlassWood => Score::authored(192_000, &MALLET_EVENTS),
            Self::BrassBass | Self::MechanicalStab => Score::authored(182_400, &BRASS_EVENTS),
        }
    }

    pub fn patch(self) -> SixOpPatch {
        let (algorithm, operators, feedback, pitch_seconds, pitch_semitones, lfo_rate, lfo_depth) =
            match self {
                Self::BellMetal => (
                    0,
                    BELL_OPERATORS,
                    0.06,
                    [0.008, 0.20, 0.65, 0.90],
                    [0.30, -0.15, 0.0, 0.0],
                    4.8,
                    1.5,
                ),
                Self::FracturedMetal => (
                    0,
                    FRACTURED_OPERATORS,
                    0.16,
                    [0.006, 0.14, 0.45, 0.65],
                    [0.55, -0.30, 0.12, 0.0],
                    6.1,
                    3.0,
                ),
                Self::ElectricPianoMallet => (
                    4,
                    MALLET_OPERATORS,
                    0.03,
                    [0.005, 0.12, 0.45, 0.50],
                    [0.18, -0.08, 0.0, 0.0],
                    4.4,
                    1.2,
                ),
                Self::GlassWood => (
                    4,
                    GLASS_OPERATORS,
                    0.08,
                    [0.004, 0.09, 0.28, 0.35],
                    [0.34, -0.16, 0.06, 0.0],
                    5.7,
                    2.4,
                ),
                Self::BrassBass => (
                    9,
                    BRASS_OPERATORS,
                    0.22,
                    [0.012, 0.08, 0.30, 0.15],
                    [-0.40, 0.12, 0.0, 0.0],
                    5.2,
                    2.0,
                ),
                Self::MechanicalStab => (
                    9,
                    MECHANICAL_OPERATORS,
                    0.34,
                    [0.003, 0.04, 0.16, 0.10],
                    [-0.75, 0.30, -0.10, 0.0],
                    6.7,
                    4.5,
                ),
            };
        let scaling = match self.pair() {
            0 | 1 => ScalingKind::Bright,
            2 => ScalingKind::LowBoost,
            _ => unreachable!("listening pair is fixed"),
        };
        let carrier_mask = CLASSIC_ALGORITHMS
            .get(algorithm)
            .expect("listening algorithm is present in the validated catalog")
            .carriers;

        SixOpPatch {
            algorithm,
            operators: std::array::from_fn(|index| {
                let carrier = carrier_mask & (1 << index) != 0;
                checked_operator(operators[index], scaling, carrier)
            }),
            pitch_envelope: PitchEnvelopeSpec::new(pitch_seconds, pitch_semitones)
                .expect("authored listening pitch envelope is valid"),
            lfo: LfoSpec::new(lfo_rate, lfo_depth).expect("authored listening LFO is valid"),
            feedback,
            output_gain: self.pair_gain(),
        }
    }
}

#[derive(Clone, Copy)]
enum ScalingKind {
    Bright,
    LowBoost,
}

fn checked_operator(tuple: OperatorTuple, scaling: ScalingKind, carrier: bool) -> OperatorSpec {
    let (ratio, level, attack, decay1, decay2, release, l1, l2, sustain, velocity, detune) = tuple;
    let scaling = match (scaling, carrier) {
        (ScalingKind::Bright, true) => KeyboardScaling::new(72, 0.0, 0.0),
        (ScalingKind::Bright, false) => KeyboardScaling::new(72, 0.0, -1.5),
        (ScalingKind::LowBoost, true) => KeyboardScaling::new(48, 0.0, 0.0),
        (ScalingKind::LowBoost, false) => KeyboardScaling::new(48, 1.0, 0.0),
    }
    .expect("authored listening scaling is valid");
    OperatorSpec::new(
        FrequencyMode::ratio(ratio).expect("authored listening ratio is valid"),
        level,
        EnvelopeSpec::new([attack, decay1, decay2, release], [l1, l2, sustain, 0.0])
            .expect("authored listening envelope is valid"),
        velocity,
        scaling,
        detune,
    )
    .expect("authored listening operator is valid")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScoreAction {
    On { note: u8, velocity: u8 },
    Off { note: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreEvent {
    pub sample: u32,
    pub action: ScoreAction,
}

impl ScoreEvent {
    pub const fn on(sample: u32, note: u8, velocity: u8) -> Self {
        Self {
            sample,
            action: ScoreAction::On { note, velocity },
        }
    }

    pub const fn off(sample: u32, note: u8) -> Self {
        Self {
            sample,
            action: ScoreAction::Off { note },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Score {
    duration_samples_48k: u32,
    events: &'static [ScoreEvent],
}

impl Score {
    const fn authored(duration_samples_48k: u32, events: &'static [ScoreEvent]) -> Self {
        Self {
            duration_samples_48k,
            events,
        }
    }

    pub fn new(
        duration_samples_48k: u32,
        events: &'static [ScoreEvent],
    ) -> Result<Self, ScoreError> {
        let score = Self::authored(duration_samples_48k, events);
        score.validate()?;
        Ok(score)
    }

    pub const fn duration_samples_48k(self) -> u32 {
        self.duration_samples_48k
    }

    pub const fn events(self) -> &'static [ScoreEvent] {
        self.events
    }

    pub fn validate(self) -> Result<(), ScoreError> {
        if self.duration_samples_48k == 0 {
            return Err(ScoreError::InvalidDuration);
        }
        let mut active = [0_u8; 128];
        let mut active_count = 0_u8;
        let mut previous_sample = 0;
        for (index, event) in self.events.iter().copied().enumerate() {
            if index != 0 && event.sample < previous_sample {
                return Err(ScoreError::Unsorted);
            }
            if event.sample >= self.duration_samples_48k {
                return Err(ScoreError::OutOfBounds);
            }
            previous_sample = event.sample;
            match event.action {
                ScoreAction::On { note, velocity } => {
                    if note > 127 {
                        return Err(ScoreError::InvalidNote);
                    }
                    if velocity == 0 || velocity > 127 {
                        return Err(ScoreError::InvalidVelocity);
                    }
                    active[usize::from(note)] = active[usize::from(note)].saturating_add(1);
                    active_count = active_count.saturating_add(1);
                    if active_count > 4 {
                        return Err(ScoreError::TooManySimultaneousNotes);
                    }
                }
                ScoreAction::Off { note } => {
                    if note > 127 {
                        return Err(ScoreError::InvalidNote);
                    }
                    let count = &mut active[usize::from(note)];
                    if *count == 0 {
                        return Err(ScoreError::UnmatchedOff);
                    }
                    *count -= 1;
                    active_count -= 1;
                }
            }
        }
        if active_count != 0 {
            return Err(ScoreError::MissingOff);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ScoreError {
    #[error("score duration must be positive")]
    InvalidDuration,
    #[error("score events must be sorted")]
    Unsorted,
    #[error("score MIDI note must be in range")]
    InvalidNote,
    #[error("score MIDI velocity must be 1 through 127")]
    InvalidVelocity,
    #[error("score note-off has no matching note-on")]
    UnmatchedOff,
    #[error("score note-on has no matching note-off")]
    MissingOff,
    #[error("score uses more than four simultaneous notes")]
    TooManySimultaneousNotes,
    #[error("score event is outside its duration")]
    OutOfBounds,
}

const MIN_SAMPLE_RATE: u32 = 8_000;
const MAX_SAMPLE_RATE: u32 = 768_000;
const MAX_SCORE_EVENTS: usize = 16;
const MAX_PREPARED_VOICES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub float: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderDiagnostics {
    pub non_finite_voices: u64,
    pub clamp_contacts: u64,
    pub note_off_events: u32,
    pub voice_steals: u32,
    pub maximum_active_voices: u8,
    pub last_stolen_note: Option<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Render {
    pub samples: Vec<f32>,
    pub format: RenderFormat,
    pub diagnostics: RenderDiagnostics,
    pub sample_hash: u64,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum RenderError {
    #[error("sample rate must be between 8000 and 768000 Hz")]
    InvalidSampleRate,
    #[error("render frame count is not representable")]
    FrameCountOutOfRange,
    #[error("score timeline collapses or falls outside the scaled render")]
    InvalidScaledTimeline,
    #[error("score has more events than the fixed renderer can prepare")]
    TooManyEvents,
    #[error("score has more note-ons than the fixed renderer can prepare")]
    TooManyNoteOns,
    #[error("prepared render output has the wrong exact length")]
    InvalidOutputLength,
    #[error("prepared note voice is missing")]
    MissingPreparedVoice,
    #[error(transparent)]
    Score(#[from] ScoreError),
    #[error(transparent)]
    Voice(#[from] VoiceError),
}

#[derive(Clone, Copy, Debug)]
struct PreparedEvent {
    sample: usize,
    action: ScoreAction,
    note_token: u8,
    voice_resource: Option<usize>,
}

#[derive(Debug)]
struct ResearchVoice {
    note: u8,
    note_token: u8,
    sequence: u64,
    released: bool,
    non_finite_reported: bool,
    clamp_contacts_seen: u64,
    #[cfg(test)]
    non_finite_fault_for_test: bool,
    voice: SixOpVoice,
}

impl ResearchVoice {
    fn new(note: u8, note_token: u8, sequence: u64, mut voice: SixOpVoice) -> Self {
        voice.reset();
        voice.note_on();
        Self {
            note,
            note_token,
            sequence,
            released: false,
            non_finite_reported: false,
            clamp_contacts_seen: 0,
            #[cfg(test)]
            non_finite_fault_for_test: false,
            voice,
        }
    }

    fn reset(&mut self) {
        self.voice.reset();
        self.released = false;
        self.non_finite_reported = false;
        self.clamp_contacts_seen = 0;
        #[cfg(test)]
        {
            self.non_finite_fault_for_test = false;
        }
    }
}

#[derive(Debug)]
struct ResearchEnsemble {
    active: [Option<ResearchVoice>; 4],
    next_sequence: u64,
    diagnostics: RenderDiagnostics,
}

impl ResearchEnsemble {
    fn new() -> Self {
        Self {
            active: std::array::from_fn(|_| None),
            next_sequence: 0,
            diagnostics: RenderDiagnostics::default(),
        }
    }

    fn note_on(&mut self, note: u8, note_token: u8, voice: SixOpVoice) {
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        let research_voice = ResearchVoice::new(note, note_token, sequence, voice);
        let slot = self
            .active
            .iter()
            .position(Option::is_none)
            .unwrap_or_else(|| {
                self.active
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, voice)| {
                        voice.as_ref().map_or(u64::MAX, |voice| voice.sequence)
                    })
                    .map_or(0, |(index, _)| index)
            });
        if let Some(mut stolen) = self.active[slot].take() {
            self.diagnostics.voice_steals = self.diagnostics.voice_steals.saturating_add(1);
            self.diagnostics.last_stolen_note = Some(stolen.note);
            retain_voice_diagnostics(&mut self.diagnostics, &mut stolen);
            stolen.reset();
        }
        self.active[slot] = Some(research_voice);
        let count = self.active.iter().flatten().count() as u8;
        self.diagnostics.maximum_active_voices = self.diagnostics.maximum_active_voices.max(count);
    }

    fn note_off(&mut self, note_token: u8) {
        let target = self
            .active
            .iter()
            .enumerate()
            .filter_map(|(index, voice)| {
                let voice = voice.as_ref()?;
                (voice.note_token == note_token && !voice.released)
                    .then_some((index, voice.sequence))
            })
            .min_by_key(|(_, sequence)| *sequence)
            .map(|(index, _)| index);
        if let Some(index) = target {
            if let Some(voice) = self.active[index].as_mut() {
                voice.voice.note_off();
                voice.released = true;
                self.diagnostics.note_off_events =
                    self.diagnostics.note_off_events.saturating_add(1);
            }
        }
    }

    #[inline]
    fn sample(&mut self) -> f32 {
        let mut sum = 0.0;
        for index in 0..self.active.len() {
            let mut became_idle = false;
            if let Some(active) = &mut self.active[index] {
                sum += active.voice.sample();
                retain_voice_diagnostics(&mut self.diagnostics, active);
                became_idle = active.voice.is_idle();
            }
            if became_idle {
                if let Some(mut finished) = self.active[index].take() {
                    finished.reset();
                }
            }
        }
        sum
    }

    const fn diagnostics(&self) -> RenderDiagnostics {
        self.diagnostics
    }
}

fn retain_voice_diagnostics(diagnostics: &mut RenderDiagnostics, active: &mut ResearchVoice) {
    let non_finite_seen = active.voice.non_finite_seen();
    #[cfg(test)]
    let non_finite_seen = non_finite_seen || active.non_finite_fault_for_test;
    if non_finite_seen && !active.non_finite_reported {
        diagnostics.non_finite_voices = diagnostics.non_finite_voices.saturating_add(1);
        active.non_finite_reported = true;
    }
    let contacts = active.voice.clamp_contacts();
    diagnostics.clamp_contacts = diagnostics
        .clamp_contacts
        .saturating_add(contacts.saturating_sub(active.clamp_contacts_seen));
    active.clamp_contacts_seen = contacts;
}

#[derive(Debug)]
struct PreparedRender {
    _master_sine_table: SharedSineTable,
    events: [Option<PreparedEvent>; MAX_SCORE_EVENTS],
    event_count: usize,
    voices: [Option<SixOpVoice>; MAX_PREPARED_VOICES],
    frame_count: usize,
    ensemble: ResearchEnsemble,
}

impl PreparedRender {
    fn new(patch_id: ListeningPatch, sample_rate: u32, score: Score) -> Result<Self, RenderError> {
        if !(MIN_SAMPLE_RATE..=MAX_SAMPLE_RATE).contains(&sample_rate) {
            return Err(RenderError::InvalidSampleRate);
        }
        score.validate()?;
        if score.events().len() > MAX_SCORE_EVENTS {
            return Err(RenderError::TooManyEvents);
        }
        let note_on_count = score
            .events()
            .iter()
            .filter(|event| matches!(event.action, ScoreAction::On { .. }))
            .count();
        if note_on_count > MAX_PREPARED_VOICES {
            return Err(RenderError::TooManyNoteOns);
        }

        let frame_count = scale_reference_sample(score.duration_samples_48k(), sample_rate)?;
        if frame_count == 0 {
            return Err(RenderError::InvalidScaledTimeline);
        }

        let master_sine_table: SharedSineTable = SineTable::new().into();
        let mut events = std::array::from_fn(|_| None);
        let mut voices = std::array::from_fn(|_| None);
        let mut voice_index = 0;
        let mut pending_tokens = [[0_u8; MAX_PREPARED_VOICES]; 128];
        let mut pending_counts = [0_u8; 128];
        let mut previous_reference_sample = 0;
        let mut previous_scaled_sample = 0;
        let patch = patch_id.patch();
        for (index, event) in score.events().iter().copied().enumerate() {
            let scaled_sample = scale_reference_sample(event.sample, sample_rate)?;
            if scaled_sample >= frame_count
                || (index != 0
                    && event.sample > previous_reference_sample
                    && scaled_sample <= previous_scaled_sample)
            {
                return Err(RenderError::InvalidScaledTimeline);
            }
            previous_reference_sample = event.sample;
            previous_scaled_sample = scaled_sample;

            let (note_token, resource) = match event.action {
                ScoreAction::On { note, velocity } => {
                    let note_token =
                        u8::try_from(voice_index).map_err(|_| RenderError::TooManyNoteOns)?;
                    voices[voice_index] = Some(SixOpVoice::new_with_sine_table(
                        patch,
                        sample_rate as f32,
                        note,
                        f32::from(velocity) / 127.0,
                        master_sine_table.clone(),
                    )?);
                    let resource = voice_index;
                    voice_index += 1;
                    let count = usize::from(pending_counts[usize::from(note)]);
                    pending_tokens[usize::from(note)][count] = note_token;
                    pending_counts[usize::from(note)] += 1;
                    (note_token, Some(resource))
                }
                ScoreAction::Off { note } => {
                    let note_index = usize::from(note);
                    let count = usize::from(pending_counts[note_index]);
                    if count == 0 {
                        return Err(RenderError::MissingPreparedVoice);
                    }
                    let note_token = pending_tokens[note_index][0];
                    pending_tokens[note_index].copy_within(1..count, 0);
                    pending_counts[note_index] -= 1;
                    (note_token, None)
                }
            };
            events[index] = Some(PreparedEvent {
                sample: scaled_sample,
                action: event.action,
                note_token,
                voice_resource: resource,
            });
        }
        Ok(Self {
            _master_sine_table: master_sine_table,
            events,
            event_count: score.events().len(),
            voices,
            frame_count,
            ensemble: ResearchEnsemble::new(),
        })
    }

    const fn frame_count(&self) -> usize {
        self.frame_count
    }

    const fn diagnostics(&self) -> RenderDiagnostics {
        self.ensemble.diagnostics()
    }

    fn run(&mut self, samples: &mut [f32]) -> Result<(), RenderError> {
        if samples.len()
            != self
                .frame_count
                .checked_mul(2)
                .ok_or(RenderError::FrameCountOutOfRange)?
        {
            return Err(RenderError::InvalidOutputLength);
        }
        let mut event_index = 0;
        for frame in 0..self.frame_count {
            while event_index < self.event_count {
                let event = self.events[event_index]
                    .as_ref()
                    .ok_or(RenderError::MissingPreparedVoice)?;
                if event.sample != frame {
                    break;
                }
                match event.action {
                    ScoreAction::On { note, .. } => {
                        let resource = event
                            .voice_resource
                            .ok_or(RenderError::MissingPreparedVoice)?;
                        let voice = self.voices[resource]
                            .take()
                            .ok_or(RenderError::MissingPreparedVoice)?;
                        self.ensemble.note_on(note, event.note_token, voice);
                    }
                    ScoreAction::Off { .. } => self.ensemble.note_off(event.note_token),
                }
                event_index += 1;
            }
            let mono = self.ensemble.sample();
            samples[frame * 2] = mono;
            samples[frame * 2 + 1] = mono;
        }
        Ok(())
    }
}

fn scale_reference_sample(sample: u32, sample_rate: u32) -> Result<usize, RenderError> {
    let numerator = u64::from(sample)
        .checked_mul(u64::from(sample_rate))
        .and_then(|value| value.checked_add(u64::from(REFERENCE_SAMPLE_RATE / 2)))
        .ok_or(RenderError::FrameCountOutOfRange)?;
    usize::try_from(numerator / u64::from(REFERENCE_SAMPLE_RATE))
        .map_err(|_| RenderError::FrameCountOutOfRange)
}

pub fn render(patch: ListeningPatch, sample_rate: u32) -> Result<Render, RenderError> {
    render_score(patch, sample_rate, patch.score())
}

fn render_score(
    patch: ListeningPatch,
    sample_rate: u32,
    score: Score,
) -> Result<Render, RenderError> {
    let mut prepared = PreparedRender::new(patch, sample_rate, score)?;
    let sample_count = prepared
        .frame_count()
        .checked_mul(2)
        .ok_or(RenderError::FrameCountOutOfRange)?;
    let mut samples = vec![0.0; sample_count];
    prepared.run(&mut samples)?;
    let diagnostics = prepared.diagnostics();
    let sample_hash = fnv1a_sample_hash(&samples);
    Ok(Render {
        samples,
        format: RenderFormat {
            sample_rate,
            channels: 2,
            bits_per_sample: 32,
            float: true,
        },
        diagnostics,
        sample_hash,
    })
}

fn fnv1a_sample_hash(samples: &[f32]) -> u64 {
    // The sine table is generated with target/runtime f32 math. This hash is
    // same-build/target replay evidence, not a universal cross-target golden.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for sample in samples {
        hash ^= u64::from(sample.to_bits());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::*;
    use crate::dsp::six_op_pm::algorithm::CLASSIC_ALGORITHMS;
    use crate::dsp::six_op_pm::{FrequencyMode, KeyboardScaling};

    const BELL_EVENTS_EXPECTED: [ScoreEvent; 6] = [
        ScoreEvent::on(0, 60, 110),
        ScoreEvent::off(42_240, 60),
        ScoreEvent::on(57_600, 72, 94),
        ScoreEvent::off(98_400, 72),
        ScoreEvent::on(117_600, 48, 122),
        ScoreEvent::off(160_800, 48),
    ];
    const MALLET_EVENTS_EXPECTED: [ScoreEvent; 8] = [
        ScoreEvent::on(0, 60, 100),
        ScoreEvent::off(36_000, 60),
        ScoreEvent::on(55_200, 60, 114),
        ScoreEvent::on(55_200, 64, 98),
        ScoreEvent::on(55_200, 67, 105),
        ScoreEvent::off(110_400, 60),
        ScoreEvent::off(110_400, 64),
        ScoreEvent::off(110_400, 67),
    ];
    const BRASS_EVENTS_EXPECTED: [ScoreEvent; 8] = [
        ScoreEvent::on(0, 36, 116),
        ScoreEvent::off(26_400, 36),
        ScoreEvent::on(34_560, 43, 96),
        ScoreEvent::off(60_000, 43),
        ScoreEvent::on(68_160, 48, 108),
        ScoreEvent::off(91_200, 48),
        ScoreEvent::on(96_000, 36, 124),
        ScoreEvent::off(124_800, 36),
    ];
    const SYNTHETIC_STEAL_EVENTS: [ScoreEvent; 10] = [
        ScoreEvent::on(0, 60, 100),
        ScoreEvent::on(0, 61, 100),
        ScoreEvent::on(0, 62, 100),
        ScoreEvent::on(0, 63, 100),
        ScoreEvent::off(10, 60),
        ScoreEvent::on(10, 64, 100),
        ScoreEvent::off(20, 61),
        ScoreEvent::off(20, 62),
        ScoreEvent::off(20, 63),
        ScoreEvent::off(20, 64),
    ];
    const OVERLAPPING_NOTE_STEAL_EVENTS: [ScoreEvent; 10] = [
        ScoreEvent::on(0, 60, 100),
        ScoreEvent::on(0, 60, 110),
        ScoreEvent::on(0, 61, 100),
        ScoreEvent::on(0, 62, 100),
        ScoreEvent::off(10, 61),
        ScoreEvent::on(10, 63, 100),
        ScoreEvent::off(20, 60),
        ScoreEvent::off(30, 60),
        ScoreEvent::off(40, 62),
        ScoreEvent::off(40, 63),
    ];

    fn dispatch_event_for_test(prepared: &mut PreparedRender, index: usize) {
        let event = prepared.events[index].unwrap();
        match event.action {
            ScoreAction::On { note, .. } => {
                let voice = prepared.voices[event.voice_resource.unwrap()]
                    .take()
                    .unwrap();
                prepared.ensemble.note_on(note, event.note_token, voice);
            }
            ScoreAction::Off { .. } => prepared.ensemble.note_off(event.note_token),
        }
    }

    fn prepared_test_voice(patch: ListeningPatch, note: u8) -> SixOpVoice {
        let table: SharedSineTable = SineTable::new().into();
        SixOpVoice::new_with_sine_table(
            patch.patch(),
            REFERENCE_SAMPLE_RATE as f32,
            note,
            1.0,
            table,
        )
        .unwrap()
    }

    fn clamp_reporting_idle_voice(note: u8) -> SixOpVoice {
        let mut patch = ListeningPatch::MechanicalStab.patch();
        patch.output_gain = 4.0;
        let table: SharedSineTable = SineTable::new().into();
        let mut voice =
            SixOpVoice::new_with_sine_table(patch, REFERENCE_SAMPLE_RATE as f32, note, 1.0, table)
                .unwrap();
        voice.note_on();
        for _ in 0..2_000 {
            voice.sample();
        }
        voice.note_off();
        for _ in 0..10_000 {
            voice.sample();
            if voice.is_idle() {
                break;
            }
        }
        assert!(voice.is_idle());
        assert!(voice.clamp_contacts() > 0);
        voice
    }

    #[test]
    fn gate_has_three_topology_pairs_and_six_files() {
        assert_eq!(
            ListeningPatch::ALL,
            [
                ListeningPatch::BellMetal,
                ListeningPatch::FracturedMetal,
                ListeningPatch::ElectricPianoMallet,
                ListeningPatch::GlassWood,
                ListeningPatch::BrassBass,
                ListeningPatch::MechanicalStab,
            ]
        );
        assert_eq!(
            ListeningPatch::ALL.map(ListeningPatch::pair),
            [0, 0, 1, 1, 2, 2]
        );
        assert_eq!(
            ListeningPatch::ALL.map(|id| id.patch().algorithm),
            [0, 0, 4, 4, 9, 9]
        );
        for pair in ListeningPatch::ALL.chunks_exact(2) {
            assert_eq!(pair[0].score(), pair[1].score());
            assert_eq!(pair[0].pair_gain(), pair[1].pair_gain());
        }
        let fingerprints = [0, 4, 9].map(|index| CLASSIC_ALGORITHMS[index].fingerprint());
        assert_ne!(fingerprints[0], fingerprints[1]);
        assert_ne!(fingerprints[0], fingerprints[2]);
        assert_ne!(fingerprints[1], fingerprints[2]);
    }

    #[test]
    fn inventory_metadata_is_exact_and_stable() {
        assert_eq!(
            ListeningPatch::ALL.map(ListeningPatch::filename),
            [
                "01_bell-metal_reference.wav",
                "02_fractured-metal_original.wav",
                "03_electric-piano-mallet_reference.wav",
                "04_glass-wood_original.wav",
                "05_brass-bass_reference.wav",
                "06_mechanical-stab_original.wav",
            ]
        );
        assert_eq!(
            ListeningPatch::ALL.map(ListeningPatch::role),
            [
                ListeningRole::Reference,
                ListeningRole::Original,
                ListeningRole::Reference,
                ListeningRole::Original,
                ListeningRole::Reference,
                ListeningRole::Original,
            ]
        );
        assert_eq!(
            ListeningPatch::ALL.map(ListeningPatch::category),
            [
                "bell/metal",
                "bell/metal",
                "electric piano/mallet",
                "electric piano/mallet",
                "brass/bass",
                "brass/bass",
            ]
        );
        assert_eq!(
            ListeningPatch::ALL.map(ListeningPatch::pair_name),
            [
                "inharmonic stack",
                "inharmonic stack",
                "parallel mallet",
                "parallel mallet",
                "feedback branch",
                "feedback branch",
            ]
        );
        assert_eq!(
            ListeningPatch::ALL.map(ListeningPatch::pair_gain),
            [0.25, 0.25, 0.23, 0.23, 0.20, 0.20]
        );
    }

    #[test]
    fn scores_have_exact_authored_events_durations_and_shared_pair_identity() {
        let bell = ListeningPatch::BellMetal.score();
        let mallet = ListeningPatch::ElectricPianoMallet.score();
        let brass = ListeningPatch::BrassBass.score();

        assert_eq!(bell.duration_samples_48k(), 230_400);
        assert_eq!(bell.events(), &BELL_EVENTS_EXPECTED);
        assert_eq!(mallet.duration_samples_48k(), 192_000);
        assert_eq!(mallet.events(), &MALLET_EVENTS_EXPECTED);
        assert_eq!(brass.duration_samples_48k(), 182_400);
        assert_eq!(brass.events(), &BRASS_EVENTS_EXPECTED);
        for pair in ListeningPatch::ALL.chunks_exact(2) {
            assert_eq!(pair[0].score(), pair[1].score());
        }
    }

    #[test]
    fn score_validation_rejects_every_invalid_timeline_shape() {
        const UNSORTED: [ScoreEvent; 2] = [ScoreEvent::on(1, 60, 100), ScoreEvent::off(0, 60)];
        const INVALID_NOTE: [ScoreEvent; 2] =
            [ScoreEvent::on(0, 128, 100), ScoreEvent::off(1, 128)];
        const INVALID_VELOCITY: [ScoreEvent; 2] =
            [ScoreEvent::on(0, 60, 0), ScoreEvent::off(1, 60)];
        const UNMATCHED_OFF: [ScoreEvent; 1] = [ScoreEvent::off(0, 60)];
        const MISSING_OFF: [ScoreEvent; 1] = [ScoreEvent::on(0, 60, 100)];
        const TOO_MANY_VOICES: [ScoreEvent; 10] = [
            ScoreEvent::on(0, 60, 100),
            ScoreEvent::on(0, 61, 100),
            ScoreEvent::on(0, 62, 100),
            ScoreEvent::on(0, 63, 100),
            ScoreEvent::on(0, 64, 100),
            ScoreEvent::off(1, 60),
            ScoreEvent::off(1, 61),
            ScoreEvent::off(1, 62),
            ScoreEvent::off(1, 63),
            ScoreEvent::off(1, 64),
        ];
        const AT_DURATION: [ScoreEvent; 2] = [ScoreEvent::on(0, 60, 100), ScoreEvent::off(1, 60)];

        assert_eq!(Score::new(2, &UNSORTED), Err(ScoreError::Unsorted));
        assert_eq!(Score::new(2, &INVALID_NOTE), Err(ScoreError::InvalidNote));
        assert_eq!(
            Score::new(2, &INVALID_VELOCITY),
            Err(ScoreError::InvalidVelocity)
        );
        assert_eq!(Score::new(2, &UNMATCHED_OFF), Err(ScoreError::UnmatchedOff));
        assert_eq!(Score::new(2, &MISSING_OFF), Err(ScoreError::MissingOff));
        assert_eq!(
            Score::new(2, &TOO_MANY_VOICES),
            Err(ScoreError::TooManySimultaneousNotes)
        );
        assert_eq!(Score::new(1, &AT_DURATION), Err(ScoreError::OutOfBounds));
        assert_eq!(Score::new(0, &[]), Err(ScoreError::InvalidDuration));

        for patch in ListeningPatch::ALL {
            assert_eq!(patch.score().validate(), Ok(()));
        }
    }

    #[test]
    fn patch_operator_tables_and_feedback_are_exact() {
        let expected = [
            (
                ListeningPatch::BellMetal,
                0.06,
                [
                    (1.0, 0.54, 0.003, 0.18, 1.8, 2.4, 1.0, 0.62, 0.18, 0.25, 0.0),
                    (
                        1.414, 0.68, 0.002, 0.09, 0.55, 1.1, 1.0, 0.48, 0.05, 0.55, 0.0,
                    ),
                    (1.0, 0.40, 0.002, 0.24, 2.4, 2.8, 1.0, 0.70, 0.22, 0.18, 0.0),
                    (
                        2.0, 0.66, 0.001, 0.08, 0.70, 1.2, 1.0, 0.42, 0.03, 0.50, 0.0,
                    ),
                    (
                        1.414, 0.62, 0.001, 0.06, 0.42, 0.9, 1.0, 0.35, 0.02, 0.45, 0.0,
                    ),
                    (
                        2.71, 0.52, 0.001, 0.04, 0.28, 0.7, 1.0, 0.24, 0.0, 0.40, 0.0,
                    ),
                ],
            ),
            (
                ListeningPatch::FracturedMetal,
                0.16,
                [
                    (
                        1.0, 0.74, 0.002, 0.12, 1.2, 1.7, 1.0, 0.50, 0.12, 0.30, -3.0,
                    ),
                    (
                        1.731, 0.72, 0.001, 0.05, 0.38, 0.8, 1.0, 0.30, 0.01, 0.65, 2.0,
                    ),
                    (0.5, 0.54, 0.003, 0.20, 1.7, 2.1, 1.0, 0.58, 0.16, 0.20, 4.0),
                    (
                        2.347, 0.70, 0.001, 0.04, 0.32, 0.7, 1.0, 0.28, 0.01, 0.60, -2.0,
                    ),
                    (
                        1.219, 0.68, 0.001, 0.035, 0.24, 0.55, 1.0, 0.20, 0.0, 0.55, 3.0,
                    ),
                    (
                        3.913, 0.58, 0.001, 0.025, 0.18, 0.45, 1.0, 0.12, 0.0, 0.50, -4.0,
                    ),
                ],
            ),
            (
                ListeningPatch::ElectricPianoMallet,
                0.03,
                [
                    (1.0, 0.56, 0.004, 0.16, 1.1, 1.5, 1.0, 0.72, 0.28, 0.45, 0.0),
                    (
                        1.0, 0.52, 0.001, 0.055, 0.34, 0.65, 1.0, 0.32, 0.01, 0.75, 0.0,
                    ),
                    (
                        1.0, 0.33, 0.003, 0.20, 1.25, 1.6, 1.0, 0.68, 0.24, 0.35, -2.0,
                    ),
                    (
                        14.0, 0.22, 0.001, 0.025, 0.16, 0.35, 1.0, 0.16, 0.0, 0.85, 0.0,
                    ),
                    (
                        2.0, 0.22, 0.002, 0.12, 0.80, 1.1, 1.0, 0.48, 0.12, 0.30, 2.0,
                    ),
                    (3.0, 0.34, 0.001, 0.04, 0.24, 0.5, 1.0, 0.22, 0.0, 0.70, 0.0),
                ],
            ),
            (
                ListeningPatch::GlassWood,
                0.08,
                [
                    (
                        1.0, 0.66, 0.003, 0.11, 0.72, 1.0, 1.0, 0.62, 0.18, 0.55, 0.0,
                    ),
                    (
                        2.71, 0.58, 0.001, 0.035, 0.22, 0.42, 1.0, 0.18, 0.0, 0.80, -2.0,
                    ),
                    (
                        0.5, 0.38, 0.002, 0.16, 0.90, 1.3, 1.0, 0.52, 0.14, 0.45, 3.0,
                    ),
                    (
                        7.07, 0.40, 0.001, 0.022, 0.14, 0.30, 1.0, 0.12, 0.0, 0.80, 0.0,
                    ),
                    (
                        1.5, 0.32, 0.002, 0.09, 0.58, 0.85, 1.0, 0.38, 0.08, 0.50, -3.0,
                    ),
                    (
                        4.13, 0.46, 0.001, 0.03, 0.20, 0.40, 1.0, 0.15, 0.0, 0.75, 2.0,
                    ),
                ],
            ),
            (
                ListeningPatch::BrassBass,
                0.22,
                [
                    (
                        1.0, 0.30, 0.012, 0.13, 0.50, 0.24, 1.0, 0.84, 0.70, 0.20, 2.0,
                    ),
                    (
                        1.0, 0.16, 0.010, 0.10, 0.34, 0.20, 1.0, 0.72, 0.52, 0.35, 0.0,
                    ),
                    (
                        1.0, 0.34, 0.008, 0.08, 0.28, 0.18, 1.0, 0.64, 0.44, 0.30, 0.0,
                    ),
                    (
                        2.0, 0.15, 0.006, 0.09, 0.30, 0.20, 1.0, 0.65, 0.42, 0.30, 0.0,
                    ),
                    (
                        1.0, 0.38, 0.005, 0.07, 0.24, 0.16, 1.0, 0.58, 0.36, 0.28, -2.0,
                    ),
                    (
                        3.0, 0.30, 0.004, 0.06, 0.20, 0.14, 1.0, 0.52, 0.30, 0.24, 2.0,
                    ),
                ],
            ),
            (
                ListeningPatch::MechanicalStab,
                0.34,
                [
                    (
                        1.0, 0.72, 0.002, 0.055, 0.22, 0.16, 1.0, 0.54, 0.08, 0.25, 0.0,
                    ),
                    (
                        1.25, 0.66, 0.001, 0.035, 0.14, 0.11, 1.0, 0.32, 0.01, 0.55, 2.0,
                    ),
                    (
                        2.01, 0.54, 0.001, 0.025, 0.10, 0.09, 1.0, 0.22, 0.0, 0.50, -2.0,
                    ),
                    (
                        1.5, 0.62, 0.001, 0.030, 0.12, 0.10, 1.0, 0.28, 0.01, 0.50, 1.0,
                    ),
                    (
                        2.75, 0.54, 0.001, 0.022, 0.09, 0.08, 1.0, 0.20, 0.0, 0.45, -1.0,
                    ),
                    (
                        5.03, 0.44, 0.001, 0.018, 0.07, 0.07, 1.0, 0.14, 0.0, 0.40, 3.0,
                    ),
                ],
            ),
        ];

        for (id, feedback, operators) in expected {
            let patch = id.patch();
            assert_eq!(patch.feedback, feedback, "{id:?}");
            assert_eq!(patch.output_gain, id.pair_gain(), "{id:?}");
            for (actual, expected) in patch.operators.into_iter().zip(operators) {
                let FrequencyMode::Ratio(ratio) = actual.frequency else {
                    panic!("{id:?} used fixed-frequency mode");
                };
                let (ratio_expected, level, a, d1, d2, r, l1, l2, sustain, velocity, detune) =
                    expected;
                assert_eq!(ratio, ratio_expected, "{id:?}");
                assert_eq!(actual.output_level, level, "{id:?}");
                assert_eq!(actual.envelope.seconds, [a, d1, d2, r], "{id:?}");
                assert_eq!(actual.envelope.levels, [l1, l2, sustain, 0.0], "{id:?}");
                assert_eq!(actual.velocity_sensitivity, velocity, "{id:?}");
                assert_eq!(actual.detune_cents, detune, "{id:?}");
            }
        }
    }

    #[test]
    fn keyboard_scaling_follows_actual_carrier_and_modulator_roles() {
        for id in ListeningPatch::ALL {
            let patch = id.patch();
            let graph = CLASSIC_ALGORITHMS[patch.algorithm];
            for (index, operator) in patch.operators.into_iter().enumerate() {
                let carrier = graph.carriers & (1 << index) != 0;
                let expected = match (id.pair(), carrier) {
                    (0 | 1, true) => KeyboardScaling::new(72, 0.0, 0.0).unwrap(),
                    (0 | 1, false) => KeyboardScaling::new(72, 0.0, -1.5).unwrap(),
                    (2, true) => KeyboardScaling::new(48, 0.0, 0.0).unwrap(),
                    (2, false) => KeyboardScaling::new(48, 1.0, 0.0).unwrap(),
                    _ => unreachable!("listening pair inventory is fixed"),
                };
                assert_eq!(operator.scaling, expected, "{id:?} operator {index}");
            }
        }
    }

    #[test]
    fn pitch_envelopes_and_lfos_are_exact_subtle_and_pair_bounded() {
        let expected = [
            ([0.008, 0.20, 0.65, 0.90], [0.30, -0.15, 0.0, 0.0], 4.8, 1.5),
            (
                [0.006, 0.14, 0.45, 0.65],
                [0.55, -0.30, 0.12, 0.0],
                6.1,
                3.0,
            ),
            ([0.005, 0.12, 0.45, 0.50], [0.18, -0.08, 0.0, 0.0], 4.4, 1.2),
            (
                [0.004, 0.09, 0.28, 0.35],
                [0.34, -0.16, 0.06, 0.0],
                5.7,
                2.4,
            ),
            ([0.012, 0.08, 0.30, 0.15], [-0.40, 0.12, 0.0, 0.0], 5.2, 2.0),
            (
                [0.003, 0.04, 0.16, 0.10],
                [-0.75, 0.30, -0.10, 0.0],
                6.7,
                4.5,
            ),
        ];

        for (id, (seconds, semitones, rate_hz, depth_cents)) in
            ListeningPatch::ALL.into_iter().zip(expected)
        {
            let patch = id.patch();
            assert_eq!(patch.pitch_envelope.seconds, seconds, "{id:?}");
            assert_eq!(patch.pitch_envelope.semitones, semitones, "{id:?}");
            assert_eq!(patch.lfo.rate_hz, rate_hz, "{id:?}");
            assert_eq!(patch.lfo.pitch_depth_cents, depth_cents, "{id:?}");
            assert!(rate_hz <= 6.7 && depth_cents <= 5.0);
        }

        for pair in ListeningPatch::ALL.chunks_exact(2) {
            let reference = pair[0]
                .patch()
                .pitch_envelope
                .semitones
                .into_iter()
                .map(f32::abs)
                .fold(0.0, f32::max);
            let original = pair[1]
                .patch()
                .pitch_envelope
                .semitones
                .into_iter()
                .map(f32::abs)
                .fold(0.0, f32::max);
            assert!(reference <= 0.8);
            assert!(original <= 2.0 * reference);
        }

        for reference in [
            ListeningPatch::BellMetal,
            ListeningPatch::ElectricPianoMallet,
            ListeningPatch::BrassBass,
        ] {
            let patch = reference.patch();
            assert!(patch.lfo.pitch_depth_cents <= 2.0);
            assert!(
                patch
                    .operators
                    .windows(2)
                    .any(|pair| pair[0].envelope != pair[1].envelope),
                "{reference:?} must carry category motion in operator envelopes"
            );
        }
    }

    #[test]
    fn renderer_is_exact_dry_dual_mono_finite_and_deterministic() {
        for id in ListeningPatch::ALL {
            let first = render(id, REFERENCE_SAMPLE_RATE).unwrap();
            let second = render(id, REFERENCE_SAMPLE_RATE).unwrap();
            assert_eq!(first, second, "{id:?}");
            assert_eq!(first.sample_hash, second.sample_hash, "{id:?}");
            assert_ne!(first.sample_hash, 0, "{id:?}");
            assert_eq!(first.format.sample_rate, REFERENCE_SAMPLE_RATE);
            assert_eq!(first.format.channels, 2);
            assert_eq!(first.format.bits_per_sample, 32);
            assert!(first.format.float);
            assert_eq!(
                first.samples.len(),
                id.score().duration_samples_48k() as usize * 2,
                "{id:?}"
            );
            assert!(
                first.samples.iter().all(|sample| sample.is_finite()),
                "{id:?}"
            );
            assert!(
                first
                    .samples
                    .chunks_exact(2)
                    .all(|frame| frame[0] == frame[1]),
                "{id:?}"
            );
            assert_eq!(first.diagnostics.non_finite_voices, 0, "{id:?}");
            assert_eq!(first.diagnostics.clamp_contacts, 0, "{id:?}");
        }
    }

    #[test]
    fn score_tails_release_and_four_note_capacity_are_observable() {
        let render = render(ListeningPatch::ElectricPianoMallet, REFERENCE_SAMPLE_RATE).unwrap();
        let first_off = 36_000_usize;
        let after_off = &render.samples[first_off * 2..(first_off + 2_000) * 2];
        assert!(after_off.iter().any(|sample| sample.abs() > 1.0e-6));
        let final_silence = &render.samples[render.samples.len() - 2_000..];
        assert!(final_silence.iter().all(|sample| *sample == 0.0));
        assert_eq!(render.diagnostics.maximum_active_voices, 4);
        assert_eq!(render.diagnostics.note_off_events, 4);

        const FOUR_NOTES: [ScoreEvent; 8] = [
            ScoreEvent::on(0, 60, 100),
            ScoreEvent::on(0, 64, 100),
            ScoreEvent::on(0, 67, 100),
            ScoreEvent::on(0, 72, 100),
            ScoreEvent::off(100, 60),
            ScoreEvent::off(100, 64),
            ScoreEvent::off(100, 67),
            ScoreEvent::off(100, 72),
        ];
        let score = Score::new(2_000, &FOUR_NOTES).unwrap();
        let render = render_score(ListeningPatch::BellMetal, REFERENCE_SAMPLE_RATE, score).unwrap();
        assert_eq!(render.diagnostics.maximum_active_voices, 4);
        assert_eq!(render.diagnostics.voice_steals, 0);
    }

    #[test]
    fn oldest_voice_is_stolen_deterministically_after_an_authored_off() {
        let score = Score::new(2_000, &SYNTHETIC_STEAL_EVENTS).unwrap();

        let first = render_score(ListeningPatch::BellMetal, REFERENCE_SAMPLE_RATE, score).unwrap();
        let second = render_score(ListeningPatch::BellMetal, REFERENCE_SAMPLE_RATE, score).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.diagnostics.maximum_active_voices, 4);
        assert_eq!(first.diagnostics.voice_steals, 1);
        assert_eq!(first.diagnostics.last_stolen_note, Some(60));
    }

    #[test]
    fn stolen_same_note_token_consumes_its_own_off_without_releasing_the_survivor() {
        let score = Score::new(2_000, &OVERLAPPING_NOTE_STEAL_EVENTS).unwrap();
        let mut prepared =
            PreparedRender::new(ListeningPatch::BellMetal, REFERENCE_SAMPLE_RATE, score).unwrap();

        for index in 0..=6 {
            dispatch_event_for_test(&mut prepared, index);
        }
        let surviving_b = prepared
            .ensemble
            .active
            .iter()
            .flatten()
            .find(|voice| voice.sequence == 1)
            .unwrap();
        assert_eq!(surviving_b.note, 60);
        assert!(
            !surviving_b.released,
            "the first Off60 belongs to stolen token A"
        );

        dispatch_event_for_test(&mut prepared, 7);
        let surviving_b = prepared
            .ensemble
            .active
            .iter()
            .flatten()
            .find(|voice| voice.sequence == 1)
            .unwrap();
        assert!(
            surviving_b.released,
            "the second Off60 must release token B"
        );
    }

    #[test]
    fn complete_prepared_event_frame_loop_is_allocation_free() {
        let score = Score::new(2_000, &SYNTHETIC_STEAL_EVENTS).unwrap();
        let mut prepared =
            PreparedRender::new(ListeningPatch::BellMetal, REFERENCE_SAMPLE_RATE, score).unwrap();
        let mut samples = vec![0.0; prepared.frame_count() * 2];

        assert_no_alloc(|| prepared.run(&mut samples).unwrap());
        assert_eq!(prepared.diagnostics().maximum_active_voices, 4);
        assert_eq!(prepared.diagnostics().voice_steals, 1);
        assert_eq!(prepared.diagnostics().last_stolen_note, Some(60));
    }

    #[test]
    fn pending_fault_diagnostics_survive_normal_voice_retirement() {
        let voice = clamp_reporting_idle_voice(36);
        let contacts = voice.clamp_contacts();
        let mut ensemble = ResearchEnsemble::new();
        ensemble.active[0] = Some(ResearchVoice {
            note: 36,
            note_token: 0,
            sequence: 0,
            released: true,
            non_finite_reported: false,
            clamp_contacts_seen: 0,
            non_finite_fault_for_test: true,
            voice,
        });

        ensemble.sample();

        assert!(ensemble.active[0].is_none());
        assert_eq!(ensemble.diagnostics().non_finite_voices, 1);
        assert_eq!(ensemble.diagnostics().clamp_contacts, contacts);
    }

    #[test]
    fn pending_fault_diagnostics_survive_oldest_voice_stealing() {
        let voice = clamp_reporting_idle_voice(36);
        let contacts = voice.clamp_contacts();
        let mut ensemble = ResearchEnsemble::new();
        ensemble.active[0] = Some(ResearchVoice {
            note: 36,
            note_token: 0,
            sequence: 0,
            released: true,
            non_finite_reported: false,
            clamp_contacts_seen: 0,
            non_finite_fault_for_test: true,
            voice,
        });
        for index in 1..4 {
            ensemble.active[index] = Some(ResearchVoice::new(
                36 + index as u8,
                index as u8,
                index as u64,
                prepared_test_voice(ListeningPatch::MechanicalStab, 36 + index as u8),
            ));
        }
        ensemble.next_sequence = 4;

        ensemble.note_on(
            40,
            4,
            prepared_test_voice(ListeningPatch::MechanicalStab, 40),
        );

        assert_eq!(ensemble.diagnostics().last_stolen_note, Some(36));
        assert_eq!(ensemble.diagnostics().non_finite_voices, 1);
        assert_eq!(ensemble.diagnostics().clamp_contacts, contacts);
    }

    #[test]
    fn render_rejects_invalid_rates_and_scales_timeline_by_integer_ratio() {
        assert_eq!(
            render(ListeningPatch::BellMetal, 0),
            Err(RenderError::InvalidSampleRate)
        );
        assert_eq!(
            render(ListeningPatch::BellMetal, MIN_SAMPLE_RATE - 1),
            Err(RenderError::InvalidSampleRate)
        );
        assert_eq!(
            render(ListeningPatch::BellMetal, MAX_SAMPLE_RATE + 1),
            Err(RenderError::InvalidSampleRate)
        );
        let high_rate = render(ListeningPatch::BrassBass, 96_000).unwrap();
        assert_eq!(high_rate.samples.len(), 364_800 * 2);
        assert_eq!(high_rate.format.sample_rate, 96_000);
    }

    #[test]
    fn scaled_timeline_is_valid_at_minimum_and_exact_eight_times_rates() {
        let minimum = render(ListeningPatch::BellMetal, MIN_SAMPLE_RATE).unwrap();
        assert_eq!(minimum.samples.len(), 38_400 * 2);
        let eight_times = render(ListeningPatch::BrassBass, 384_000).unwrap();
        assert_eq!(eight_times.samples.len(), 1_459_200 * 2);
    }

    #[test]
    fn scaled_timeline_rejects_zero_duration_and_event_at_scaled_end() {
        const ZERO_DURATION_AFTER_SCALE: [ScoreEvent; 2] =
            [ScoreEvent::on(0, 60, 100), ScoreEvent::off(1, 60)];
        const EVENT_AT_SCALED_END: [ScoreEvent; 2] =
            [ScoreEvent::on(0, 60, 100), ScoreEvent::off(3, 60)];
        const COLLAPSED_EVENTS: [ScoreEvent; 2] =
            [ScoreEvent::on(0, 60, 100), ScoreEvent::off(1, 60)];
        let zero_duration = Score::new(2, &ZERO_DURATION_AFTER_SCALE).unwrap();
        let event_at_end = Score::new(4, &EVENT_AT_SCALED_END).unwrap();
        let collapsed = Score::new(100, &COLLAPSED_EVENTS).unwrap();

        assert_eq!(
            render_score(ListeningPatch::BellMetal, MIN_SAMPLE_RATE, zero_duration),
            Err(RenderError::InvalidScaledTimeline)
        );
        assert_eq!(
            render_score(ListeningPatch::BellMetal, MIN_SAMPLE_RATE, event_at_end),
            Err(RenderError::InvalidScaledTimeline)
        );
        assert_eq!(
            render_score(ListeningPatch::BellMetal, MIN_SAMPLE_RATE, collapsed),
            Err(RenderError::InvalidScaledTimeline)
        );
    }
}
