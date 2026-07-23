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
            output.left += gain
                * (layer.spec.matrix[0][0] * frame.left + layer.spec.matrix[0][1] * frame.right);
            output.right += gain
                * (layer.spec.matrix[1][0] * frame.left + layer.spec.matrix[1][1] * frame.right);
        }
        self.last_junction = 0.0;
        if self.candidate == CompositeCandidate::RiskyNonlinearBraid {
            let first_mid = 0.5 * (raw[0].left + raw[0].right);
            let third_mid = 0.5 * (raw[2].left + raw[2].right);
            let junction = (2.4 * first_mid * third_mid).clamp(-0.06, 0.06);
            output.left += junction;
            output.right -= 0.32 * junction;
            self.last_junction = junction;
        }
        self.frame_index = self.frame_index.saturating_add(1);
        output
    }

    pub fn reset(&mut self) {
        self.frame_index = 0;
        self.follower_state = 0.0;
        self.last_follower_control = 0.75;
        self.last_junction = 0.0;
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
                "dry heterogeneous anchor+bounded multiplicative junction"
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
                Score::Drone,
                "cross-junction-source",
                0.22,
                MID_FOCUS_MATRIX,
            ),
            custom_spec(
                HybridFamily::SpectralShadow,
                Score::LongHeldChord,
                "spectral-dry-anchor",
                0.22,
                IDENTITY_MATRIX,
            ),
            custom_spec(
                HybridFamily::DualResonantBody,
                Score::Progression,
                "dual-junction-source",
                0.19,
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
    let mut specs = Vec::with_capacity(12);
    let mut index = 0;
    for family in HybridFamily::ALL {
        for (score, label) in [
            (Score::Single, "single"),
            (Score::HeldChord, "chord"),
            (Score::Progression, "progression"),
            (Score::ProgressionMono, "progression-mono"),
        ] {
            specs.push(LayerSpec {
                family,
                score,
                start_ms: if delayed { DELAYED_LAUNCH_MS[index] } else { 0 },
                label: original_label(family, score, label),
                mix_gain: ORIGINAL_COMPOSITE_GAIN,
                matrix: IDENTITY_MATRIX,
            });
            index += 1;
        }
    }
    specs
}

fn original_label(family: HybridFamily, score: Score, fallback: &'static str) -> &'static str {
    match (family, score) {
        (HybridFamily::CrossCoupledMachine, Score::Single) => "cross-single",
        (HybridFamily::CrossCoupledMachine, Score::HeldChord) => "cross-chord",
        (HybridFamily::CrossCoupledMachine, Score::Progression) => "cross-progression",
        (HybridFamily::CrossCoupledMachine, Score::ProgressionMono) => "cross-progression-mono",
        (HybridFamily::SpectralShadow, Score::Single) => "spectral-single",
        (HybridFamily::SpectralShadow, Score::HeldChord) => "spectral-chord",
        (HybridFamily::SpectralShadow, Score::Progression) => "spectral-progression",
        (HybridFamily::SpectralShadow, Score::ProgressionMono) => "spectral-progression-mono",
        (HybridFamily::DualResonantBody, Score::Single) => "dual-single",
        (HybridFamily::DualResonantBody, Score::HeldChord) => "dual-chord",
        (HybridFamily::DualResonantBody, Score::Progression) => "dual-progression",
        (HybridFamily::DualResonantBody, Score::ProgressionMono) => "dual-progression-mono",
        _ => fallback,
    }
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
    use super::{CompositeCandidate, CompositeMachine, delayed_launch_offsets, midi_frequency};
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
}
