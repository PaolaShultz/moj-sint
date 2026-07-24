use crate::composite_machine::{
    OriginalLayer, original_composite_layer_gain, render_original_layer,
};
use crate::dsp::hybrid::HybridFrame;
use crate::hybrid::HybridRenderError;
use crate::hybrid_subset::{
    MAX_CEILING_PROPORTION, MAX_DC, MAX_JUMP, MAX_MONO_LOSS_DB, MIN_CORRELATION,
    MIN_TONAL_PASS_FRACTION, OUTPUT_CEILING, SubsetCandidate, SubsetMetrics, classify_noise_like,
    measure_spectral_flatness, measure_subset, measure_tonal_pass_fraction,
};
use std::sync::Arc;
use thiserror::Error;

pub const MONOPHONIC_LAYERS: [OriginalLayer; 3] = [
    OriginalLayer::CrossSingle,
    OriginalLayer::SpectralSingle,
    OriginalLayer::DualSingle,
];
pub const LAYER_OFFSETS_MS: [u32; 3] = [0, 2, 5];
pub const DURATION_MS: u32 = 2_400;
pub const DECAY_MS: u32 = 220;
pub const SUSTAIN: f32 = 0.58;
pub const RELEASE_MS: u32 = 500;
pub const MIN_TOTAL_RMS: f64 = 0.199_526_23;
pub const MAX_TOTAL_RMS: f64 = 0.316_227_77;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttackProfile {
    VeryShort,
    Moderate,
    Slow,
}

impl AttackProfile {
    pub const ALL: [Self; 3] = [Self::VeryShort, Self::Moderate, Self::Slow];

    pub const fn attack_ms(self) -> u32 {
        match self {
            Self::VeryShort => 6,
            Self::Moderate => 35,
            Self::Slow => 140,
        }
    }

    pub const fn duration_ms(self) -> u32 {
        DURATION_MS
    }

    pub const fn decay_ms(self) -> u32 {
        DECAY_MS
    }

    pub const fn sustain(self) -> f32 {
        SUSTAIN
    }

    pub const fn release_ms(self) -> u32 {
        RELEASE_MS
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::VeryShort => "attack-006ms",
            Self::Moderate => "attack-035ms",
            Self::Slow => "attack-140ms",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::VeryShort => "very short attack",
            Self::Moderate => "moderately longer attack",
            Self::Slow => "slow attack",
        }
    }

    pub const fn filename(self) -> &'static str {
        match self {
            Self::VeryShort => "01_monophonic_attack_006ms.wav",
            Self::Moderate => "02_monophonic_attack_035ms.wav",
            Self::Slow => "03_monophonic_attack_140ms.wav",
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum EnvelopeAuditionError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error("gain must be finite and positive")]
    InvalidGain,
    #[error("no positive shared gain satisfies total-rms and sparse-ceiling limits")]
    NoSharedGain,
    #[error(transparent)]
    Hybrid(#[from] HybridRenderError),
}

#[derive(Clone)]
struct PreparedLayer {
    samples: Arc<[f32]>,
    start_frame: usize,
}

#[derive(Clone, Copy, Debug)]
struct PreparedEnvelope {
    attack_frames: usize,
    decay_frames: usize,
    release_frames: usize,
}

impl PreparedEnvelope {
    fn new(profile: AttackProfile, sample_rate: u32) -> Self {
        let frames = |milliseconds: u32| {
            ((u64::from(milliseconds) * u64::from(sample_rate)) / 1_000).max(1) as usize
        };
        Self {
            attack_frames: frames(profile.attack_ms()),
            decay_frames: frames(profile.decay_ms()),
            release_frames: frames(profile.release_ms()),
        }
    }

    #[inline]
    fn value_at(self, frame: usize, duration_frames: usize) -> f32 {
        if frame >= duration_frames {
            return 0.0;
        }
        let release_start = duration_frames.saturating_sub(self.release_frames);
        if frame >= release_start {
            let release_length = duration_frames - release_start;
            if release_length <= 1 {
                return 0.0;
            }
            return SUSTAIN * (duration_frames - 1 - frame) as f32 / (release_length - 1) as f32;
        }
        if frame < self.attack_frames {
            return frame as f32 / self.attack_frames as f32;
        }
        if frame < self.attack_frames + self.decay_frames {
            let progress = (frame - self.attack_frames) as f32 / self.decay_frames as f32;
            return 1.0 - (1.0 - SUSTAIN) * progress;
        }
        SUSTAIN
    }
}

pub struct EnvelopeAuditionMixer {
    layers: Vec<PreparedLayer>,
    envelope: PreparedEnvelope,
    frame: usize,
    duration_frames: usize,
    layer_trim: f32,
    gain: f32,
    last_envelope: f32,
}

impl EnvelopeAuditionMixer {
    pub fn new(
        profile: AttackProfile,
        sample_rate: u32,
        gain: f32,
    ) -> Result<Self, EnvelopeAuditionError> {
        if sample_rate == 0 {
            return Err(EnvelopeAuditionError::InvalidSampleRate);
        }
        if !gain.is_finite() || gain <= 0.0 {
            return Err(EnvelopeAuditionError::InvalidGain);
        }
        let mut layers = Vec::with_capacity(MONOPHONIC_LAYERS.len());
        for (&layer, &offset_ms) in MONOPHONIC_LAYERS.iter().zip(&LAYER_OFFSETS_MS) {
            let samples: Arc<[f32]> = render_original_layer(layer, sample_rate)?.into();
            let start_frame = (u64::from(offset_ms) * u64::from(sample_rate) / 1_000) as usize;
            layers.push(PreparedLayer {
                samples,
                start_frame,
            });
        }
        Ok(Self {
            layers,
            envelope: PreparedEnvelope::new(profile, sample_rate),
            frame: 0,
            duration_frames: (u64::from(DURATION_MS) * u64::from(sample_rate) / 1_000) as usize,
            layer_trim: 1.0 / (MONOPHONIC_LAYERS.len() as f32).sqrt(),
            gain,
            last_envelope: 0.0,
        })
    }

    #[inline]
    fn sample_raw(&mut self) -> HybridFrame {
        let mut sum = HybridFrame::default();
        for layer in &self.layers {
            if let Some(local) = self.frame.checked_sub(layer.start_frame) {
                let offset = 2 * local;
                if offset + 1 < layer.samples.len() {
                    sum.left += original_composite_layer_gain() * layer.samples[offset];
                    sum.right += original_composite_layer_gain() * layer.samples[offset + 1];
                }
            }
        }
        sum.left *= self.layer_trim;
        sum.right *= self.layer_trim;
        let envelope = self.envelope.value_at(self.frame, self.duration_frames);
        self.last_envelope = envelope;
        self.frame = self.frame.saturating_add(1);
        HybridFrame {
            left: sum.left * envelope,
            right: sum.right * envelope,
        }
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

    pub fn last_envelope(&self) -> f32 {
        self.last_envelope
    }
}

pub fn measure_total_rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let energy = samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>();
    (energy / samples.len() as f64).sqrt()
}

pub fn total_rms_is_too_low(total_rms: f64) -> bool {
    total_rms < MIN_TOTAL_RMS
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnvelopePreview {
    pub profile: AttackProfile,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnvelopeRender {
    pub profile: AttackProfile,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub gain: f32,
    pub total_rms: f64,
    pub metrics: SubsetMetrics,
    pub active_start_frame: usize,
    pub active_end_frame: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeRejection {
    NonFinite,
    TotalRms,
    ExcessiveDc,
    ExcessiveCeiling,
    MaximumJump,
    StereoMono,
    TonalInventory,
    NoiseLike,
    ReturnToZero,
}

impl EnvelopeRejection {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::NonFinite => "non_finite",
            Self::TotalRms => "total_rms",
            Self::ExcessiveDc => "excessive_dc",
            Self::ExcessiveCeiling => "excessive_ceiling",
            Self::MaximumJump => "maximum_jump",
            Self::StereoMono => "stereo_mono",
            Self::TonalInventory => "tonal_inventory",
            Self::NoiseLike => "noise_like",
            Self::ReturnToZero => "return_to_zero",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeEvidence {
    pub total_rms: f64,
    pub metrics: SubsetMetrics,
    pub tonal_pass_fraction: f64,
    pub spectral_flatness: f64,
    pub returns_to_zero: bool,
}

impl EnvelopeEvidence {
    pub fn rejection_reasons(self) -> Vec<EnvelopeRejection> {
        let mut reasons = Vec::new();
        if !self.metrics.finite || !self.total_rms.is_finite() {
            reasons.push(EnvelopeRejection::NonFinite);
        }
        if total_rms_is_too_low(self.total_rms) || self.total_rms > MAX_TOTAL_RMS {
            reasons.push(EnvelopeRejection::TotalRms);
        }
        if self.metrics.dc.abs() > MAX_DC {
            reasons.push(EnvelopeRejection::ExcessiveDc);
        }
        if self.metrics.ceiling_proportion > MAX_CEILING_PROPORTION {
            reasons.push(EnvelopeRejection::ExcessiveCeiling);
        }
        if self.metrics.maximum_jump > MAX_JUMP {
            reasons.push(EnvelopeRejection::MaximumJump);
        }
        if self.metrics.mono_loss_db > MAX_MONO_LOSS_DB
            || self.metrics.correlation <= MIN_CORRELATION
        {
            reasons.push(EnvelopeRejection::StereoMono);
        }
        if self.tonal_pass_fraction + f64::EPSILON < MIN_TONAL_PASS_FRACTION {
            reasons.push(EnvelopeRejection::TonalInventory);
        }
        if classify_noise_like(self.spectral_flatness, self.tonal_pass_fraction) {
            reasons.push(EnvelopeRejection::NoiseLike);
        }
        if !self.returns_to_zero {
            reasons.push(EnvelopeRejection::ReturnToZero);
        }
        reasons
    }
}

pub fn preview_all(sample_rate: u32) -> Result<Vec<EnvelopePreview>, EnvelopeAuditionError> {
    AttackProfile::ALL
        .into_iter()
        .map(|profile| preview_profile(profile, sample_rate))
        .collect()
}

pub fn preview_profile(
    profile: AttackProfile,
    sample_rate: u32,
) -> Result<EnvelopePreview, EnvelopeAuditionError> {
    let mut mixer = EnvelopeAuditionMixer::new(profile, sample_rate, 1.0)?;
    let mut samples = Vec::with_capacity(2 * mixer.duration_frames());
    for _ in 0..mixer.duration_frames() {
        let frame = mixer.sample_raw();
        samples.extend([frame.left, frame.right]);
    }
    Ok(EnvelopePreview {
        profile,
        samples,
        sample_rate,
    })
}

pub fn select_shared_gain(previews: &[EnvelopePreview]) -> Result<f32, EnvelopeAuditionError> {
    if previews.is_empty() {
        return Err(EnvelopeAuditionError::NoSharedGain);
    }
    let mut maximum = f32::INFINITY;
    for preview in previews {
        let raw_rms = measure_total_rms(&preview.samples);
        if raw_rms > 0.0 {
            maximum = maximum.min((MAX_TOTAL_RMS / raw_rms) as f32);
        }
        let mut magnitudes = preview
            .samples
            .iter()
            .map(|sample| sample.abs())
            .collect::<Vec<_>>();
        magnitudes.sort_unstable_by(f32::total_cmp);
        let index = ((magnitudes.len() as f64 * (1.0 - MAX_CEILING_PROPORTION)).ceil() as usize)
            .saturating_sub(1)
            .min(magnitudes.len().saturating_sub(1));
        let percentile = magnitudes.get(index).copied().unwrap_or(0.0);
        if percentile > 0.0 {
            maximum = maximum.min(OUTPUT_CEILING / percentile);
        }
    }
    if !maximum.is_finite() {
        return Err(EnvelopeAuditionError::NoSharedGain);
    }
    let mut gain = (maximum * 4.0).floor() * 0.25;
    while gain > 0.0 {
        if previews.iter().all(|preview| {
            let samples = apply_gain_and_ceiling(&preview.samples, gain);
            let metrics = measure_subset(&samples, 0, samples.len() / 2);
            measure_total_rms(&samples) <= MAX_TOTAL_RMS
                && metrics.ceiling_proportion <= MAX_CEILING_PROPORTION
        }) {
            return Ok(gain);
        }
        gain -= 0.25;
    }
    Err(EnvelopeAuditionError::NoSharedGain)
}

pub fn render_profile(
    preview: &EnvelopePreview,
    gain: f32,
) -> Result<EnvelopeRender, EnvelopeAuditionError> {
    if !gain.is_finite() || gain <= 0.0 {
        return Err(EnvelopeAuditionError::InvalidGain);
    }
    let samples = apply_gain_and_ceiling(&preview.samples, gain);
    let total_rms = measure_total_rms(&samples);
    let (active_start_frame, active_end_frame) = active_region(&samples);
    let metrics = measure_subset(&samples, active_start_frame, active_end_frame);
    Ok(EnvelopeRender {
        profile: preview.profile,
        samples,
        sample_rate: preview.sample_rate,
        gain,
        total_rms,
        metrics,
        active_start_frame,
        active_end_frame,
    })
}

pub fn evaluate_render(render: &EnvelopeRender) -> EnvelopeEvidence {
    EnvelopeEvidence {
        total_rms: render.total_rms,
        metrics: render.metrics,
        tonal_pass_fraction: measure_tonal_pass_fraction(
            SubsetCandidate::ThreeSingles,
            &render.samples,
            render.sample_rate,
        ),
        spectral_flatness: measure_spectral_flatness(
            &render.samples,
            render.active_start_frame,
            render.active_end_frame,
        ),
        returns_to_zero: returns_to_zero(&render.samples),
    }
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

fn returns_to_zero(samples: &[f32]) -> bool {
    samples
        .get(samples.len().saturating_sub(2)..)
        .is_some_and(|tail| tail.iter().all(|sample| sample.abs() <= 1.0e-7))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hybrid::HybridCondition;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn audition_uses_only_exact_single_note_layers() {
        assert_eq!(
            MONOPHONIC_LAYERS,
            [
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ]
        );
        assert_eq!(LAYER_OFFSETS_MS, [0, 2, 5]);
        assert!(
            MONOPHONIC_LAYERS
                .iter()
                .all(|layer| layer.condition() == HybridCondition::Single)
        );
    }

    #[test]
    fn attack_profiles_are_short_fixed_and_share_the_remaining_shape() {
        assert_eq!(
            AttackProfile::ALL.map(AttackProfile::attack_ms),
            [6, 35, 140]
        );
        for profile in AttackProfile::ALL {
            assert_eq!(profile.duration_ms(), 2_400);
            assert_eq!(profile.decay_ms(), 220);
            assert_eq!(profile.sustain(), 0.58);
            assert_eq!(profile.release_ms(), 500);
            assert!(
                LAYER_OFFSETS_MS
                    .iter()
                    .all(|offset| *offset < profile.attack_ms())
            );
        }
    }

    #[test]
    fn complete_sum_uses_one_finite_allocation_free_envelope() {
        for profile in AttackProfile::ALL {
            let mut mixer = EnvelopeAuditionMixer::new(profile, 8_000, 1.0).unwrap();
            assert_eq!(mixer.duration_frames(), 19_200);
            let first = assert_no_alloc(|| mixer.sample());
            assert_eq!(first, HybridFrame::default());
            for _ in 0..mixer.duration_frames().saturating_sub(2) {
                let frame = assert_no_alloc(|| mixer.sample());
                assert!(frame.left.is_finite() && frame.right.is_finite());
            }
            let final_frame = assert_no_alloc(|| mixer.sample());
            assert_eq!(final_frame, HybridFrame::default());
            assert_eq!(mixer.last_envelope(), 0.0);
        }
    }

    #[test]
    fn total_rms_counts_the_complete_render() {
        let samples = [1.0_f32, 1.0, 0.0, 0.0];
        assert!((measure_total_rms(&samples) - std::f64::consts::FRAC_1_SQRT_2).abs() < 1.0e-12);
    }

    #[test]
    fn low_total_rms_is_rejected_even_when_active_samples_are_loud() {
        assert!(total_rms_is_too_low(0.19));
        assert!(!total_rms_is_too_low(MIN_TOTAL_RMS));
    }

    #[test]
    fn shared_gain_keeps_every_profile_inside_the_total_rms_gate() {
        let previews = preview_all(48_000).unwrap();
        let gain = select_shared_gain(&previews).unwrap();
        for preview in &previews {
            let render = render_profile(preview, gain).unwrap();
            assert!(
                (MIN_TOTAL_RMS..=MAX_TOTAL_RMS).contains(&render.total_rms),
                "{} total RMS {} at shared gain {}",
                render.profile.slug(),
                render.total_rms,
                gain
            );
            assert!(evaluate_render(&render).rejection_reasons().is_empty());
        }
    }
}
