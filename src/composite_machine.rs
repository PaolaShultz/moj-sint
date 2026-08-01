use crate::dsp::hybrid::{HybridFamily, HybridFrame, HybridVoice};
use crate::hybrid::{
    HybridCondition, HybridEnsemble, HybridRenderError, HybridRenderSpec, PROGRESSION,
    render_hybrid,
};
use std::sync::Arc;
use thiserror::Error;

const ORIGINAL_COMPOSITE_GAIN: f32 = 0.16;
const HELD_CHORD: [u8; 3] = [50, 53, 57];
const SINGLE_NOTE: u8 = 38;
const DELAYED_LAUNCH_MS: [u32; 12] = [
    0, 180, 420, 710, 1_030, 1_380, 1_760, 2_170, 2_610, 3_080, 3_580, 4_110,
];
const MAX_LAYERS: usize = 12;
const IDENTITY_MATRIX: [[f32; 2]; 2] = [[1.0, 0.0], [0.0, 1.0]];
const MID_FOCUS_MATRIX: [[f32; 2]; 2] = [[0.86, 0.14], [0.14, 0.86]];
const OPEN_MATRIX: [[f32; 2]; 2] = [[1.04, -0.04], [-0.04, 1.04]];
pub const ACTIVE_BODY_START_SECONDS: f64 = 4.25;
pub const ACTIVE_BODY_END_SECONDS: f64 = 8.0;
pub const HOT_TARGET_RMS: f64 = 0.12;
pub const HOT_MINIMUM_RMS: f64 = 0.10;
pub const HOT_MAXIMUM_RMS: f64 = 0.14;
pub const HOT_PEAK_CEILING: f64 = 0.75;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositeCandidate {
    SynchronizedReference,
    DelayedLaunchEstimate,
    ReducedHeterogeneousStack,
    RoleSeparatedMachine,
    HarmonicLattice,
    CrossTopologyFollower,
    RiskyNonlinearBraid,
}

impl CompositeCandidate {
    pub const REFERENCES: [Self; 2] = [Self::SynchronizedReference, Self::DelayedLaunchEstimate];
    pub const FINAL_CANDIDATES: [Self; 5] = [
        Self::ReducedHeterogeneousStack,
        Self::RoleSeparatedMachine,
        Self::HarmonicLattice,
        Self::CrossTopologyFollower,
        Self::RiskyNonlinearBraid,
    ];
    pub const ALL: [Self; 7] = [
        Self::SynchronizedReference,
        Self::DelayedLaunchEstimate,
        Self::ReducedHeterogeneousStack,
        Self::RoleSeparatedMachine,
        Self::HarmonicLattice,
        Self::CrossTopologyFollower,
        Self::RiskyNonlinearBraid,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::SynchronizedReference => "synchronized-reference",
            Self::DelayedLaunchEstimate => "delayed-launch-estimate",
            Self::ReducedHeterogeneousStack => "reduced-heterogeneous-stack",
            Self::RoleSeparatedMachine => "role-separated-machine",
            Self::HarmonicLattice => "harmonic-lattice",
            Self::CrossTopologyFollower => "cross-topology-follower",
            Self::RiskyNonlinearBraid => "risky-nonlinear-braid",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutputGainPolicy {
    pub linear: f32,
    pub decibels: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditionKind {
    HeldOctave,
    BassPedal,
    Punch,
}

pub fn audition_gain_policy(candidate: CompositeCandidate, kind: AuditionKind) -> OutputGainPolicy {
    let linear = match kind {
        // These tables are deliberately independent of pitch. They are
        // calibrated from the complete three-register set, never per file.
        AuditionKind::HeldOctave => match candidate {
            CompositeCandidate::ReducedHeterogeneousStack => 3.52,
            CompositeCandidate::RoleSeparatedMachine => 4.11,
            CompositeCandidate::HarmonicLattice => 3.64,
            CompositeCandidate::CrossTopologyFollower => 3.76,
            CompositeCandidate::RiskyNonlinearBraid => 10.10,
            _ => gain_policy(candidate).linear,
        },
        AuditionKind::BassPedal => match candidate {
            CompositeCandidate::ReducedHeterogeneousStack => 3.498,
            CompositeCandidate::RoleSeparatedMachine => 3.678,
            CompositeCandidate::HarmonicLattice => 3.76,
            CompositeCandidate::CrossTopologyFollower => 3.974,
            CompositeCandidate::RiskyNonlinearBraid => 4.729,
            _ => gain_policy(candidate).linear,
        },
        AuditionKind::Punch => match candidate {
            CompositeCandidate::ReducedHeterogeneousStack => 4.114,
            CompositeCandidate::RoleSeparatedMachine => 4.493,
            CompositeCandidate::HarmonicLattice => 4.609,
            CompositeCandidate::CrossTopologyFollower => 5.03,
            CompositeCandidate::RiskyNonlinearBraid => 6.665,
            _ => gain_policy(candidate).linear,
        },
    };
    OutputGainPolicy {
        linear,
        decibels: 20.0 * f64::from(linear).log10(),
    }
}

pub fn gain_policy(candidate: CompositeCandidate) -> OutputGainPolicy {
    let linear = match candidate {
        CompositeCandidate::SynchronizedReference => 1.731_418_3,
        CompositeCandidate::DelayedLaunchEstimate => 2.881_948,
        CompositeCandidate::ReducedHeterogeneousStack => 3.979_082,
        CompositeCandidate::RoleSeparatedMachine => 4.638_279,
        CompositeCandidate::HarmonicLattice => 4.906_858,
        CompositeCandidate::CrossTopologyFollower => 5.123_818,
        CompositeCandidate::RiskyNonlinearBraid => 6.633_916_4,
    };
    OutputGainPolicy {
        linear,
        decibels: 20.0 * f64::from(linear).log10(),
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ResearchAdsrError {
    #[error("ADSR times must be finite and positive")]
    InvalidTime,
    #[error("ADSR sustain must be finite and between zero and one")]
    InvalidSustain,
    #[error("ADSR sample rate must be positive")]
    InvalidSampleRate,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResearchAdsrConfig {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_level: f32,
    pub release_seconds: f32,
}

impl ResearchAdsrConfig {
    pub fn new(
        attack_seconds: f32,
        decay_seconds: f32,
        sustain_level: f32,
        release_seconds: f32,
    ) -> Result<Self, ResearchAdsrError> {
        if !attack_seconds.is_finite()
            || attack_seconds <= 0.0
            || !decay_seconds.is_finite()
            || decay_seconds <= 0.0
            || !release_seconds.is_finite()
            || release_seconds <= 0.0
        {
            return Err(ResearchAdsrError::InvalidTime);
        }
        if !sustain_level.is_finite() || !(0.0..=1.0).contains(&sustain_level) {
            return Err(ResearchAdsrError::InvalidSustain);
        }
        Ok(Self {
            attack_seconds,
            decay_seconds,
            sustain_level,
            release_seconds,
        })
    }
}

pub fn retained_punch_config() -> ResearchAdsrConfig {
    ResearchAdsrConfig {
        attack_seconds: 0.003,
        decay_seconds: 0.110,
        sustain_level: 0.55,
        release_seconds: 0.180,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResearchAdsrStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug)]
pub struct ResearchAdsr {
    config: ResearchAdsrConfig,
    attack_samples: u32,
    decay_samples: u32,
    release_samples: u32,
    stage: ResearchAdsrStage,
    stage_sample: u32,
    level: f32,
    release_start: f32,
}

impl ResearchAdsr {
    pub fn new(sample_rate: u32, config: ResearchAdsrConfig) -> Result<Self, ResearchAdsrError> {
        if sample_rate == 0 {
            return Err(ResearchAdsrError::InvalidSampleRate);
        }
        let to_samples = |seconds: f32| ((seconds * sample_rate as f32).round() as u32).max(1);
        Ok(Self {
            config,
            attack_samples: to_samples(config.attack_seconds),
            decay_samples: to_samples(config.decay_seconds),
            release_samples: to_samples(config.release_seconds),
            stage: ResearchAdsrStage::Idle,
            stage_sample: 0,
            level: 0.0,
            release_start: 0.0,
        })
    }

    pub fn note_on(&mut self) {
        self.stage = ResearchAdsrStage::Attack;
        self.stage_sample = 0;
        self.level = 0.0;
        self.release_start = 0.0;
    }

    pub fn note_off(&mut self) {
        if self.stage != ResearchAdsrStage::Idle {
            self.stage = ResearchAdsrStage::Release;
            self.stage_sample = 0;
            self.release_start = self.level;
        }
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        match self.stage {
            ResearchAdsrStage::Idle => self.level = 0.0,
            ResearchAdsrStage::Attack => {
                self.stage_sample += 1;
                self.level = self.stage_sample as f32 / self.attack_samples as f32;
                if self.stage_sample >= self.attack_samples {
                    self.level = 1.0;
                    self.stage = ResearchAdsrStage::Decay;
                    self.stage_sample = 0;
                }
            }
            ResearchAdsrStage::Decay => {
                self.stage_sample += 1;
                let progress = self.stage_sample as f32 / self.decay_samples as f32;
                self.level = 1.0 - (1.0 - self.config.sustain_level) * progress;
                if self.stage_sample >= self.decay_samples {
                    self.level = self.config.sustain_level;
                    self.stage = ResearchAdsrStage::Sustain;
                    self.stage_sample = 0;
                }
            }
            ResearchAdsrStage::Sustain => self.level = self.config.sustain_level,
            ResearchAdsrStage::Release => {
                self.stage_sample += 1;
                let progress = self.stage_sample as f32 / self.release_samples as f32;
                self.level = self.release_start * (1.0 - progress);
                if self.stage_sample >= self.release_samples {
                    self.level = 0.0;
                    self.stage = ResearchAdsrStage::Idle;
                    self.stage_sample = 0;
                }
            }
        }
        self.level = self.level.clamp(0.0, 1.0);
        self.level
    }

    pub fn level(&self) -> f32 {
        self.level
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum CompositeError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error(transparent)]
    Render(#[from] HybridRenderError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Score {
    Single,
    HeldChord,
    Progression,
    ProgressionMono,
    Drone,
    LongHeldChord,
    ProgressionVoice(u8),
}

impl Score {
    fn duration_seconds(self) -> f64 {
        match self {
            Self::Single => 10.0,
            Self::HeldChord => 12.0,
            Self::Progression
            | Self::ProgressionMono
            | Self::Drone
            | Self::LongHeldChord
            | Self::ProgressionVoice(_) => 16.0,
        }
    }

    fn target_rms(self) -> f64 {
        match self {
            Self::Single => 0.075,
            Self::Drone => 0.070,
            _ => 0.065,
        }
    }

    fn notes_at_seconds(self, seconds: f64) -> NoteInventory {
        let mut inventory = NoteInventory::default();
        if seconds < 0.0 || seconds >= self.duration_seconds() {
            return inventory;
        }
        match self {
            Self::Single => inventory.add(SINGLE_NOTE),
            Self::HeldChord => inventory.add_notes(HELD_CHORD),
            Self::Progression | Self::ProgressionMono => {
                let segment = ((seconds / 4.0) as usize).min(3);
                inventory.add_notes(PROGRESSION[segment]);
            }
            Self::Drone => inventory.add(SINGLE_NOTE),
            Self::LongHeldChord => inventory.add_notes(HELD_CHORD),
            Self::ProgressionVoice(voice) => {
                let segment = ((seconds / 4.0) as usize).min(3);
                inventory.add(PROGRESSION[segment][usize::from(voice.min(2))]);
            }
        }
        inventory
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OriginalLayer {
    CrossSingle,
    CrossChord,
    CrossProgression,
    CrossProgressionMono,
    SpectralSingle,
    SpectralChord,
    SpectralProgression,
    SpectralProgressionMono,
    DualSingle,
    DualChord,
    DualProgression,
    DualProgressionMono,
}

impl OriginalLayer {
    pub const ALL: [Self; 12] = [
        Self::CrossSingle,
        Self::CrossChord,
        Self::CrossProgression,
        Self::CrossProgressionMono,
        Self::SpectralSingle,
        Self::SpectralChord,
        Self::SpectralProgression,
        Self::SpectralProgressionMono,
        Self::DualSingle,
        Self::DualChord,
        Self::DualProgression,
        Self::DualProgressionMono,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::CrossSingle => "cross-single",
            Self::CrossChord => "cross-chord",
            Self::CrossProgression => "cross-progression",
            Self::CrossProgressionMono => "cross-progression-mono",
            Self::SpectralSingle => "spectral-single",
            Self::SpectralChord => "spectral-chord",
            Self::SpectralProgression => "spectral-progression",
            Self::SpectralProgressionMono => "spectral-progression-mono",
            Self::DualSingle => "dual-single",
            Self::DualChord => "dual-chord",
            Self::DualProgression => "dual-progression",
            Self::DualProgressionMono => "dual-progression-mono",
        }
    }

    pub const fn family(self) -> HybridFamily {
        match self {
            Self::CrossSingle
            | Self::CrossChord
            | Self::CrossProgression
            | Self::CrossProgressionMono => HybridFamily::CrossCoupledMachine,
            Self::SpectralSingle
            | Self::SpectralChord
            | Self::SpectralProgression
            | Self::SpectralProgressionMono => HybridFamily::SpectralShadow,
            Self::DualSingle
            | Self::DualChord
            | Self::DualProgression
            | Self::DualProgressionMono => HybridFamily::DualResonantBody,
        }
    }

    pub const fn condition(self) -> HybridCondition {
        match self {
            Self::CrossSingle | Self::SpectralSingle | Self::DualSingle => HybridCondition::Single,
            Self::CrossChord | Self::SpectralChord | Self::DualChord => HybridCondition::HeldChord,
            Self::CrossProgression | Self::SpectralProgression | Self::DualProgression => {
                HybridCondition::Progression
            }
            Self::CrossProgressionMono
            | Self::SpectralProgressionMono
            | Self::DualProgressionMono => HybridCondition::ProgressionMono,
        }
    }

    pub const fn delayed_start_ms(self) -> u32 {
        DELAYED_LAUNCH_MS[self.index()]
    }

    const fn index(self) -> usize {
        match self {
            Self::CrossSingle => 0,
            Self::CrossChord => 1,
            Self::CrossProgression => 2,
            Self::CrossProgressionMono => 3,
            Self::SpectralSingle => 4,
            Self::SpectralChord => 5,
            Self::SpectralProgression => 6,
            Self::SpectralProgressionMono => 7,
            Self::DualSingle => 8,
            Self::DualChord => 9,
            Self::DualProgression => 10,
            Self::DualProgressionMono => 11,
        }
    }

    const fn score(self) -> Score {
        match self.condition() {
            HybridCondition::Single => Score::Single,
            HybridCondition::HeldChord => Score::HeldChord,
            HybridCondition::Progression => Score::Progression,
            HybridCondition::ProgressionMono => Score::ProgressionMono,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoteInventory {
    counts: [u8; 128],
    pub total_voices: usize,
}

impl Default for NoteInventory {
    fn default() -> Self {
        Self {
            counts: [0; 128],
            total_voices: 0,
        }
    }
}

impl NoteInventory {
    fn add(&mut self, note: u8) {
        self.counts[usize::from(note)] = self.counts[usize::from(note)].saturating_add(1);
        self.total_voices += 1;
    }

    fn add_notes(&mut self, notes: [u8; 3]) {
        for note in notes {
            self.add(note);
        }
    }

    fn merge(&mut self, other: Self) {
        for (target, source) in self.counts.iter_mut().zip(other.counts) {
            *target = target.saturating_add(source);
        }
        self.total_voices += other.total_voices;
    }

    pub fn note_count(&self, note: u8) -> usize {
        usize::from(self.counts[usize::from(note)])
    }

    pub fn iter(&self) -> impl Iterator<Item = (u8, u8)> + '_ {
        self.counts
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, count)| *count > 0)
            .map(|(note, count)| (note as u8, count))
    }
}

#[derive(Clone, Copy, Debug)]
struct LayerSpec {
    family: HybridFamily,
    score: Score,
    start_ms: u32,
    label: &'static str,
    mix_gain: f32,
    matrix: [[f32; 2]; 2],
}

#[derive(Clone, Debug)]
struct CompositeLayer {
    spec: LayerSpec,
    samples: Arc<[f32]>,
    start_frame: usize,
    duration_frames: usize,
}

impl CompositeLayer {
    fn new(spec: LayerSpec, sample_rate: u32) -> Result<Self, HybridRenderError> {
        let samples = prepare_layer_samples(spec, sample_rate)?;
        let duration_frames = samples.len() / 2;
        Ok(Self {
            spec,
            samples: samples.into(),
            start_frame: ((f64::from(spec.start_ms) * f64::from(sample_rate)) / 1_000.0).round()
                as usize,
            duration_frames,
        })
    }

    #[inline]
    fn sample_raw(&self, frame_index: usize) -> HybridFrame {
        if frame_index < self.start_frame
            || frame_index >= self.start_frame.saturating_add(self.duration_frames)
        {
            return HybridFrame::default();
        }
        let local = frame_index - self.start_frame;
        HybridFrame {
            left: self.samples[2 * local],
            right: self.samples[2 * local + 1],
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompositeMachine {
    candidate: CompositeCandidate,
    sample_rate: u32,
    frame_index: usize,
    duration_frames: usize,
    layers: Vec<CompositeLayer>,
    follower_state: f32,
    last_follower_control: f32,
    last_junction: f32,
    junction_dc_state: [f32; 2],
    junction_dc_alpha: f32,
    junction_enabled: bool,
    muted_layer: Option<usize>,
}

impl CompositeMachine {
    pub fn new(candidate: CompositeCandidate, sample_rate: u32) -> Result<Self, CompositeError> {
        Self::new_with_mute(candidate, sample_rate, None)
    }

    fn new_with_mute(
        candidate: CompositeCandidate,
        sample_rate: u32,
        muted_layer: Option<usize>,
    ) -> Result<Self, CompositeError> {
        if sample_rate == 0 {
            return Err(CompositeError::InvalidSampleRate);
        }
        let specs = candidate_layer_specs(candidate);
        let mut layers = Vec::with_capacity(specs.len());
        for spec in specs {
            layers.push(CompositeLayer::new(spec, sample_rate)?);
        }
        let duration_frames = layers
            .iter()
            .map(|layer| layer.start_frame + layer.duration_frames)
            .max()
            .unwrap_or(0);
        Ok(Self {
            candidate,
            sample_rate,
            frame_index: 0,
            duration_frames,
            layers,
            follower_state: 0.0,
            last_follower_control: 0.75,
            last_junction: 0.0,
            junction_dc_state: [0.0; 2],
            junction_dc_alpha: 1.0 - (-std::f32::consts::TAU * 8.0 / sample_rate as f32).exp(),
            junction_enabled: true,
            muted_layer,
        })
    }

    #[inline]
    pub fn sample(&mut self) -> HybridFrame {
        let mut raw = [HybridFrame::default(); MAX_LAYERS];
        for (index, layer) in self.layers.iter().enumerate() {
            if self.muted_layer != Some(index) {
                raw[index] = layer.sample_raw(self.frame_index);
            }
        }

        let follower_control = if self.candidate == CompositeCandidate::CrossTopologyFollower {
            let measurement = (0.5 * (raw[0].left + raw[0].right)).abs();
            let coefficient = if measurement > self.follower_state {
                0.012
            } else {
                0.0007
            };
            self.follower_state += coefficient * (measurement - self.follower_state);
            (0.75 + 7.0 * self.follower_state).clamp(0.75, 1.25)
        } else {
            1.0
        };
        self.last_follower_control = follower_control;

        let mut output = HybridFrame::default();
        for (index, layer) in self.layers.iter().enumerate() {
            let frame = raw[index];
            let follower_gain =
                if self.candidate == CompositeCandidate::CrossTopologyFollower && index == 1 {
                    follower_control
                } else {
                    1.0
                };
            let gain = layer.spec.mix_gain * follower_gain;
            let matrix_left =
                layer.spec.matrix[0][0] * frame.left + layer.spec.matrix[0][1] * frame.right;
            let matrix_right =
                layer.spec.matrix[1][0] * frame.left + layer.spec.matrix[1][1] * frame.right;
            if self.candidate == CompositeCandidate::CrossTopologyFollower && index == 2 {
                let mid = 0.5 * (matrix_left + matrix_right);
                let side = 0.5 * (matrix_left - matrix_right);
                let side_gain = 0.65 + 1.4 * (follower_control - 0.75);
                output.left += gain * (mid + side_gain * side);
                output.right += gain * (mid - side_gain * side);
            } else {
                output.left += gain * matrix_left;
                output.right += gain * matrix_right;
            }
        }
        self.last_junction = 0.0;
        if self.candidate == CompositeCandidate::RiskyNonlinearBraid && self.junction_enabled {
            let a = 0.5 * (raw[0].left + raw[0].right);
            let b = 0.5 * (raw[1].left + raw[1].right);
            let c = 0.5 * (raw[2].left + raw[2].right);
            let ab = a * b;
            let bc = b * c;
            let ca = c * a;
            let center = 0.06 * (ab + bc + ca);
            let side = 0.045 * (ab - bc);
            let junction = [
                (center + side).clamp(-0.06, 0.06),
                (center - side).clamp(-0.06, 0.06),
            ];
            let mut high_passed = [0.0; 2];
            for channel in 0..2 {
                self.junction_dc_state[channel] +=
                    self.junction_dc_alpha * (junction[channel] - self.junction_dc_state[channel]);
                high_passed[channel] = junction[channel] - self.junction_dc_state[channel];
            }
            output.left += high_passed[0];
            output.right += high_passed[1];
            self.last_junction = high_passed[0];
        }
        self.frame_index = self.frame_index.saturating_add(1);
        output
    }

    pub fn reset(&mut self) {
        self.frame_index = 0;
        self.follower_state = 0.0;
        self.last_follower_control = 0.75;
        self.last_junction = 0.0;
        self.junction_dc_state = [0.0; 2];
    }

    pub fn candidate(&self) -> CompositeCandidate {
        self.candidate
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }

    pub fn duration_seconds(&self) -> f64 {
        self.duration_frames as f64 / f64::from(self.sample_rate)
    }

    pub fn inventory_at_seconds(&self, seconds: f64) -> NoteInventory {
        let mut inventory = NoteInventory::default();
        for layer in &self.layers {
            let local = seconds - f64::from(layer.spec.start_ms) / 1_000.0;
            inventory.merge(layer.spec.score.notes_at_seconds(local));
        }
        inventory
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn family_count(&self) -> usize {
        HybridFamily::ALL
            .iter()
            .filter(|family| {
                self.layers
                    .iter()
                    .any(|layer| layer.spec.family == **family)
            })
            .count()
    }

    pub fn topology_signature(&self) -> &'static str {
        match self.candidate {
            CompositeCandidate::SynchronizedReference => "12-layer synchronized original schedules",
            CompositeCandidate::DelayedLaunchEstimate => {
                "12-layer staggered process-launch estimate"
            }
            CompositeCandidate::ReducedHeterogeneousStack => {
                "single+held+two stereo progressions without mono duplicates"
            }
            CompositeCandidate::RoleSeparatedMachine => {
                "D2 anchor+held spectral object+transition resonant body"
            }
            CompositeCandidate::HarmonicLattice => {
                "D2 lattice+heterogeneous individual progression voices"
            }
            CompositeCandidate::CrossTopologyFollower => {
                "cross envelope measurement controls spectral gain"
            }
            CompositeCandidate::RiskyNonlinearBraid => {
                "three progression bodies+pairwise exchange junction+internal DC control"
            }
        }
    }

    pub fn last_follower_control(&self) -> f32 {
        self.last_follower_control
    }

    pub fn last_junction(&self) -> f32 {
        self.last_junction
    }

    pub fn layer_info(&self) -> Vec<LayerInfo> {
        self.layers
            .iter()
            .enumerate()
            .map(|(index, layer)| LayerInfo {
                index,
                label: layer.spec.label,
                family: layer.spec.family,
                start_ms: layer.spec.start_ms,
                duration_ms: (1_000.0 * layer.duration_frames as f64 / f64::from(self.sample_rate))
                    .round() as u32,
                mix_gain: layer.spec.mix_gain,
                matrix: layer.spec.matrix,
            })
            .collect()
    }

    pub fn frequency_timeline(&self) -> Vec<TimelineInterval> {
        let mut boundaries = vec![0_u32, (1_000.0 * self.duration_seconds()).round() as u32];
        for layer in &self.layers {
            let start = layer.spec.start_ms;
            boundaries.push(start);
            boundaries.push(start + (1_000.0 * layer.spec.score.duration_seconds()).round() as u32);
            if matches!(
                layer.spec.score,
                Score::Progression | Score::ProgressionMono | Score::ProgressionVoice(_)
            ) {
                boundaries.extend([start + 4_000, start + 8_000, start + 12_000]);
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        boundaries
            .windows(2)
            .filter_map(|window| {
                if window[0] == window[1] {
                    return None;
                }
                let middle_seconds = f64::from(window[0] + window[1]) / 2_000.0;
                Some(TimelineInterval {
                    start_ms: window[0],
                    end_ms: window[1],
                    inventory: self.inventory_at_seconds(middle_seconds),
                })
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerInfo {
    pub index: usize,
    pub label: &'static str,
    pub family: HybridFamily,
    pub start_ms: u32,
    pub duration_ms: u32,
    pub mix_gain: f32,
    pub matrix: [[f32; 2]; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimelineInterval {
    pub start_ms: u32,
    pub end_ms: u32,
    pub inventory: NoteInventory,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompositeMetrics {
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub crest_factor: f64,
    pub maximum_jump: f64,
    pub low_rms: f64,
    pub mid_rms: f64,
    pub high_rms: f64,
    pub brightness_ratio: f64,
    pub spectral_flux: f64,
    pub correlation: f64,
    pub side_to_mid: f64,
    pub low_side_to_mid: f64,
    pub mono_rms: f64,
    pub mono_to_stereo: f64,
    pub difference_rms: f64,
    pub onset_peak_ms: f64,
    pub early_rms: f64,
    pub body_rms: f64,
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompositeRender {
    pub samples: Vec<f32>,
    pub metrics: CompositeMetrics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HotCompositeRender {
    pub samples: Vec<f32>,
    pub metrics: CompositeMetrics,
    pub active_body_rms: f64,
    pub raw_hash: u64,
    pub gain: OutputGainPolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuditionRender {
    pub samples: Vec<f32>,
    pub metrics: CompositeMetrics,
    pub active_body_rms: f64,
    pub raw_rms: f64,
    pub raw_peak: f64,
    pub raw_hash: u64,
    pub gain: OutputGainPolicy,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PunchMetrics {
    pub window_rms: [f64; 4],
    pub window_peak: [f64; 4],
    pub onset_to_sustain_ratio: f64,
    pub attack_time_ms: f64,
    pub decay_settling_ms: f64,
    pub maximum_jump: f64,
    pub low_band_transient_rms: f64,
    pub fundamental_decay_change_db: f64,
    pub release_continuity_jump: f64,
    pub tail_duration_ms: f64,
    pub peak: f64,
    pub crest_factor: f64,
    pub finite: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PunchSweepRow {
    pub config: ResearchAdsrConfig,
    pub metrics: PunchMetrics,
    pub retained: bool,
    pub status: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AblationRow {
    pub layer_index: usize,
    pub label: &'static str,
    pub difference_db: f64,
}

fn candidate_layer_specs(candidate: CompositeCandidate) -> Vec<LayerSpec> {
    match candidate {
        CompositeCandidate::SynchronizedReference => original_layer_specs(false),
        CompositeCandidate::DelayedLaunchEstimate => original_layer_specs(true),
        CompositeCandidate::ReducedHeterogeneousStack => vec![
            custom_spec(
                HybridFamily::CrossCoupledMachine,
                Score::Single,
                "cross-single-anchor",
                0.24,
                MID_FOCUS_MATRIX,
            ),
            custom_spec(
                HybridFamily::SpectralShadow,
                Score::HeldChord,
                "spectral-held-chord",
                0.21,
                OPEN_MATRIX,
            ),
            custom_spec(
                HybridFamily::CrossCoupledMachine,
                Score::Progression,
                "cross-progression",
                0.15,
                IDENTITY_MATRIX,
            ),
            custom_spec(
                HybridFamily::DualResonantBody,
                Score::Progression,
                "dual-progression",
                0.18,
                OPEN_MATRIX,
            ),
        ],
        CompositeCandidate::RoleSeparatedMachine => vec![
            custom_spec(
                HybridFamily::CrossCoupledMachine,
                Score::Drone,
                "cross-d2-anchor",
                0.28,
                MID_FOCUS_MATRIX,
            ),
            custom_spec(
                HybridFamily::SpectralShadow,
                Score::LongHeldChord,
                "spectral-harmonic-object",
                0.22,
                OPEN_MATRIX,
            ),
            custom_spec(
                HybridFamily::DualResonantBody,
                Score::Progression,
                "dual-transition-body",
                0.20,
                IDENTITY_MATRIX,
            ),
        ],
        CompositeCandidate::HarmonicLattice => vec![
            custom_spec(
                HybridFamily::CrossCoupledMachine,
                Score::Drone,
                "cross-d2-lattice",
                0.25,
                MID_FOCUS_MATRIX,
            ),
            custom_spec(
                HybridFamily::CrossCoupledMachine,
                Score::ProgressionVoice(0),
                "cross-lower-relation",
                0.15,
                MID_FOCUS_MATRIX,
            ),
            custom_spec(
                HybridFamily::SpectralShadow,
                Score::ProgressionVoice(1),
                "spectral-middle-relation",
                0.19,
                OPEN_MATRIX,
            ),
            custom_spec(
                HybridFamily::DualResonantBody,
                Score::ProgressionVoice(2),
                "dual-upper-relation",
                0.19,
                IDENTITY_MATRIX,
            ),
        ],
        CompositeCandidate::CrossTopologyFollower => vec![
            custom_spec(
                HybridFamily::CrossCoupledMachine,
                Score::Drone,
                "cross-envelope-source",
                0.25,
                MID_FOCUS_MATRIX,
            ),
            custom_spec(
                HybridFamily::SpectralShadow,
                Score::Progression,
                "spectral-follower-destination",
                0.21,
                OPEN_MATRIX,
            ),
            custom_spec(
                HybridFamily::DualResonantBody,
                Score::ProgressionVoice(2),
                "dual-side-response",
                0.18,
                IDENTITY_MATRIX,
            ),
        ],
        CompositeCandidate::RiskyNonlinearBraid => vec![
            custom_spec(
                HybridFamily::CrossCoupledMachine,
                Score::Progression,
                "cross-pairwise-source",
                0.10,
                MID_FOCUS_MATRIX,
            ),
            custom_spec(
                HybridFamily::SpectralShadow,
                Score::Progression,
                "spectral-pairwise-source",
                0.10,
                IDENTITY_MATRIX,
            ),
            custom_spec(
                HybridFamily::DualResonantBody,
                Score::Progression,
                "dual-pairwise-source",
                0.10,
                OPEN_MATRIX,
            ),
        ],
    }
}

fn custom_spec(
    family: HybridFamily,
    score: Score,
    label: &'static str,
    mix_gain: f32,
    matrix: [[f32; 2]; 2],
) -> LayerSpec {
    LayerSpec {
        family,
        score,
        start_ms: 0,
        label,
        mix_gain,
        matrix,
    }
}

fn original_layer_specs(delayed: bool) -> Vec<LayerSpec> {
    OriginalLayer::ALL
        .into_iter()
        .map(|layer| original_layer_spec(layer, delayed))
        .collect()
}

fn original_layer_spec(layer: OriginalLayer, delayed: bool) -> LayerSpec {
    LayerSpec {
        family: layer.family(),
        score: layer.score(),
        start_ms: if delayed { layer.delayed_start_ms() } else { 0 },
        label: layer.label(),
        mix_gain: ORIGINAL_COMPOSITE_GAIN,
        matrix: IDENTITY_MATRIX,
    }
}

pub fn render_original_layer(
    layer: OriginalLayer,
    sample_rate: u32,
) -> Result<Vec<f32>, HybridRenderError> {
    prepare_layer_samples(original_layer_spec(layer, true), sample_rate)
}

pub const fn original_composite_layer_gain() -> f32 {
    ORIGINAL_COMPOSITE_GAIN
}

fn prepare_layer_samples(spec: LayerSpec, sample_rate: u32) -> Result<Vec<f32>, HybridRenderError> {
    let mut samples = match spec.score {
        Score::Single | Score::HeldChord | Score::Progression | Score::ProgressionMono => {
            let condition = match spec.score {
                Score::Single => HybridCondition::Single,
                Score::HeldChord => HybridCondition::HeldChord,
                Score::Progression => HybridCondition::Progression,
                Score::ProgressionMono => HybridCondition::ProgressionMono,
                _ => unreachable!(),
            };
            render_hybrid(HybridRenderSpec {
                sample_rate,
                family: spec.family,
                condition,
            })?
            .samples
        }
        Score::Drone => render_long_voice(spec.family, sample_rate, SINGLE_NOTE)?,
        Score::LongHeldChord => render_long_chord(spec.family, sample_rate)?,
        Score::ProgressionVoice(voice) => {
            render_progression_voice(spec.family, sample_rate, usize::from(voice.min(2)))?
        }
    };
    let energy = samples
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    let rms = (energy / samples.len().max(1) as f64).sqrt();
    let peak = samples
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0, f64::max);
    if rms > 0.0 && peak > 0.0 {
        let gain = (spec.score.target_rms() / rms).min(0.979 / peak) as f32;
        for sample in &mut samples {
            *sample *= gain;
        }
    }
    Ok(samples)
}

fn render_long_voice(
    family: HybridFamily,
    sample_rate: u32,
    note: u8,
) -> Result<Vec<f32>, HybridRenderError> {
    let mut voice = HybridVoice::new(
        family,
        sample_rate as f32,
        midi_frequency(note),
        0xc011_a000 ^ u32::from(note),
    )?;
    let frames = 16 * sample_rate as usize;
    let mut samples = Vec::with_capacity(2 * frames);
    for _ in 0..frames {
        let frame = voice.sample();
        samples.push(frame.left);
        samples.push(frame.right);
    }
    apply_stereo_fade(&mut samples, sample_rate, 0.02);
    Ok(samples)
}

fn render_long_chord(
    family: HybridFamily,
    sample_rate: u32,
) -> Result<Vec<f32>, HybridRenderError> {
    let mut ensemble = HybridEnsemble::new(family, sample_rate as f32, HELD_CHORD, 0xc011_c057)?;
    let frames = 16 * sample_rate as usize;
    let mut samples = Vec::with_capacity(2 * frames);
    for _ in 0..frames {
        let frame = ensemble.sample();
        samples.push(frame.left);
        samples.push(frame.right);
    }
    apply_stereo_fade(&mut samples, sample_rate, 0.02);
    Ok(samples)
}

fn render_progression_voice(
    family: HybridFamily,
    sample_rate: u32,
    voice_index: usize,
) -> Result<Vec<f32>, HybridRenderError> {
    let segment_frames = 4 * sample_rate as usize;
    let mut samples = Vec::with_capacity(8 * segment_frames);
    for (segment, notes) in PROGRESSION.iter().enumerate() {
        let note = notes[voice_index];
        let mut voice = HybridVoice::new(
            family,
            sample_rate as f32,
            midi_frequency(note),
            0xc011_1000 ^ (segment as u32 * 0x9e37) ^ voice_index as u32,
        )?;
        let start = samples.len();
        for _ in 0..segment_frames {
            let frame = voice.sample();
            samples.push(frame.left);
            samples.push(frame.right);
        }
        apply_stereo_fade(&mut samples[start..], sample_rate, 0.025);
    }
    apply_stereo_fade(&mut samples, sample_rate, 0.01);
    Ok(samples)
}

fn apply_stereo_fade(samples: &mut [f32], sample_rate: u32, seconds: f32) {
    let frames = samples.len() / 2;
    if frames == 0 {
        return;
    }
    let fade = ((sample_rate as f32 * seconds).round() as usize).clamp(1, frames / 2);
    for index in 0..frames {
        let gain = (index as f32 / fade as f32)
            .min(1.0)
            .min(((frames - 1 - index) as f32 / fade as f32).min(1.0));
        samples[2 * index] *= gain;
        samples[2 * index + 1] *= gain;
    }
}

pub fn render_composite(
    candidate: CompositeCandidate,
    sample_rate: u32,
    mono: bool,
) -> Result<CompositeRender, CompositeError> {
    render_composite_with_mute(candidate, sample_rate, mono, None)
}

pub fn render_hot_composite(
    candidate: CompositeCandidate,
    sample_rate: u32,
) -> Result<HotCompositeRender, CompositeError> {
    let raw = render_composite(candidate, sample_rate, false)?;
    let raw_hash = raw.metrics.sample_hash;
    let gain = gain_policy(candidate);
    let samples: Vec<f32> = raw
        .samples
        .iter()
        .map(|sample| sample * gain.linear)
        .collect();
    let metrics = measure_composite(&samples, sample_rate as f32);
    let active_body_rms = window_rms(
        &samples,
        sample_rate,
        ACTIVE_BODY_START_SECONDS,
        ACTIVE_BODY_END_SECONDS,
    );
    Ok(HotCompositeRender {
        samples,
        metrics,
        active_body_rms,
        raw_hash,
        gain,
    })
}

pub fn render_held_candidate(
    candidate: CompositeCandidate,
    note: u8,
    sample_rate: u32,
) -> Result<AuditionRender, CompositeError> {
    let raw_samples = render_held_candidate_raw(candidate, note, sample_rate)?;
    Ok(prepare_audition_render(
        raw_samples,
        sample_rate,
        audition_gain_policy(candidate, AuditionKind::HeldOctave),
        0.25,
        3.75,
    ))
}

pub fn render_bass_candidate(
    candidate: CompositeCandidate,
    sample_rate: u32,
    punch: bool,
) -> Result<AuditionRender, CompositeError> {
    let kind = if punch {
        AuditionKind::Punch
    } else {
        AuditionKind::BassPedal
    };
    let raw_samples =
        render_bass_candidate_raw(candidate, sample_rate, punch.then(retained_punch_config))?;
    Ok(prepare_audition_render(
        raw_samples,
        sample_rate,
        audition_gain_policy(candidate, kind),
        ACTIVE_BODY_START_SECONDS,
        ACTIVE_BODY_END_SECONDS,
    ))
}

fn render_bass_candidate_raw(
    candidate: CompositeCandidate,
    sample_rate: u32,
    punch_config: Option<ResearchAdsrConfig>,
) -> Result<Vec<f32>, CompositeError> {
    let mut raw = render_composite(candidate, sample_rate, false)?.samples;
    let frames = raw.len() / 2;
    let family = candidate_layer_specs(candidate)
        .first()
        .map(|spec| spec.family)
        .unwrap_or(HybridFamily::CrossCoupledMachine);
    let mut pedal = HybridVoice::new(
        family,
        sample_rate as f32,
        midi_frequency(26),
        0xd100_0000 ^ candidate as u32,
    )
    .map_err(HybridRenderError::from)?;
    let mut envelope = punch_config
        .map(|config| ResearchAdsr::new(sample_rate, config))
        .transpose()
        .map_err(|_| CompositeError::InvalidSampleRate)?;
    let segment_frames = 4 * sample_rate as usize;
    let gate_frames = 3 * sample_rate as usize / 4;
    let fade_frames = ((0.02 * sample_rate as f32).round() as usize).max(1);
    for frame_index in 0..frames {
        let envelope_gain = if let Some(envelope) = &mut envelope {
            let segment_frame = frame_index % segment_frames;
            if segment_frame == 0 {
                envelope.note_on();
            } else if segment_frame == gate_frames {
                envelope.note_off();
            }
            envelope.sample()
        } else {
            (frame_index as f32 / fade_frames as f32)
                .min(1.0)
                .min(((frames - 1 - frame_index) as f32 / fade_frames as f32).min(1.0))
        };
        let frame = pedal.sample();
        let mid_left = 0.86 * frame.left + 0.14 * frame.right;
        let mid_right = 0.14 * frame.left + 0.86 * frame.right;
        raw[2 * frame_index] += 0.18 * envelope_gain * mid_left;
        raw[2 * frame_index + 1] += 0.18 * envelope_gain * mid_right;
    }
    Ok(raw)
}

pub fn punch_sweep(
    candidate: CompositeCandidate,
    sample_rate: u32,
) -> Result<Vec<PunchSweepRow>, CompositeError> {
    let settings = [
        ResearchAdsrConfig::new(0.001, 0.070, 0.55, 0.100).expect("fixed sweep setting is valid"),
        retained_punch_config(),
        ResearchAdsrConfig::new(0.005, 0.160, 0.75, 0.250).expect("fixed sweep setting is valid"),
    ];
    let mut rows = Vec::with_capacity(settings.len());
    let mut selected = None;
    let mut best_score = f64::INFINITY;
    for (index, config) in settings.into_iter().enumerate() {
        let metrics = measure_isolated_punch(candidate, sample_rate, config)?;
        let bounded = metrics.finite
            && metrics.peak <= HOT_PEAK_CEILING
            && metrics.maximum_jump <= 0.1
            && metrics.release_continuity_jump <= 0.1
            && metrics.onset_to_sustain_ratio >= 1.05;
        let score = 400.0 * f64::from((config.attack_seconds - 0.003).abs())
            + 4.0 * f64::from((config.decay_seconds - 0.110).abs())
            + 2.0 * f64::from((config.sustain_level - 0.55).abs())
            + 2.0 * f64::from((config.release_seconds - 0.180).abs())
            + (1.0 - metrics.onset_to_sustain_ratio).max(0.0);
        if bounded && score < best_score {
            best_score = score;
            selected = Some(index);
        }
        rows.push(PunchSweepRow {
            config,
            metrics,
            retained: false,
            status: if bounded {
                "bounded_not_selected"
            } else {
                "rejected_engineering_bound"
            },
        });
    }
    if let Some(index) = selected {
        rows[index].retained = true;
        rows[index].status = "retained_engineering_candidate";
    }
    Ok(rows)
}

fn measure_isolated_punch(
    candidate: CompositeCandidate,
    sample_rate: u32,
    config: ResearchAdsrConfig,
) -> Result<PunchMetrics, CompositeError> {
    if sample_rate == 0 {
        return Err(CompositeError::InvalidSampleRate);
    }
    let family = candidate_layer_specs(candidate)
        .first()
        .map(|spec| spec.family)
        .unwrap_or(HybridFamily::CrossCoupledMachine);
    let mut voice = HybridVoice::new(
        family,
        sample_rate as f32,
        midi_frequency(26),
        0xadd5_0026 ^ candidate as u32,
    )
    .map_err(HybridRenderError::from)?;
    let mut envelope =
        ResearchAdsr::new(sample_rate, config).map_err(|_| CompositeError::InvalidSampleRate)?;
    envelope.note_on();
    let frames = 2 * sample_rate as usize;
    let gate_frame = 3 * sample_rate as usize / 4;
    let gain = audition_gain_policy(candidate, AuditionKind::Punch).linear * 0.18;
    let mut samples = Vec::with_capacity(2 * frames);
    for frame_index in 0..frames {
        if frame_index == gate_frame {
            envelope.note_off();
        }
        let env = envelope.sample();
        let frame = voice.sample();
        samples.push(gain * env * (0.86 * frame.left + 0.14 * frame.right));
        samples.push(gain * env * (0.14 * frame.left + 0.86 * frame.right));
    }
    let metrics = measure_composite(&samples, sample_rate as f32);
    let windows_ms = [10_u32, 50, 100, 250];
    let mut window_rms_values = [0.0; 4];
    let mut window_peak_values = [0.0; 4];
    for (index, milliseconds) in windows_ms.into_iter().enumerate() {
        let end = ((milliseconds as usize * sample_rate as usize) / 1_000).max(1);
        let window = &samples[..2 * end.min(frames)];
        window_rms_values[index] =
            window_rms(window, sample_rate, 0.0, f64::from(milliseconds) / 1_000.0);
        window_peak_values[index] = window
            .iter()
            .map(|sample| f64::from(sample.abs()))
            .fold(0.0, f64::max);
    }
    let sustain_start = sample_rate as usize / 4;
    let sustain_end = 3 * sample_rate as usize / 5;
    let sustain_rms = window_rms(
        &samples,
        sample_rate,
        sustain_start as f64 / f64::from(sample_rate),
        sustain_end as f64 / f64::from(sample_rate),
    );
    let onset_to_sustain_ratio = window_rms_values[2] / sustain_rms.max(1.0e-12);
    let transient_end = (sample_rate as usize / 4).min(frames);
    let transient = measure_composite(&samples[..2 * transient_end], sample_rate as f32);
    let early_start = sample_rate as usize / 20;
    let early_end = sample_rate as usize / 10;
    let early_fundamental = measure_projection(
        &samples[2 * early_start..2 * early_end],
        sample_rate as f32,
        midi_frequency(26),
    );
    let sustain_fundamental = measure_projection(
        &samples[2 * sustain_start..2 * sustain_end],
        sample_rate as f32,
        midi_frequency(26),
    );
    let before_release = &samples[2 * (gate_frame - 1)..2 * gate_frame];
    let after_release = &samples[2 * gate_frame..2 * (gate_frame + 1)];
    let release_continuity_jump = before_release
        .iter()
        .zip(after_release)
        .map(|(before, after)| f64::from((*after - *before).abs()))
        .fold(0.0, f64::max);
    Ok(PunchMetrics {
        window_rms: window_rms_values,
        window_peak: window_peak_values,
        onset_to_sustain_ratio,
        attack_time_ms: 1_000.0 * f64::from(config.attack_seconds),
        decay_settling_ms: 1_000.0 * f64::from(config.attack_seconds + config.decay_seconds),
        maximum_jump: metrics.maximum_jump,
        low_band_transient_rms: transient.low_rms,
        fundamental_decay_change_db: (early_fundamental - sustain_fundamental).abs(),
        release_continuity_jump,
        tail_duration_ms: 1_000.0 * f64::from(config.release_seconds),
        peak: metrics.peak,
        crest_factor: metrics.crest_factor,
        finite: metrics.finite,
    })
}

fn render_held_candidate_raw(
    candidate: CompositeCandidate,
    note: u8,
    sample_rate: u32,
) -> Result<Vec<f32>, CompositeError> {
    if sample_rate == 0 {
        return Err(CompositeError::InvalidSampleRate);
    }
    let specs = candidate_layer_specs(candidate);
    let frames = 4 * sample_rate as usize;
    let mut voices = Vec::with_capacity(specs.len());
    for (index, spec) in specs.iter().enumerate() {
        voices.push(
            HybridVoice::new(
                spec.family,
                sample_rate as f32,
                midi_frequency(note),
                0x0c7a_0000 ^ u32::from(note) ^ (index as u32 * 0x9e37),
            )
            .map_err(HybridRenderError::from)?,
        );
    }
    let mut samples = Vec::with_capacity(2 * frames);
    let mut follower_state = 0.0_f32;
    let mut junction_dc_state = [0.0_f32; 2];
    let junction_dc_alpha = 1.0 - (-std::f32::consts::TAU * 8.0 / sample_rate as f32).exp();
    for frame_index in 0..frames {
        let fade_frames = (0.02 * sample_rate as f32).round() as usize;
        let fade = (frame_index as f32 / fade_frames.max(1) as f32)
            .min(1.0)
            .min(((frames - 1 - frame_index) as f32 / fade_frames.max(1) as f32).min(1.0));
        let mut raw = [HybridFrame::default(); MAX_LAYERS];
        for (index, voice) in voices.iter_mut().enumerate() {
            raw[index] = voice.sample();
        }
        let follower_control = if candidate == CompositeCandidate::CrossTopologyFollower {
            let measurement = (0.5 * (raw[0].left + raw[0].right)).abs();
            let coefficient = if measurement > follower_state {
                0.012
            } else {
                0.0007
            };
            follower_state += coefficient * (measurement - follower_state);
            (0.75 + 7.0 * follower_state).clamp(0.75, 1.25)
        } else {
            1.0
        };
        let mut output = HybridFrame::default();
        for (index, spec) in specs.iter().enumerate() {
            let frame = raw[index];
            let matrix_left = spec.matrix[0][0] * frame.left + spec.matrix[0][1] * frame.right;
            let matrix_right = spec.matrix[1][0] * frame.left + spec.matrix[1][1] * frame.right;
            let gain = spec.mix_gain
                * if candidate == CompositeCandidate::CrossTopologyFollower && index == 1 {
                    follower_control
                } else {
                    1.0
                };
            if candidate == CompositeCandidate::CrossTopologyFollower && index == 2 {
                let mid = 0.5 * (matrix_left + matrix_right);
                let side = 0.5 * (matrix_left - matrix_right);
                let side_gain = 0.65 + 1.4 * (follower_control - 0.75);
                output.left += gain * (mid + side_gain * side);
                output.right += gain * (mid - side_gain * side);
            } else {
                output.left += gain * matrix_left;
                output.right += gain * matrix_right;
            }
        }
        if candidate == CompositeCandidate::RiskyNonlinearBraid {
            let a = 0.5 * (raw[0].left + raw[0].right);
            let b = 0.5 * (raw[1].left + raw[1].right);
            let c = 0.5 * (raw[2].left + raw[2].right);
            let center = 0.06 * (a * b + b * c + c * a);
            let side = 0.045 * (a * b - b * c);
            let junction = [center + side, center - side];
            for channel in 0..2 {
                junction_dc_state[channel] +=
                    junction_dc_alpha * (junction[channel] - junction_dc_state[channel]);
            }
            output.left += junction[0] - junction_dc_state[0];
            output.right += junction[1] - junction_dc_state[1];
        }
        samples.push(fade * output.left);
        samples.push(fade * output.right);
    }
    Ok(samples)
}

fn prepare_audition_render(
    raw_samples: Vec<f32>,
    sample_rate: u32,
    gain: OutputGainPolicy,
    active_start: f64,
    active_end: f64,
) -> AuditionRender {
    let raw_metrics = measure_composite(&raw_samples, sample_rate as f32);
    let samples: Vec<f32> = raw_samples
        .iter()
        .map(|sample| sample * gain.linear)
        .collect();
    let metrics = measure_composite(&samples, sample_rate as f32);
    AuditionRender {
        active_body_rms: window_rms(&samples, sample_rate, active_start, active_end),
        samples,
        metrics,
        raw_rms: raw_metrics.rms,
        raw_peak: raw_metrics.peak,
        raw_hash: raw_metrics.sample_hash,
        gain,
    }
}

pub fn window_rms(samples: &[f32], sample_rate: u32, start: f64, end: f64) -> f64 {
    if sample_rate == 0 || samples.len() % 2 != 0 || !start.is_finite() || end <= start {
        return 0.0;
    }
    let frames = samples.len() / 2;
    let start_frame = ((start * f64::from(sample_rate)).round() as usize).min(frames);
    let end_frame = ((end * f64::from(sample_rate)).round() as usize).min(frames);
    if start_frame >= end_frame {
        return 0.0;
    }
    let window = &samples[2 * start_frame..2 * end_frame];
    (window
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / window.len() as f64)
        .sqrt()
}

fn render_composite_with_mute(
    candidate: CompositeCandidate,
    sample_rate: u32,
    mono: bool,
    muted_layer: Option<usize>,
) -> Result<CompositeRender, CompositeError> {
    let machine = CompositeMachine::new_with_mute(candidate, sample_rate, muted_layer)?;
    Ok(render_prepared_machine(machine, mono))
}

fn render_prepared_machine(mut machine: CompositeMachine, mono: bool) -> CompositeRender {
    let sample_rate = machine.sample_rate;
    let mut samples = Vec::with_capacity(2 * machine.duration_frames());
    for _ in 0..machine.duration_frames() {
        let frame = machine.sample();
        if mono {
            let mid = 0.5 * (frame.left + frame.right);
            samples.push(mid);
            samples.push(mid);
        } else {
            samples.push(frame.left);
            samples.push(frame.right);
        }
    }
    let metrics = measure_composite(&samples, sample_rate as f32);
    CompositeRender { samples, metrics }
}

pub fn measure_composite(samples: &[f32], sample_rate: f32) -> CompositeMetrics {
    if samples.is_empty() || samples.len() % 2 != 0 || sample_rate <= 0.0 {
        return CompositeMetrics {
            peak: 0.0,
            rms: 0.0,
            dc: 0.0,
            crest_factor: 0.0,
            maximum_jump: 0.0,
            low_rms: 0.0,
            mid_rms: 0.0,
            high_rms: 0.0,
            brightness_ratio: 0.0,
            spectral_flux: 0.0,
            correlation: 0.0,
            side_to_mid: 0.0,
            low_side_to_mid: 0.0,
            mono_rms: 0.0,
            mono_to_stereo: 0.0,
            difference_rms: 0.0,
            onset_peak_ms: 0.0,
            early_rms: 0.0,
            body_rms: 0.0,
            sample_hash: 0,
            finite: false,
        };
    }
    let frames = samples.len() / 2;
    let low_alpha = 1.0 - (-std::f64::consts::TAU * 160.0 / f64::from(sample_rate)).exp();
    let mid_alpha = 1.0 - (-std::f64::consts::TAU * 2_500.0 / f64::from(sample_rate)).exp();
    let mut peak = 0.0_f64;
    let mut energy = 0.0_f64;
    let mut dc = 0.0_f64;
    let mut maximum_jump = 0.0_f64;
    let mut previous = [0.0_f64; 2];
    let mut left_energy = 0.0_f64;
    let mut right_energy = 0.0_f64;
    let mut cross = 0.0_f64;
    let mut mid_energy = 0.0_f64;
    let mut side_energy = 0.0_f64;
    let mut difference_energy = 0.0_f64;
    let mut low_state = 0.0_f64;
    let mut mid_state = 0.0_f64;
    let mut low_side_state = 0.0_f64;
    let mut low_mid_energy = 0.0_f64;
    let mut low_side_energy = 0.0_f64;
    let mut bands = [0.0_f64; 3];
    let mut finite = true;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let window_frames = (sample_rate as usize / 2).max(1);
    let mut window_bands = [0.0_f64; 3];
    let mut previous_window: Option<[f64; 3]> = None;
    let mut flux_sum = 0.0;
    let mut flux_windows = 0;
    let early_frames = (0.005 * sample_rate).round() as usize;
    let onset_frames = (0.010 * sample_rate).round() as usize;
    let body_start = (0.020 * sample_rate).round() as usize;
    let body_end = ((0.040 * sample_rate).round() as usize).min(frames);
    let mut early_energy = 0.0;
    let mut early_count = 0;
    let mut body_energy = 0.0;
    let mut body_count = 0;
    let mut onset_peak = 0.0;
    let mut onset_index = 0;

    for (index, frame) in samples.chunks_exact(2).enumerate() {
        let left = f64::from(frame[0]);
        let right = f64::from(frame[1]);
        finite &= left.is_finite() && right.is_finite();
        peak = peak.max(left.abs()).max(right.abs());
        energy += left * left + right * right;
        left_energy += left * left;
        right_energy += right * right;
        cross += left * right;
        maximum_jump = maximum_jump
            .max((left - previous[0]).abs())
            .max((right - previous[1]).abs());
        previous = [left, right];
        let mid = 0.5 * (left + right);
        let side = 0.5 * (left - right);
        dc += mid;
        mid_energy += mid * mid;
        side_energy += side * side;
        difference_energy += (left - right) * (left - right);
        low_state += low_alpha * (mid - low_state);
        mid_state += mid_alpha * (mid - mid_state);
        low_side_state += low_alpha * (side - low_side_state);
        let band_values = [low_state, mid_state - low_state, mid - mid_state];
        for band in 0..3 {
            let value = band_values[band];
            bands[band] += value * value;
            window_bands[band] += value * value;
        }
        low_mid_energy += low_state * low_state;
        low_side_energy += low_side_state * low_side_state;
        if index < early_frames {
            early_energy += mid * mid;
            early_count += 1;
        }
        if (body_start..body_end).contains(&index) {
            body_energy += mid * mid;
            body_count += 1;
        }
        if index < onset_frames && mid.abs() > onset_peak {
            onset_peak = mid.abs();
            onset_index = index;
        }
        if (index + 1) % window_frames == 0 || index + 1 == frames {
            let count = if (index + 1) % window_frames == 0 {
                window_frames
            } else {
                (index + 1) % window_frames
            }
            .max(1);
            let current = window_bands.map(|value| (value / count as f64).sqrt());
            if let Some(previous) = previous_window {
                let delta = current
                    .iter()
                    .zip(previous)
                    .map(|(current, previous)| {
                        let scale = current.max(previous).max(1.0e-9);
                        ((current - previous) / scale).powi(2)
                    })
                    .sum::<f64>()
                    .sqrt();
                flux_sum += delta;
                flux_windows += 1;
            }
            previous_window = Some(current);
            window_bands = [0.0; 3];
        }
        for sample in frame {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let rms = (energy / samples.len() as f64).sqrt();
    let band_rms = bands.map(|value| (value / frames as f64).sqrt());
    let mono_rms = (mid_energy / frames as f64).sqrt();
    CompositeMetrics {
        peak,
        rms,
        dc: dc / frames as f64,
        crest_factor: if rms > 0.0 { peak / rms } else { 0.0 },
        maximum_jump,
        low_rms: band_rms[0],
        mid_rms: band_rms[1],
        high_rms: band_rms[2],
        brightness_ratio: band_rms[2] / band_rms[0].max(1.0e-12),
        spectral_flux: flux_sum / flux_windows.max(1) as f64,
        correlation: if left_energy > 0.0 && right_energy > 0.0 {
            cross / (left_energy * right_energy).sqrt()
        } else {
            0.0
        },
        side_to_mid: side_energy / mid_energy.max(1.0e-24),
        low_side_to_mid: low_side_energy / low_mid_energy.max(1.0e-24),
        mono_rms,
        mono_to_stereo: mono_rms / rms.max(1.0e-24),
        difference_rms: (difference_energy / frames as f64).sqrt(),
        onset_peak_ms: 1_000.0 * onset_index as f64 / f64::from(sample_rate),
        early_rms: (early_energy / early_count.max(1) as f64).sqrt(),
        body_rms: (body_energy / body_count.max(1) as f64).sqrt(),
        sample_hash: hash,
        finite,
    }
}

pub fn measure_ablations(
    candidate: CompositeCandidate,
    sample_rate: u32,
) -> Result<Vec<AblationRow>, CompositeError> {
    let prepared = CompositeMachine::new(candidate, sample_rate)?;
    let layer_info = prepared.layer_info();
    let base = render_prepared_machine(prepared.clone(), false);
    let base_rms = base.metrics.rms.max(1.0e-24);
    let mut rows = Vec::with_capacity(layer_info.len());
    for layer in layer_info {
        let mut muted_machine = prepared.clone();
        muted_machine.muted_layer = Some(layer.index);
        let muted = render_prepared_machine(muted_machine, false);
        let difference_energy = base
            .samples
            .iter()
            .zip(&muted.samples)
            .map(|(base, muted)| {
                let difference = f64::from(*base) - f64::from(*muted);
                difference * difference
            })
            .sum::<f64>();
        let difference_rms = (difference_energy / base.samples.len().max(1) as f64).sqrt();
        rows.push(AblationRow {
            layer_index: layer.index,
            label: layer.label,
            difference_db: amplitude_db(difference_rms / base_rms),
        });
    }
    Ok(rows)
}

pub fn candidate_similarity(
    first: CompositeCandidate,
    second: CompositeCandidate,
    sample_rate: u32,
) -> Result<f64, CompositeError> {
    let first = render_composite(first, sample_rate, false)?;
    let second = render_composite(second, sample_rate, false)?;
    Ok(sample_similarity(&first.samples, &second.samples))
}

pub fn sample_similarity(first: &[f32], second: &[f32]) -> f64 {
    let frames = (first.len().min(second.len())) / 2;
    let mut dot = 0.0;
    let mut first_energy = 0.0;
    let mut second_energy = 0.0;
    for index in 0..frames {
        let first_mid = f64::from(0.5 * (first[2 * index] + first[2 * index + 1]));
        let second_mid = f64::from(0.5 * (second[2 * index] + second[2 * index + 1]));
        dot += first_mid * second_mid;
        first_energy += first_mid * first_mid;
        second_energy += second_mid * second_mid;
    }
    dot / (first_energy * second_energy).sqrt().max(1.0e-24)
}

pub fn measure_composite_residual(
    candidate: CompositeCandidate,
    sample_rate: u32,
    sample_count: usize,
) -> Result<f64, CompositeError> {
    const FACTOR: usize = 8;
    let mut target = CompositeMachine::new(candidate, sample_rate)?;
    let mut reference = CompositeMachine::new(candidate, sample_rate * FACTOR as u32)?;
    let mut target_samples = Vec::with_capacity(sample_count);
    let mut reference_samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let target_frame = target.sample();
        target_samples.push(0.5 * (target_frame.left + target_frame.right));
        let mut reference_sum = 0.0;
        for _ in 0..FACTOR {
            let frame = reference.sample();
            reference_sum += 0.5 * (frame.left + frame.right);
        }
        reference_samples.push(reference_sum / FACTOR as f32);
    }
    Ok(fitted_residual_db(&target_samples, &reference_samples))
}

pub fn measure_junction_residual(
    candidate: CompositeCandidate,
    sample_rate: u32,
    sample_count: usize,
) -> Result<f64, CompositeError> {
    const FACTOR: usize = 8;
    let mut target = CompositeMachine::new(candidate, sample_rate)?;
    let mut reference = CompositeMachine::new(candidate, sample_rate * FACTOR as u32)?;
    let mut target_samples = Vec::with_capacity(sample_count);
    let mut reference_samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        target.sample();
        target_samples.push(target.last_junction());
        let mut reference_sum = 0.0;
        for _ in 0..FACTOR {
            reference.sample();
            reference_sum += reference.last_junction();
        }
        reference_samples.push(reference_sum / FACTOR as f32);
    }
    Ok(fitted_residual_db(&target_samples, &reference_samples))
}

pub fn measure_junction_product_levels(
    candidate: CompositeCandidate,
    sample_rate: u32,
) -> Result<[f64; 3], CompositeError> {
    let mut with_junction = CompositeMachine::new(candidate, sample_rate)?;
    let mut linear_control = with_junction.clone();
    linear_control.junction_enabled = false;
    let frames = (4 * sample_rate as usize)
        .min(with_junction.duration_frames())
        .min(linear_control.duration_frames());
    let mut difference = Vec::with_capacity(2 * frames);
    for _ in 0..frames {
        let nonlinear = with_junction.sample();
        let linear = linear_control.sample();
        difference.push(nonlinear.left - linear.left);
        difference.push(nonlinear.right - linear.right);
    }
    Ok(measure_product_levels(
        &difference,
        sample_rate as f32,
        [50, 53, 57],
    ))
}

pub fn measure_product_levels(samples: &[f32], sample_rate: f32, notes: [u8; 3]) -> [f64; 3] {
    let frequencies = notes.map(midi_frequency);
    crate::hybrid::measure_frequency_levels(
        samples,
        sample_rate,
        [
            (frequencies[1] - frequencies[0]).abs(),
            (frequencies[2] - frequencies[1]).abs(),
            (frequencies[2] - frequencies[0]).abs(),
        ],
    )
}

pub fn measure_projection(samples: &[f32], sample_rate: f32, frequency: f32) -> f64 {
    crate::hybrid::measure_frequency_levels(samples, sample_rate, [frequency; 3])[0]
}

fn fitted_residual_db(target: &[f32], reference: &[f32]) -> f64 {
    let target_dc =
        target.iter().map(|sample| f64::from(*sample)).sum::<f64>() / target.len().max(1) as f64;
    let reference_dc = reference
        .iter()
        .map(|sample| f64::from(*sample))
        .sum::<f64>()
        / reference.len().max(1) as f64;
    let mut dot = 0.0;
    let mut reference_energy = 0.0;
    let mut target_energy = 0.0;
    for (&target, &reference) in target.iter().zip(reference) {
        let target = f64::from(target) - target_dc;
        let reference = f64::from(reference) - reference_dc;
        dot += target * reference;
        reference_energy += reference * reference;
        target_energy += target * target;
    }
    let gain = dot / reference_energy.max(1.0e-24);
    let residual_energy = target
        .iter()
        .zip(reference)
        .map(|(&target, &reference)| {
            let difference =
                (f64::from(target) - target_dc) - gain * (f64::from(reference) - reference_dc);
            difference * difference
        })
        .sum::<f64>();
    10.0 * (residual_energy / target_energy.max(1.0e-24))
        .max(1.0e-24)
        .log10()
}

fn amplitude_db(amplitude: f64) -> f64 {
    20.0 * amplitude.max(1.0e-12).log10()
}

pub const fn delayed_launch_offsets() -> [u32; 12] {
    DELAYED_LAUNCH_MS
}

pub fn midi_frequency(note: u8) -> f32 {
    crate::hybrid::midi_frequency(note)
}

#[cfg(test)]
mod tests {
    use super::{
        AuditionKind, CompositeCandidate, CompositeMachine, HOT_PEAK_CEILING, OriginalLayer,
        ResearchAdsr, ResearchAdsrConfig, audition_gain_policy, delayed_launch_offsets,
        gain_policy, midi_frequency, punch_sweep, render_bass_candidate, render_composite,
        render_held_candidate, render_hot_composite, render_original_layer, retained_punch_config,
    };
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn boundary_reconstructs_synchronized_and_delayed_inventories() {
        assert!((midi_frequency(38) - 73.416_19).abs() < 1.0e-4);
        assert!((midi_frequency(50) - 146.832_38).abs() < 1.0e-4);
        assert!((midi_frequency(53) - 174.614_12).abs() < 1.0e-4);
        assert!((midi_frequency(57) - 220.0).abs() < 1.0e-6);

        let synchronized =
            CompositeMachine::new(CompositeCandidate::SynchronizedReference, 48_000).unwrap();
        let first = synchronized.inventory_at_seconds(2.0);
        assert_eq!(first.total_voices, 30);
        assert_eq!(first.note_count(38), 3);
        assert_eq!(first.note_count(50), 9);
        assert_eq!(first.note_count(53), 9);
        assert_eq!(first.note_count(57), 9);

        let offsets = delayed_launch_offsets();
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[11], 4_110);
        assert!(offsets.windows(2).all(|pair| pair[1] > pair[0]));
        let delayed =
            CompositeMachine::new(CompositeCandidate::DelayedLaunchEstimate, 48_000).unwrap();
        assert!(delayed.inventory_at_seconds(2.0).total_voices < 30);
        assert!(delayed.duration_seconds() > 20.0);
    }

    #[test]
    fn original_layer_inventory_is_exact_and_stable() {
        assert_eq!(OriginalLayer::ALL.len(), 12);
        assert_eq!(
            OriginalLayer::ALL.map(OriginalLayer::label),
            [
                "cross-single",
                "cross-chord",
                "cross-progression",
                "cross-progression-mono",
                "spectral-single",
                "spectral-chord",
                "spectral-progression",
                "spectral-progression-mono",
                "dual-single",
                "dual-chord",
                "dual-progression",
                "dual-progression-mono",
            ]
        );
        assert_eq!(
            OriginalLayer::ALL.map(OriginalLayer::delayed_start_ms),
            delayed_launch_offsets()
        );
    }

    #[test]
    fn public_original_layer_render_preserves_reference_hashes() {
        let synchronized =
            render_composite(CompositeCandidate::SynchronizedReference, 48_000, false).unwrap();
        let delayed =
            render_composite(CompositeCandidate::DelayedLaunchEstimate, 48_000, false).unwrap();
        assert_eq!(synchronized.metrics.sample_hash, 0xbe3e_a0fd_d66b_4472);
        assert_eq!(delayed.metrics.sample_hash, 0xc919_cb57_920c_520f);
        let layer = render_original_layer(OriginalLayer::CrossSingle, 8_000).unwrap();
        assert!(!layer.is_empty());
        assert!(layer.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn boundary_is_deterministic_finite_bounded_and_allocation_free() {
        assert!(CompositeMachine::new(CompositeCandidate::SynchronizedReference, 0,).is_err());
        let mut first =
            CompositeMachine::new(CompositeCandidate::DelayedLaunchEstimate, 8_000).unwrap();
        let mut second =
            CompositeMachine::new(CompositeCandidate::DelayedLaunchEstimate, 8_000).unwrap();
        for _ in 0..24_000 {
            let a = assert_no_alloc(|| first.sample());
            let b = second.sample();
            assert_eq!(a, b);
            assert!(a.left.is_finite() && a.right.is_finite());
            assert!(a.left.abs() <= 0.8 && a.right.abs() <= 0.8);
        }
        first.reset();
        let mut fresh =
            CompositeMachine::new(CompositeCandidate::DelayedLaunchEstimate, 8_000).unwrap();
        assert_eq!(first.sample(), fresh.sample());
    }

    #[test]
    fn topologies_are_heterogeneous_distinct_stereo_and_mono_safe() {
        let mut hashes = Vec::new();
        for candidate in CompositeCandidate::FINAL_CANDIDATES {
            let mut machine = CompositeMachine::new(candidate, 8_000).unwrap();
            assert!(machine.layer_count() >= 3);
            assert!(machine.family_count() >= 2);
            assert!(!machine.topology_signature().is_empty());
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            let mut mid_energy = 0.0_f64;
            let mut side_energy = 0.0_f64;
            for _ in 0..32_000 {
                let frame = assert_no_alloc(|| machine.sample());
                assert!(frame.left.is_finite() && frame.right.is_finite());
                assert!(frame.left.abs() <= 0.8 && frame.right.abs() <= 0.8);
                let mid = f64::from(0.5 * (frame.left + frame.right));
                let side = f64::from(0.5 * (frame.left - frame.right));
                mid_energy += mid * mid;
                side_energy += side * side;
                hash ^= u64::from(frame.left.to_bits());
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                hash ^= u64::from(frame.right.to_bits());
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            assert!(mid_energy > 0.01, "candidate={candidate:?}");
            assert!(
                side_energy / mid_energy > 0.005,
                "candidate={candidate:?} side/mid={}",
                side_energy / mid_energy
            );
            assert!(
                (mid_energy / 32_000.0).sqrt()
                    >= 0.5 * ((mid_energy + side_energy) / 32_000.0).sqrt(),
                "candidate={candidate:?}"
            );
            hashes.push(hash);
        }
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), CompositeCandidate::FINAL_CANDIDATES.len());
    }

    #[test]
    fn follower_and_risky_junction_are_active_and_bounded() {
        let mut follower =
            CompositeMachine::new(CompositeCandidate::CrossTopologyFollower, 8_000).unwrap();
        let mut follower_min = f32::INFINITY;
        let mut follower_max = f32::NEG_INFINITY;
        for _ in 0..64_000 {
            follower.sample();
            follower_min = follower_min.min(follower.last_follower_control());
            follower_max = follower_max.max(follower.last_follower_control());
        }
        assert!(follower_max - follower_min >= 0.03);

        let mut risky =
            CompositeMachine::new(CompositeCandidate::RiskyNonlinearBraid, 8_000).unwrap();
        let mut junction_peak = 0.0_f32;
        for _ in 0..32_000 {
            risky.sample();
            junction_peak = junction_peak.max(risky.last_junction().abs());
        }
        assert!(junction_peak > 1.0e-5);
        assert!(junction_peak < 0.08);
    }

    #[test]
    fn analysis_is_deterministic_segmented_and_causally_diagnostic() {
        let first = super::render_composite(CompositeCandidate::RoleSeparatedMachine, 8_000, false)
            .unwrap();
        let second =
            super::render_composite(CompositeCandidate::RoleSeparatedMachine, 8_000, false)
                .unwrap();
        assert_eq!(first.samples, second.samples);
        assert_eq!(first.metrics, second.metrics);
        assert!(first.metrics.finite);
        assert!(first.metrics.peak <= 0.8);
        assert!(first.metrics.low_rms > 0.0);
        assert!(first.metrics.mid_rms > 0.0);
        assert!(first.metrics.high_rms > 0.0);
        assert!(first.metrics.spectral_flux > 0.0);
        assert!(first.metrics.low_side_to_mid.is_finite());

        let ablations =
            super::measure_ablations(CompositeCandidate::RoleSeparatedMachine, 4_000).unwrap();
        assert_eq!(ablations.len(), 3);
        assert!(ablations.iter().all(|row| row.difference_db.is_finite()));
        assert!(ablations.iter().all(|row| row.difference_db > -42.0));

        let similarity = super::candidate_similarity(
            CompositeCandidate::RoleSeparatedMachine,
            CompositeCandidate::HarmonicLattice,
            4_000,
        )
        .unwrap();
        assert!(similarity.is_finite());
        assert!(similarity.abs() < 0.985);
    }

    #[test]
    fn high_rate_and_nonlinear_product_evidence_is_finite() {
        let residual =
            super::measure_composite_residual(CompositeCandidate::RiskyNonlinearBraid, 2_000, 512)
                .unwrap();
        assert!(residual.is_finite());
        assert!(residual <= 3.0);

        let render =
            super::render_composite(CompositeCandidate::RiskyNonlinearBraid, 8_000, false).unwrap();
        let products =
            super::measure_product_levels(&render.samples[..8_000 * 2], 8_000.0, [50, 53, 57]);
        assert!(products.iter().all(|level| level.is_finite()));
    }

    #[test]
    fn rejected_risky_revision_is_replaced_by_a_distinct_pairwise_exchange() {
        let similarity = super::candidate_similarity(
            CompositeCandidate::RoleSeparatedMachine,
            CompositeCandidate::RiskyNonlinearBraid,
            4_000,
        )
        .unwrap();
        assert!(similarity.abs() < 0.90, "similarity={similarity}");
        let residual =
            super::measure_junction_residual(CompositeCandidate::RiskyNonlinearBraid, 2_000, 512)
                .unwrap();
        assert!(residual.is_finite());
        assert!(residual <= 3.0);
    }

    #[test]
    fn risky_junction_products_stay_below_the_dry_chord_targets() {
        let render =
            super::render_composite(CompositeCandidate::RiskyNonlinearBraid, 8_000, false).unwrap();
        let first_segment = &render.samples[..2 * 4 * 8_000];
        let targets = [50, 53, 57]
            .map(|note| super::measure_projection(first_segment, 8_000.0, midi_frequency(note)));
        let weakest_target = targets.iter().copied().fold(f64::INFINITY, f64::min);
        let products =
            super::measure_junction_product_levels(CompositeCandidate::RiskyNonlinearBraid, 8_000)
                .unwrap();
        let strongest_product = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            strongest_product <= weakest_target - 30.0,
            "products={products:?} targets={targets:?}"
        );
    }

    #[test]
    #[ignore = "development-only exhaustive historical composite render"]
    fn hot_output_contract_uses_explicit_gains_without_changing_raw_references() {
        let expected_raw_hashes = [
            (
                CompositeCandidate::SynchronizedReference,
                0xbe3e_a0fd_d66b_4472,
            ),
            (
                CompositeCandidate::DelayedLaunchEstimate,
                0xc919_cb57_920c_520f,
            ),
        ];
        for (candidate, expected_hash) in expected_raw_hashes {
            let raw = super::render_composite(candidate, 48_000, false).unwrap();
            assert_eq!(raw.metrics.sample_hash, expected_hash);
            let policy = gain_policy(candidate);
            assert!(policy.linear.is_finite() && policy.linear > 0.0);
            let hot = render_hot_composite(candidate, 48_000).unwrap();
            assert!(hot.metrics.finite);
            assert!(hot.metrics.peak <= 0.75, "{candidate:?}: {:?}", hot.metrics);
            assert!(
                (0.10..=0.14).contains(&hot.active_body_rms),
                "{candidate:?}: {}",
                hot.active_body_rms
            );
            assert_eq!(raw.metrics.sample_hash, hot.raw_hash);
            assert!(
                raw.samples
                    .iter()
                    .zip(&hot.samples)
                    .all(|(raw, hot)| *hot == *raw * policy.linear),
                "candidate={candidate:?} contains an undeclared final processor"
            );
        }
    }

    #[test]
    fn research_adsr_is_sample_accurate_deterministic_bounded_and_allocation_free() {
        let config = ResearchAdsrConfig::new(0.003, 0.100, 0.65, 0.180).unwrap();
        let mut first = ResearchAdsr::new(1_000, config).unwrap();
        let mut second = ResearchAdsr::new(1_000, config).unwrap();
        first.note_on();
        second.note_on();
        let mut values = Vec::new();
        for _ in 0..150 {
            let a = assert_no_alloc(|| first.sample());
            let b = second.sample();
            assert_eq!(a, b);
            assert!(a.is_finite() && (0.0..=1.0).contains(&a));
            values.push(a);
        }
        assert_eq!(values[2], 1.0);
        assert!((values[102] - 0.65).abs() < 1.0e-6);

        first.note_off();
        let before = first.level();
        let released = assert_no_alloc(|| first.sample());
        assert!(released <= before);
        assert!(before - released <= before / 180.0 + 1.0e-6);
        for _ in 1..180 {
            first.sample();
        }
        assert_eq!(first.level(), 0.0);

        first.note_on();
        assert!((first.sample() - 1.0 / 3.0).abs() < 1.0e-6);
        first.note_on();
        assert!((first.sample() - 1.0 / 3.0).abs() < 1.0e-6);
    }

    #[test]
    fn octave_renders_keep_one_candidate_gain_across_d1_d2_d3() {
        let expected = [
            (26, 36.708_096_f32),
            (38, 73.416_19_f32),
            (50, 146.832_38_f32),
        ];
        for (note, frequency) in expected {
            assert!((midi_frequency(note) - frequency).abs() < 1.0e-4);
        }
        for candidate in CompositeCandidate::FINAL_CANDIDATES {
            let policy = audition_gain_policy(candidate, AuditionKind::HeldOctave);
            let mut hashes = Vec::new();
            for (note, _) in expected {
                let raw = super::render_held_candidate_raw(candidate, note, 48_000).unwrap();
                let render = render_held_candidate(candidate, note, 48_000).unwrap();
                assert_eq!(render.gain, policy);
                assert!(render.metrics.finite);
                assert!(render.metrics.peak <= HOT_PEAK_CEILING);
                assert!(render.raw_rms > 0.0);
                assert!(
                    (super::HOT_MINIMUM_RMS..=super::HOT_MAXIMUM_RMS)
                        .contains(&render.active_body_rms),
                    "candidate={candidate:?} note={note} active={}",
                    render.active_body_rms
                );
                assert!(
                    raw.iter()
                        .zip(&render.samples)
                        .all(|(raw, hot)| *hot == *raw * policy.linear),
                    "candidate={candidate:?} note={note} contains an undeclared final processor"
                );
                hashes.push(render.metrics.sample_hash);
            }
            hashes.sort_unstable();
            hashes.dedup();
            assert_eq!(hashes.len(), 3, "candidate={candidate:?}");
        }
    }

    #[test]
    fn octave_topologies_preserve_follower_and_nonlinear_identity() {
        for note in [26, 38, 50] {
            let role = render_held_candidate(CompositeCandidate::RoleSeparatedMachine, note, 8_000)
                .unwrap();
            let follower =
                render_held_candidate(CompositeCandidate::CrossTopologyFollower, note, 8_000)
                    .unwrap();
            let risky = render_held_candidate(CompositeCandidate::RiskyNonlinearBraid, note, 8_000)
                .unwrap();
            let role_follower = super::sample_similarity(&role.samples, &follower.samples).abs();
            let follower_risky = super::sample_similarity(&follower.samples, &risky.samples).abs();
            assert!(role_follower < 0.985);
            assert!(follower_risky < 0.99);
        }
    }

    #[test]
    #[ignore = "development-only exhaustive historical composite render"]
    fn d1_pedal_and_punch_are_finite_hot_and_deterministic() {
        let retained = retained_punch_config();
        assert_eq!(
            retained,
            ResearchAdsrConfig::new(0.003, 0.110, 0.55, 0.180).unwrap()
        );
        for candidate in CompositeCandidate::FINAL_CANDIDATES {
            let bass = render_bass_candidate(candidate, 48_000, false).unwrap();
            let punch = render_bass_candidate(candidate, 48_000, true).unwrap();
            let repeated = render_bass_candidate(candidate, 48_000, true).unwrap();
            assert_eq!(punch.samples, repeated.samples);
            for render in [&bass, &punch] {
                assert!(render.metrics.finite);
                assert!(render.metrics.peak <= HOT_PEAK_CEILING);
                assert!(
                    (super::HOT_MINIMUM_RMS..=super::HOT_MAXIMUM_RMS)
                        .contains(&render.active_body_rms),
                    "candidate={candidate:?} active={}",
                    render.active_body_rms
                );
            }
            assert_ne!(bass.metrics.sample_hash, punch.metrics.sample_hash);
        }
    }

    #[test]
    fn punch_envelope_changes_only_the_scheduled_source_before_sum() {
        let candidate = CompositeCandidate::RoleSeparatedMachine;
        let raw = super::render_composite(candidate, 8_000, false).unwrap();
        let punch =
            super::render_bass_candidate_raw(candidate, 8_000, Some(retained_punch_config()))
                .unwrap();
        let active_frame = 400;
        assert_ne!(
            &raw.samples[2 * active_frame..2 * active_frame + 2],
            &punch[2 * active_frame..2 * active_frame + 2]
        );
        let idle_frame = 2 * 8_000;
        assert_eq!(
            &raw.samples[2 * idle_frame..2 * idle_frame + 2],
            &punch[2 * idle_frame..2 * idle_frame + 2]
        );
    }

    #[test]
    fn punch_sweep_reports_required_transient_and_release_evidence() {
        let rows = punch_sweep(CompositeCandidate::RoleSeparatedMachine, 48_000).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().filter(|row| row.retained).count(), 1);
        let retained = rows.iter().find(|row| row.retained).unwrap();
        assert_eq!(retained.config, retained_punch_config());
        for row in rows {
            assert!(row.metrics.finite);
            assert!(row.metrics.maximum_jump.is_finite());
            assert!(row.metrics.onset_to_sustain_ratio.is_finite());
            assert!(row.metrics.low_band_transient_rms > 0.0);
            assert!(row.metrics.fundamental_decay_change_db.is_finite());
            assert!(row.metrics.release_continuity_jump.is_finite());
            assert!(row.metrics.tail_duration_ms > 0.0);
            assert!(
                row.metrics
                    .window_rms
                    .iter()
                    .chain(&row.metrics.window_peak)
                    .all(|value| value.is_finite())
            );
        }
    }
}
