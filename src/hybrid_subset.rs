use crate::composite_machine::{
    CompositeCandidate, CompositeError, OriginalLayer, original_composite_layer_gain,
    render_composite, render_original_layer,
};
use crate::dsp::hybrid::HybridFrame;
use crate::hybrid::{HybridCondition, HybridRenderError, PROGRESSION, measure_frequency_levels};
use std::f64::consts::TAU;
use std::sync::Arc;
use thiserror::Error;

pub const OUTPUT_CEILING: f32 = 0.966_050_86;
pub const MAX_CEILING_PROPORTION: f64 = 0.01;
pub const MIN_ACTIVE_RMS: f64 = 0.199_526_23;
pub const MAX_ACTIVE_RMS: f64 = 0.316_227_77;
pub const MAX_DC: f64 = 0.001;
pub const MAX_JUMP: f64 = 1.5;
pub const MAX_MONO_LOSS_DB: f64 = 1.5;
pub const MIN_CORRELATION: f64 = -0.25;
pub const MAX_HIGH_RATE_RESIDUAL_DB: f64 = -1.0;
pub const MAX_SPECTRAL_FLATNESS: f64 = 0.50;
pub const MIN_TONAL_PASS_FRACTION: f64 = 2.0 / 3.0;
pub const TARGET_MARGIN_DB: f64 = 36.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubsetCandidate {
    ThreeSingles,
    ThreeChords,
    ThreeStereoProgressions,
    ThreeMonoProgressions,
    CrossAnchor,
    SpectralAnchor,
    DualAnchor,
}

impl SubsetCandidate {
    pub const ALL: [Self; 7] = [
        Self::ThreeSingles,
        Self::ThreeChords,
        Self::ThreeStereoProgressions,
        Self::ThreeMonoProgressions,
        Self::CrossAnchor,
        Self::SpectralAnchor,
        Self::DualAnchor,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::ThreeSingles => "three-singles",
            Self::ThreeChords => "three-chords",
            Self::ThreeStereoProgressions => "three-stereo-progressions",
            Self::ThreeMonoProgressions => "three-mono-progressions",
            Self::CrossAnchor => "cross-anchor",
            Self::SpectralAnchor => "spectral-anchor",
            Self::DualAnchor => "dual-anchor",
        }
    }

    pub const fn layers(self) -> &'static [OriginalLayer] {
        match self {
            Self::ThreeSingles => &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ],
            Self::ThreeChords => &[
                OriginalLayer::CrossChord,
                OriginalLayer::SpectralChord,
                OriginalLayer::DualChord,
            ],
            Self::ThreeStereoProgressions => &[
                OriginalLayer::CrossProgression,
                OriginalLayer::SpectralProgression,
                OriginalLayer::DualProgression,
            ],
            Self::ThreeMonoProgressions => &[
                OriginalLayer::CrossProgressionMono,
                OriginalLayer::SpectralProgressionMono,
                OriginalLayer::DualProgressionMono,
            ],
            Self::CrossAnchor => &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralChord,
                OriginalLayer::DualProgression,
                OriginalLayer::CrossProgressionMono,
            ],
            Self::SpectralAnchor => &[
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualChord,
                OriginalLayer::CrossProgression,
                OriginalLayer::SpectralProgressionMono,
            ],
            Self::DualAnchor => &[
                OriginalLayer::DualSingle,
                OriginalLayer::CrossChord,
                OriginalLayer::SpectralProgression,
                OriginalLayer::DualProgressionMono,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum SubsetError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error("gain must be finite and positive")]
    InvalidGain,
    #[error("no positive shared gain satisfies the sparse-ceiling rule")]
    NoSharedGain,
    #[error(transparent)]
    Hybrid(#[from] HybridRenderError),
    #[error(transparent)]
    Composite(#[from] CompositeError),
}

#[derive(Clone)]
struct PreparedLayer {
    samples: Arc<[f32]>,
    start_frame: usize,
}

pub struct HybridSubsetMixer {
    layers: Vec<PreparedLayer>,
    frame: usize,
    duration_frames: usize,
    gain: f32,
}

impl HybridSubsetMixer {
    pub fn new(
        candidate: SubsetCandidate,
        sample_rate: u32,
        gain: f32,
    ) -> Result<Self, SubsetError> {
        if sample_rate == 0 {
            return Err(SubsetError::InvalidSampleRate);
        }
        if !gain.is_finite() || gain <= 0.0 {
            return Err(SubsetError::InvalidGain);
        }
        let first_ms = candidate
            .layers()
            .iter()
            .map(|layer| layer.delayed_start_ms())
            .min()
            .unwrap_or(0);
        let mut layers = Vec::with_capacity(candidate.layers().len());
        let mut duration_frames = 0;
        for &layer in candidate.layers() {
            let samples: Arc<[f32]> = render_original_layer(layer, sample_rate)?.into();
            let start_frame = ((u64::from(layer.delayed_start_ms() - first_ms)
                * u64::from(sample_rate))
                / 1_000) as usize;
            duration_frames = duration_frames.max(start_frame + samples.len() / 2);
            layers.push(PreparedLayer {
                samples,
                start_frame,
            });
        }
        Ok(Self {
            layers,
            frame: 0,
            duration_frames,
            gain,
        })
    }

    #[inline]
    fn sample_raw(&mut self) -> HybridFrame {
        let mut output = HybridFrame::default();
        for layer in &self.layers {
            if let Some(local) = self.frame.checked_sub(layer.start_frame) {
                let offset = 2 * local;
                if offset + 1 < layer.samples.len() {
                    output.left += original_composite_layer_gain() * layer.samples[offset];
                    output.right += original_composite_layer_gain() * layer.samples[offset + 1];
                }
            }
        }
        self.frame = self.frame.saturating_add(1);
        output
    }

    #[inline]
    pub fn sample(&mut self) -> HybridFrame {
        let frame = self.sample_raw();
        HybridFrame {
            left: (frame.left * self.gain).clamp(-OUTPUT_CEILING, OUTPUT_CEILING),
            right: (frame.right * self.gain).clamp(-OUTPUT_CEILING, OUTPUT_CEILING),
        }
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubsetMetrics {
    pub peak: f64,
    pub active_rms: f64,
    pub dc: f64,
    pub maximum_jump: f64,
    pub correlation: f64,
    pub mono_loss_db: f64,
    pub ceiling_proportion: f64,
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawPreview {
    pub candidate: SubsetCandidate,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub active_start_frame: usize,
    pub active_end_frame: usize,
}

impl RawPreview {
    pub fn metrics_at_gain(&self, gain: f32) -> SubsetMetrics {
        let samples = apply_gain_and_ceiling(&self.samples, gain);
        measure_subset(&samples, self.active_start_frame, self.active_end_frame)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubsetRender {
    pub candidate: SubsetCandidate,
    pub samples: Vec<f32>,
    pub metrics: SubsetMetrics,
    pub gain: f32,
    pub active_start_frame: usize,
    pub active_end_frame: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceRender {
    pub samples: Vec<f32>,
    pub metrics: SubsetMetrics,
    pub gain: f32,
    pub raw_hash: u64,
    pub active_start_frame: usize,
    pub active_end_frame: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupGains {
    pub three_layer: f32,
    pub four_layer: f32,
}

impl GroupGains {
    pub fn for_candidate(self, candidate: SubsetCandidate) -> f32 {
        if candidate.layers().len() == 3 {
            self.three_layer
        } else {
            self.four_layer
        }
    }
}

pub fn rebased_offsets_ms(candidate: SubsetCandidate) -> Vec<u32> {
    let first = candidate
        .layers()
        .iter()
        .map(|layer| layer.delayed_start_ms())
        .min()
        .unwrap_or(0);
    candidate
        .layers()
        .iter()
        .map(|layer| layer.delayed_start_ms() - first)
        .collect()
}

pub fn preview_all(sample_rate: u32) -> Result<Vec<RawPreview>, SubsetError> {
    SubsetCandidate::ALL
        .into_iter()
        .map(|candidate| render_raw_candidate(candidate, sample_rate))
        .collect()
}

pub fn render_raw_candidate(
    candidate: SubsetCandidate,
    sample_rate: u32,
) -> Result<RawPreview, SubsetError> {
    let mut mixer = HybridSubsetMixer::new(candidate, sample_rate, 1.0)?;
    let mut samples = Vec::with_capacity(2 * mixer.duration_frames());
    for _ in 0..mixer.duration_frames() {
        let frame = mixer.sample_raw();
        samples.extend([frame.left, frame.right]);
    }
    let (active_start_frame, active_end_frame) = active_region(&samples);
    Ok(RawPreview {
        candidate,
        samples,
        sample_rate,
        active_start_frame,
        active_end_frame,
    })
}

pub fn render_candidate(preview: &RawPreview, gain: f32) -> Result<SubsetRender, SubsetError> {
    if !gain.is_finite() || gain <= 0.0 {
        return Err(SubsetError::InvalidGain);
    }
    let samples = apply_gain_and_ceiling(&preview.samples, gain);
    let metrics = measure_subset(
        &samples,
        preview.active_start_frame,
        preview.active_end_frame,
    );
    Ok(SubsetRender {
        candidate: preview.candidate,
        samples,
        metrics,
        gain,
        active_start_frame: preview.active_start_frame,
        active_end_frame: preview.active_end_frame,
    })
}

pub fn render_reference(sample_rate: u32) -> Result<ReferenceRender, SubsetError> {
    let raw = render_composite(
        CompositeCandidate::DelayedLaunchEstimate,
        sample_rate,
        false,
    )?;
    let raw_hash = raw.metrics.sample_hash;
    let (active_start_frame, active_end_frame) = active_region(&raw.samples);
    let preview = RawPreview {
        candidate: SubsetCandidate::ThreeSingles,
        samples: raw.samples,
        sample_rate,
        active_start_frame,
        active_end_frame,
    };
    let gain = select_shared_gain(std::iter::once(&preview))?;
    let samples = apply_gain_and_ceiling(&preview.samples, gain);
    let metrics = measure_subset(&samples, active_start_frame, active_end_frame);
    Ok(ReferenceRender {
        samples,
        metrics,
        gain,
        raw_hash,
        active_start_frame,
        active_end_frame,
    })
}

pub fn select_group_gains(previews: &[RawPreview]) -> Result<GroupGains, SubsetError> {
    Ok(GroupGains {
        three_layer: select_shared_gain(
            previews
                .iter()
                .filter(|preview| preview.candidate.layers().len() == 3),
        )?,
        four_layer: select_shared_gain(
            previews
                .iter()
                .filter(|preview| preview.candidate.layers().len() == 4),
        )?,
    })
}

fn select_shared_gain<'a>(
    previews: impl Iterator<Item = &'a RawPreview> + Clone,
) -> Result<f32, SubsetError> {
    let mut maximum = f32::INFINITY;
    let mut found = false;
    for preview in previews.clone() {
        let window = active_samples(
            &preview.samples,
            preview.active_start_frame,
            preview.active_end_frame,
        );
        let mut magnitudes: Vec<f32> = window.iter().map(|sample| sample.abs()).collect();
        magnitudes.sort_unstable_by(f32::total_cmp);
        let index = ((magnitudes.len() as f64 * (1.0 - MAX_CEILING_PROPORTION)).ceil() as usize)
            .saturating_sub(1)
            .min(magnitudes.len().saturating_sub(1));
        let percentile = magnitudes.get(index).copied().unwrap_or(0.0);
        if percentile > 0.0 {
            maximum = maximum.min(OUTPUT_CEILING / percentile);
            found = true;
        }
    }
    if !found {
        return Err(SubsetError::NoSharedGain);
    }
    let mut gain = (maximum * 4.0).floor() * 0.25;
    while gain > 0.0 {
        if previews.clone().all(|preview| {
            let metrics = preview.metrics_at_gain(gain);
            metrics.ceiling_proportion <= MAX_CEILING_PROPORTION + f64::EPSILON
                && metrics.active_rms <= MAX_ACTIVE_RMS
        }) {
            return Ok(gain);
        }
        gain -= 0.25;
    }
    Err(SubsetError::NoSharedGain)
}

fn apply_gain_and_ceiling(samples: &[f32], gain: f32) -> Vec<f32> {
    samples
        .iter()
        .map(|sample| (sample * gain).clamp(-OUTPUT_CEILING, OUTPUT_CEILING))
        .collect()
}

fn active_region(samples: &[f32]) -> (usize, usize) {
    let frames = samples.len() / 2;
    let start = samples
        .chunks_exact(2)
        .position(|frame| frame[0] != 0.0 || frame[1] != 0.0)
        .unwrap_or(0);
    let end = samples
        .chunks_exact(2)
        .rposition(|frame| frame[0] != 0.0 || frame[1] != 0.0)
        .map_or(frames, |index| index + 1);
    (start, end.max(start))
}

fn active_samples(samples: &[f32], start_frame: usize, end_frame: usize) -> &[f32] {
    let start = (2 * start_frame).min(samples.len());
    let end = (2 * end_frame).min(samples.len());
    &samples[start..end.max(start)]
}

pub fn measure_subset(
    samples: &[f32],
    active_start_frame: usize,
    active_end_frame: usize,
) -> SubsetMetrics {
    let active = active_samples(samples, active_start_frame, active_end_frame);
    if active.is_empty() {
        return SubsetMetrics {
            peak: 0.0,
            active_rms: 0.0,
            dc: 0.0,
            maximum_jump: 0.0,
            correlation: 0.0,
            mono_loss_db: 0.0,
            ceiling_proportion: 0.0,
            sample_hash: 0,
            finite: false,
        };
    }
    let mut peak = 0.0_f64;
    let mut energy = 0.0_f64;
    let mut dc = 0.0_f64;
    let mut maximum_jump = 0.0_f64;
    let mut previous = [0.0_f64; 2];
    let mut left_energy = 0.0_f64;
    let mut right_energy = 0.0_f64;
    let mut cross = 0.0_f64;
    let mut mono_energy = 0.0_f64;
    let mut ceiling_count = 0;
    let mut finite = true;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for frame in active.chunks_exact(2) {
        let left = f64::from(frame[0]);
        let right = f64::from(frame[1]);
        finite &= left.is_finite() && right.is_finite();
        peak = peak.max(left.abs()).max(right.abs());
        energy += left * left + right * right;
        dc += 0.5 * (left + right);
        maximum_jump = maximum_jump
            .max((left - previous[0]).abs())
            .max((right - previous[1]).abs());
        previous = [left, right];
        left_energy += left * left;
        right_energy += right * right;
        cross += left * right;
        let mono = 0.5 * (left + right);
        mono_energy += mono * mono;
        ceiling_count += usize::from(frame[0].abs() >= OUTPUT_CEILING - 1.0e-7);
        ceiling_count += usize::from(frame[1].abs() >= OUTPUT_CEILING - 1.0e-7);
        for sample in frame {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let frames = active.len() / 2;
    let rms = (energy / active.len() as f64).sqrt();
    let mono_rms = (mono_energy / frames as f64).sqrt();
    SubsetMetrics {
        peak,
        active_rms: rms,
        dc: dc / frames as f64,
        maximum_jump,
        correlation: cross / (left_energy * right_energy).sqrt().max(1.0e-24),
        mono_loss_db: (-20.0 * (mono_rms / rms.max(1.0e-24)).log10()).max(0.0),
        ceiling_proportion: ceiling_count as f64 / active.len() as f64,
        sample_hash: hash,
        finite,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectionReason {
    NonFinite,
    ExcessiveDc,
    ActiveRms,
    ExcessiveCeiling,
    MaximumJump,
    StereoMono,
    TonalInventory,
    NoiseLike,
    HighRateResidual,
    ReturnToZero,
}

impl RejectionReason {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::NonFinite => "non_finite",
            Self::ExcessiveDc => "excessive_dc",
            Self::ActiveRms => "active_rms",
            Self::ExcessiveCeiling => "excessive_ceiling",
            Self::MaximumJump => "maximum_jump",
            Self::StereoMono => "stereo_mono",
            Self::TonalInventory => "tonal_inventory",
            Self::NoiseLike => "noise_like",
            Self::HighRateResidual => "high_rate_residual",
            Self::ReturnToZero => "return_to_zero",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateEvidence {
    pub metrics: SubsetMetrics,
    pub tonal_pass_fraction: f64,
    pub spectral_flatness: f64,
    pub high_rate_residual_db: f64,
    pub returns_to_zero: bool,
}

impl CandidateEvidence {
    pub fn rejection_reasons(self) -> Vec<RejectionReason> {
        let mut reasons = Vec::new();
        if !self.metrics.finite {
            reasons.push(RejectionReason::NonFinite);
        }
        if self.metrics.dc.abs() > MAX_DC {
            reasons.push(RejectionReason::ExcessiveDc);
        }
        if !(MIN_ACTIVE_RMS..=MAX_ACTIVE_RMS).contains(&self.metrics.active_rms) {
            reasons.push(RejectionReason::ActiveRms);
        }
        if self.metrics.ceiling_proportion > MAX_CEILING_PROPORTION {
            reasons.push(RejectionReason::ExcessiveCeiling);
        }
        if self.metrics.maximum_jump > MAX_JUMP {
            reasons.push(RejectionReason::MaximumJump);
        }
        if self.metrics.mono_loss_db > MAX_MONO_LOSS_DB
            || self.metrics.correlation <= MIN_CORRELATION
        {
            reasons.push(RejectionReason::StereoMono);
        }
        if self.tonal_pass_fraction + f64::EPSILON < MIN_TONAL_PASS_FRACTION {
            reasons.push(RejectionReason::TonalInventory);
        }
        if classify_noise_like(self.spectral_flatness, self.tonal_pass_fraction) {
            reasons.push(RejectionReason::NoiseLike);
        }
        if self.high_rate_residual_db >= MAX_HIGH_RATE_RESIDUAL_DB {
            reasons.push(RejectionReason::HighRateResidual);
        }
        if !self.returns_to_zero {
            reasons.push(RejectionReason::ReturnToZero);
        }
        reasons
    }
}

pub const fn classify_noise_like(spectral_flatness: f64, tonal_pass_fraction: f64) -> bool {
    spectral_flatness > MAX_SPECTRAL_FLATNESS && tonal_pass_fraction < MIN_TONAL_PASS_FRACTION
}

pub fn evaluate_candidate(
    render: &SubsetRender,
    sample_rate: u32,
    high_rate_residual_db: f64,
) -> CandidateEvidence {
    CandidateEvidence {
        metrics: render.metrics,
        tonal_pass_fraction: measure_tonal_pass_fraction(
            render.candidate,
            &render.samples,
            sample_rate,
        ),
        spectral_flatness: measure_spectral_flatness(
            &render.samples,
            render.active_start_frame,
            render.active_end_frame,
        ),
        high_rate_residual_db,
        returns_to_zero: returns_to_zero(&render.samples),
    }
}

pub fn measure_tonal_pass_fraction(
    candidate: SubsetCandidate,
    samples: &[f32],
    sample_rate: u32,
) -> f64 {
    if sample_rate == 0 || samples.len() < 2 {
        return 0.0;
    }
    let offsets = rebased_offsets_ms(candidate);
    let mut minimum_fraction = 1.0_f64;
    let mut measured = false;
    for (&layer, &offset_ms) in candidate.layers().iter().zip(&offsets) {
        let segments: Vec<(u32, u32, [u8; 3], usize)> = match layer.condition() {
            HybridCondition::Single => vec![(0, 10_000, [38, 38, 38], 1)],
            HybridCondition::HeldChord => vec![(0, 12_000, [50, 53, 57], 3)],
            HybridCondition::Progression | HybridCondition::ProgressionMono => PROGRESSION
                .iter()
                .copied()
                .enumerate()
                .map(|(index, notes)| (index as u32 * 4_000, 4_000, notes, 3))
                .collect(),
        };
        for (segment_ms, duration_ms, notes, unique_count) in segments {
            let start_frame =
                (u64::from(offset_ms + segment_ms) * u64::from(sample_rate) / 1_000) as usize;
            let end_frame = (start_frame
                + (u64::from(duration_ms) * u64::from(sample_rate) / 1_000) as usize)
                .min(samples.len() / 2);
            if end_frame <= start_frame + 64 {
                minimum_fraction = 0.0;
                continue;
            }
            let window = &samples[2 * start_frame..2 * end_frame];
            let levels = measure_frequency_levels(
                window,
                sample_rate as f32,
                notes.map(crate::hybrid::midi_frequency),
            );
            let strongest = levels[..unique_count]
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let passed = levels[..unique_count]
                .iter()
                .filter(|level| strongest - **level <= TARGET_MARGIN_DB)
                .count();
            minimum_fraction = minimum_fraction.min(passed as f64 / unique_count as f64);
            measured = true;
        }
    }
    if measured { minimum_fraction } else { 0.0 }
}

pub fn measure_spectral_flatness(
    samples: &[f32],
    active_start_frame: usize,
    active_end_frame: usize,
) -> f64 {
    const WINDOW: usize = 4_096;
    const BINS: usize = 256;
    let available = active_end_frame.saturating_sub(active_start_frame);
    if available < 64 {
        return 1.0;
    }
    let count = available.min(WINDOW);
    let start = active_start_frame + (available - count) / 2;
    let mut logarithmic_sum = 0.0;
    let mut arithmetic_sum = 0.0;
    for bin in 1..=BINS {
        let mut real = 0.0;
        let mut imaginary = 0.0;
        for index in 0..count {
            let frame = start + index;
            let mono = 0.5 * f64::from(samples[2 * frame] + samples[2 * frame + 1]);
            let window = 0.5 - 0.5 * (TAU * index as f64 / (count - 1).max(1) as f64).cos();
            let phase = TAU * bin as f64 * index as f64 / (2 * BINS) as f64;
            real += mono * window * phase.cos();
            imaginary -= mono * window * phase.sin();
        }
        let magnitude = real.hypot(imaginary).max(1.0e-18);
        logarithmic_sum += magnitude.ln();
        arithmetic_sum += magnitude;
    }
    let geometric = (logarithmic_sum / BINS as f64).exp();
    geometric / (arithmetic_sum / BINS as f64).max(1.0e-18)
}

pub fn measure_high_rate_residual(
    candidate: SubsetCandidate,
    sample_rate: u32,
    gain: f32,
    sample_count: usize,
) -> Result<f64, SubsetError> {
    if sample_count < 64 {
        return Err(SubsetError::InvalidSampleRate);
    }
    const FACTOR: usize = 8;
    let mut target = HybridSubsetMixer::new(candidate, sample_rate, gain)?;
    let mut reference = HybridSubsetMixer::new(candidate, sample_rate * FACTOR as u32, gain)?;
    let latest_ms = rebased_offsets_ms(candidate).into_iter().max().unwrap_or(0) + 1_000;
    let target_skip = (u64::from(latest_ms) * u64::from(sample_rate) / 1_000) as usize;
    for _ in 0..target_skip {
        target.sample();
    }
    for _ in 0..target_skip * FACTOR {
        reference.sample();
    }
    let mut target_samples = Vec::with_capacity(sample_count);
    let mut reference_samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let frame = target.sample();
        target_samples.push(0.5 * (frame.left + frame.right));
        let mut sum = 0.0;
        for _ in 0..FACTOR {
            let frame = reference.sample();
            sum += 0.5 * (frame.left + frame.right);
        }
        reference_samples.push(sum / FACTOR as f32);
    }
    Ok(fitted_residual_db(&target_samples, &reference_samples))
}

fn fitted_residual_db(target: &[f32], reference: &[f32]) -> f64 {
    let target_dc =
        target.iter().map(|sample| f64::from(*sample)).sum::<f64>() / target.len() as f64;
    let reference_dc = reference
        .iter()
        .map(|sample| f64::from(*sample))
        .sum::<f64>()
        / reference.len() as f64;
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

fn returns_to_zero(samples: &[f32]) -> bool {
    samples
        .get(samples.len().saturating_sub(2)..)
        .is_some_and(|tail| tail.iter().all(|sample| sample.abs() <= 1.0e-7))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite_machine::OriginalLayer;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn candidates_use_only_exact_original_layers() {
        assert_eq!(SubsetCandidate::ALL.len(), 7);
        for candidate in SubsetCandidate::ALL {
            assert!((3..=4).contains(&candidate.layers().len()));
            assert!(
                candidate
                    .layers()
                    .iter()
                    .all(|layer| OriginalLayer::ALL.contains(layer))
            );
        }
        assert_eq!(
            SubsetCandidate::ThreeSingles.layers(),
            &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ]
        );
        assert_eq!(
            SubsetCandidate::CrossAnchor.layers(),
            &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralChord,
                OriginalLayer::DualProgression,
                OriginalLayer::CrossProgressionMono,
            ]
        );
    }

    #[test]
    fn subset_sampling_is_finite_bounded_and_allocation_free() {
        let mut mixer = HybridSubsetMixer::new(SubsetCandidate::ThreeSingles, 8_000, 1.0).unwrap();
        for _ in 0..1_024 {
            let frame = assert_no_alloc(|| mixer.sample());
            assert!(frame.left.is_finite() && frame.right.is_finite());
            assert!(frame.left.abs() <= OUTPUT_CEILING);
            assert!(frame.right.abs() <= OUTPUT_CEILING);
        }
    }

    #[test]
    fn shared_gain_is_group_wide_and_respects_sparse_ceiling_rule() {
        let previews = preview_all(8_000).unwrap();
        let gains = select_group_gains(&previews).unwrap();
        assert!(gains.three_layer.is_finite() && gains.three_layer > 0.0);
        assert!(gains.four_layer.is_finite() && gains.four_layer > 0.0);
        for preview in previews {
            let gain = gains.for_candidate(preview.candidate);
            let metrics = preview.metrics_at_gain(gain);
            assert!(metrics.ceiling_proportion <= MAX_CEILING_PROPORTION);
        }
    }

    #[test]
    fn noise_requires_flat_spectrum_and_failed_tonal_anchors() {
        assert!(!classify_noise_like(0.75, 1.0));
        assert!(!classify_noise_like(0.25, 0.2));
        assert!(classify_noise_like(0.75, 0.2));
    }

    #[test]
    fn rejection_contract_matches_the_design_thresholds() {
        let passing = CandidateEvidence {
            metrics: SubsetMetrics {
                peak: 0.9,
                active_rms: 0.25,
                dc: 0.0,
                maximum_jump: 0.5,
                correlation: 0.5,
                mono_loss_db: 0.2,
                ceiling_proportion: 0.005,
                sample_hash: 1,
                finite: true,
            },
            tonal_pass_fraction: 1.0,
            spectral_flatness: 0.2,
            high_rate_residual_db: -12.0,
            returns_to_zero: true,
        };
        assert!(passing.rejection_reasons().is_empty());
        let mut clipped = passing;
        clipped.metrics.ceiling_proportion = 0.010_001;
        assert!(
            clipped
                .rejection_reasons()
                .contains(&RejectionReason::ExcessiveCeiling)
        );
        let mut quiet = passing;
        quiet.metrics.active_rms = 0.19;
        assert!(
            quiet
                .rejection_reasons()
                .contains(&RejectionReason::ActiveRms)
        );
        let mut residual = passing;
        residual.high_rate_residual_db = -0.99;
        assert!(
            residual
                .rejection_reasons()
                .contains(&RejectionReason::HighRateResidual)
        );
    }
}
