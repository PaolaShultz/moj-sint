use crate::dsp::hybrid::HybridFrame as StereoFrame;
use crate::envelope_audition::measure_total_rms;
use crate::hybrid_subset::{
    MAX_CEILING_PROPORTION, MAX_JUMP, MIN_CORRELATION, MIN_TONAL_PASS_FRACTION, OUTPUT_CEILING,
    SubsetCandidate, SubsetMetrics, classify_noise_like, measure_spectral_flatness, measure_subset,
    measure_tonal_pass_fraction,
};
use crate::struck_object::{
    MAX_ABSOLUTE_DC, MAX_TOTAL_RMS, MIN_TOTAL_RMS, StruckError, StruckObject, StruckTopology,
    window_rms,
};
use std::f32::consts::TAU;

pub const DEVELOPED_DECAY_SCALE: f32 = 5.0;
pub const MOTION_SPLIT_HZ: f32 = 320.0;
pub const REFERENCE_GAIN: f32 = 4.43;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoupledMotionProfile {
    Reference,
    WarmHold,
    SlowOrbit,
    FastOrbit,
}

impl CoupledMotionProfile {
    pub const ALL: [Self; 4] = [
        Self::Reference,
        Self::WarmHold,
        Self::SlowOrbit,
        Self::FastOrbit,
    ];
    pub const DEVELOPED: [Self; 3] = [Self::WarmHold, Self::SlowOrbit, Self::FastOrbit];

    pub const fn spec(self) -> CoupledMotionSpec {
        match self {
            Self::Reference => CoupledMotionSpec {
                duration_ms: 1_600,
                attack_ms: 0,
                decay_ms: 0,
                sustain: 0.0,
                note_off_ms: 0,
                release_ms: 0,
                pan_rate_hz: 0.0,
                pan_depth: 0.0,
            },
            Self::WarmHold => CoupledMotionSpec {
                duration_ms: 3_000,
                attack_ms: 200,
                decay_ms: 650,
                sustain: 0.78,
                note_off_ms: 1_900,
                release_ms: 1_100,
                pan_rate_hz: 0.0,
                pan_depth: 0.0,
            },
            Self::SlowOrbit => CoupledMotionSpec {
                duration_ms: 3_300,
                attack_ms: 200,
                decay_ms: 900,
                sustain: 0.84,
                note_off_ms: 2_100,
                release_ms: 1_200,
                pan_rate_hz: 2.6,
                pan_depth: 0.10,
            },
            Self::FastOrbit => CoupledMotionSpec {
                duration_ms: 3_300,
                attack_ms: 200,
                decay_ms: 500,
                sustain: 0.74,
                note_off_ms: 1_800,
                release_ms: 1_500,
                pan_rate_hz: 6.2,
                pan_depth: 0.055,
            },
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::WarmHold => "warm-hold",
            Self::SlowOrbit => "slow-orbit",
            Self::FastOrbit => "fast-orbit",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Reference => "Coupled Wire Reference",
            Self::WarmHold => "Warm Hold",
            Self::SlowOrbit => "Slow High Orbit",
            Self::FastOrbit => "Fast High Orbit",
        }
    }

    pub const fn filename(self) -> &'static str {
        match self {
            Self::Reference => "00_coupled_wire_reference.wav",
            Self::WarmHold => "01_coupled_wire_warm_hold.wav",
            Self::SlowOrbit => "02_coupled_wire_slow_orbit.wav",
            Self::FastOrbit => "03_coupled_wire_fast_orbit.wav",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoupledMotionSpec {
    pub duration_ms: u32,
    pub attack_ms: u32,
    pub decay_ms: u32,
    pub sustain: f32,
    pub note_off_ms: u32,
    pub release_ms: u32,
    pub pan_rate_hz: f32,
    pub pan_depth: f32,
}

#[derive(Clone, Copy)]
struct OnePoleLowPass {
    coefficient: f32,
    state: f32,
}

impl OnePoleLowPass {
    fn new(sample_rate: u32) -> Self {
        Self {
            coefficient: (-TAU * MOTION_SPLIT_HZ / sample_rate as f32).exp(),
            state: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        self.state = (1.0 - self.coefficient) * input + self.coefficient * self.state;
        self.state
    }
}

#[derive(Clone, Copy)]
struct RotationLfo {
    sine: f32,
    cosine: f32,
    sine_step: f32,
    cosine_step: f32,
}

impl RotationLfo {
    fn new(rate_hz: f32, sample_rate: u32) -> Self {
        let angle = TAU * rate_hz / sample_rate as f32;
        Self {
            sine: 0.0,
            cosine: 1.0,
            sine_step: angle.sin(),
            cosine_step: angle.cos(),
        }
    }

    #[inline]
    fn sample(&mut self) -> f32 {
        let value = self.sine;
        let sine = self.sine * self.cosine_step + self.cosine * self.sine_step;
        let cosine = self.cosine * self.cosine_step - self.sine * self.sine_step;
        self.sine = sine;
        self.cosine = cosine;
        value
    }
}

pub struct CoupledWireEnvelopeVoice {
    profile: CoupledMotionProfile,
    spec: CoupledMotionSpec,
    body: StruckObject,
    sample_rate: u32,
    frame: usize,
    duration_frames: usize,
    high_split: OnePoleLowPass,
    pan_lfo: RotationLfo,
    motion_enabled: bool,
}

impl CoupledWireEnvelopeVoice {
    pub fn new(profile: CoupledMotionProfile, sample_rate: u32) -> Result<Self, StruckError> {
        Self::with_motion_enabled(profile, sample_rate, true)
    }

    fn with_motion_enabled(
        profile: CoupledMotionProfile,
        sample_rate: u32,
        motion_enabled: bool,
    ) -> Result<Self, StruckError> {
        let spec = profile.spec();
        let body = if profile == CoupledMotionProfile::Reference {
            StruckObject::new(StruckTopology::CoupledWire, sample_rate)?
        } else {
            StruckObject::configured(
                StruckTopology::CoupledWire,
                sample_rate,
                spec.duration_ms,
                DEVELOPED_DECAY_SCALE,
            )?
        };
        Ok(Self {
            profile,
            spec,
            body,
            sample_rate,
            frame: 0,
            duration_frames: spec.duration_ms as usize * sample_rate as usize / 1_000,
            high_split: OnePoleLowPass::new(sample_rate),
            pan_lfo: RotationLfo::new(spec.pan_rate_hz, sample_rate),
            motion_enabled,
        })
    }

    #[inline]
    pub fn sample(&mut self) -> StereoFrame {
        if self.frame >= self.duration_frames {
            return StereoFrame::default();
        }
        let body = self.body.sample();
        if self.profile == CoupledMotionProfile::Reference {
            self.frame += 1;
            return body;
        }

        let mid = 0.5 * (body.left + body.right);
        let high_mid = mid - self.high_split.sample(mid);
        let pan = if self.motion_enabled {
            self.spec.pan_depth * self.pan_lfo.sample()
        } else {
            let _ = self.pan_lfo.sample();
            0.0
        };
        let envelope = self.envelope(self.frame);
        self.frame += 1;
        StereoFrame {
            left: (body.left + pan * high_mid) * envelope,
            right: (body.right - pan * high_mid) * envelope,
        }
    }

    #[inline]
    fn envelope(&self, frame: usize) -> f32 {
        let attack = self.spec.attack_ms as usize * self.sample_rate as usize / 1_000;
        let decay = self.spec.decay_ms as usize * self.sample_rate as usize / 1_000;
        let note_off = self.spec.note_off_ms as usize * self.sample_rate as usize / 1_000;
        let release = self.spec.release_ms as usize * self.sample_rate as usize / 1_000;
        if frame < attack {
            return smoothstep(frame as f32 / attack.max(1) as f32);
        }
        if frame < attack + decay {
            let phase = (frame - attack) as f32 / decay.max(1) as f32;
            return 1.0 + (self.spec.sustain - 1.0) * smoothstep(phase);
        }
        if frame < note_off {
            return self.spec.sustain;
        }
        if frame < note_off + release {
            let denominator = release.saturating_sub(1).max(1);
            let phase = (frame - note_off) as f32 / denominator as f32;
            return self.spec.sustain * (1.0 - smoothstep(phase.min(1.0)));
        }
        0.0
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }
}

#[inline]
fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

pub fn render_unpresented(
    profile: CoupledMotionProfile,
    sample_rate: u32,
    motion_enabled: bool,
) -> Result<Vec<f32>, StruckError> {
    let mut voice =
        CoupledWireEnvelopeVoice::with_motion_enabled(profile, sample_rate, motion_enabled)?;
    let mut samples = Vec::with_capacity(2 * voice.duration_frames());
    for _ in 0..voice.duration_frames() {
        let frame = voice.sample();
        samples.extend([frame.left, frame.right]);
    }
    Ok(samples)
}

#[derive(Clone, Debug)]
pub struct CoupledMotionPreview {
    pub profile: CoupledMotionProfile,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct CoupledMotionRender {
    pub profile: CoupledMotionProfile,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub gain: f32,
    pub total_rms: f64,
    pub metrics: SubsetMetrics,
    pub attack_rms: [f64; 3],
    pub sustain_rms: f64,
    pub release_early_rms: f64,
    pub release_late_rms: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoupledMotionRejection {
    NonFinite,
    TotalRms,
    ExcessiveDc,
    ExcessiveCeiling,
    MaximumJump,
    StereoMono,
    TonalInventory,
    NoiseLike,
    Attack,
    Sustain,
    Release,
    ReturnToZero,
}

impl CoupledMotionRejection {
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
            Self::Attack => "attack",
            Self::Sustain => "sustain",
            Self::Release => "release",
            Self::ReturnToZero => "return_to_zero",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CoupledMotionEvidence {
    pub total_rms: f64,
    pub metrics: SubsetMetrics,
    pub tonal_pass_fraction: f64,
    pub spectral_flatness: f64,
    pub attack_rms: [f64; 3],
    pub sustain_rms: f64,
    pub release_early_rms: f64,
    pub release_late_rms: f64,
    pub returns_to_zero: bool,
}

impl CoupledMotionEvidence {
    pub fn rejection_reasons(self) -> Vec<CoupledMotionRejection> {
        let mut reasons = Vec::new();
        if !self.metrics.finite || !self.total_rms.is_finite() {
            reasons.push(CoupledMotionRejection::NonFinite);
        }
        if !(MIN_TOTAL_RMS..=MAX_TOTAL_RMS).contains(&self.total_rms) {
            reasons.push(CoupledMotionRejection::TotalRms);
        }
        if self.metrics.dc.abs() > MAX_ABSOLUTE_DC {
            reasons.push(CoupledMotionRejection::ExcessiveDc);
        }
        if self.metrics.ceiling_proportion > MAX_CEILING_PROPORTION {
            reasons.push(CoupledMotionRejection::ExcessiveCeiling);
        }
        if self.metrics.maximum_jump > MAX_JUMP {
            reasons.push(CoupledMotionRejection::MaximumJump);
        }
        if self.metrics.mono_loss_db > 1.0 || self.metrics.correlation <= MIN_CORRELATION {
            reasons.push(CoupledMotionRejection::StereoMono);
        }
        if self.tonal_pass_fraction + f64::EPSILON < MIN_TONAL_PASS_FRACTION {
            reasons.push(CoupledMotionRejection::TonalInventory);
        }
        if classify_noise_like(self.spectral_flatness, self.tonal_pass_fraction) {
            reasons.push(CoupledMotionRejection::NoiseLike);
        }
        if !(self.attack_rms[0] < self.attack_rms[1] && self.attack_rms[1] < self.attack_rms[2]) {
            reasons.push(CoupledMotionRejection::Attack);
        }
        if self.sustain_rms < 0.1 {
            reasons.push(CoupledMotionRejection::Sustain);
        }
        if self.release_late_rms >= self.release_early_rms {
            reasons.push(CoupledMotionRejection::Release);
        }
        if !self.returns_to_zero {
            reasons.push(CoupledMotionRejection::ReturnToZero);
        }
        reasons
    }
}

pub fn preview_developed(sample_rate: u32) -> Result<Vec<CoupledMotionPreview>, StruckError> {
    CoupledMotionProfile::DEVELOPED
        .into_iter()
        .map(|profile| preview(profile, sample_rate))
        .collect()
}

pub fn preview(
    profile: CoupledMotionProfile,
    sample_rate: u32,
) -> Result<CoupledMotionPreview, StruckError> {
    Ok(CoupledMotionPreview {
        profile,
        samples: render_unpresented(profile, sample_rate, true)?,
        sample_rate,
    })
}

pub fn select_shared_gain(previews: &[CoupledMotionPreview]) -> Result<f32, StruckError> {
    if previews.is_empty() {
        return Err(StruckError::NoSharedGain);
    }
    let mut maximum = f32::INFINITY;
    for preview in previews {
        let rms = measure_total_rms(&preview.samples);
        if rms > 0.0 {
            maximum = maximum.min((MAX_TOTAL_RMS / rms) as f32);
        }
        let mut magnitudes = preview
            .samples
            .iter()
            .map(|value| value.abs())
            .collect::<Vec<_>>();
        magnitudes.sort_unstable_by(f32::total_cmp);
        let index = ((magnitudes.len() as f64 * (1.0 - MAX_CEILING_PROPORTION)).ceil() as usize)
            .saturating_sub(1)
            .min(magnitudes.len() - 1);
        if magnitudes[index] > 0.0 {
            maximum = maximum.min(OUTPUT_CEILING / magnitudes[index]);
        }
    }
    let mut gain = (maximum * 100.0).floor() * 0.01;
    while gain > 0.0 {
        if previews.iter().all(|preview| {
            let samples = apply_gain(&preview.samples, gain);
            let metrics = measure_subset(&samples, 0, samples.len() / 2);
            measure_total_rms(&samples) <= MAX_TOTAL_RMS
                && metrics.ceiling_proportion <= MAX_CEILING_PROPORTION
        }) {
            return Ok(gain);
        }
        gain -= 0.01;
    }
    Err(StruckError::NoSharedGain)
}

pub fn render_preview(
    preview: &CoupledMotionPreview,
    gain: f32,
) -> Result<CoupledMotionRender, StruckError> {
    if !gain.is_finite() || gain <= 0.0 {
        return Err(StruckError::InvalidGain);
    }
    let samples = apply_gain(&preview.samples, gain);
    let spec = preview.profile.spec();
    Ok(CoupledMotionRender {
        profile: preview.profile,
        total_rms: measure_total_rms(&samples),
        metrics: measure_subset(&samples, 0, samples.len() / 2),
        attack_rms: [
            window_rms(&samples, 0, 60, preview.sample_rate),
            window_rms(&samples, 80, 140, preview.sample_rate),
            window_rms(&samples, 160, 220, preview.sample_rate),
        ],
        sustain_rms: window_rms(
            &samples,
            spec.note_off_ms.saturating_sub(400) as usize,
            spec.note_off_ms.saturating_sub(100) as usize,
            preview.sample_rate,
        ),
        release_early_rms: window_rms(
            &samples,
            spec.note_off_ms as usize,
            spec.note_off_ms as usize + 200,
            preview.sample_rate,
        ),
        release_late_rms: window_rms(
            &samples,
            spec.duration_ms.saturating_sub(250) as usize,
            spec.duration_ms.saturating_sub(50) as usize,
            preview.sample_rate,
        ),
        samples,
        sample_rate: preview.sample_rate,
        gain,
    })
}

pub fn evaluate(render: &CoupledMotionRender) -> CoupledMotionEvidence {
    CoupledMotionEvidence {
        total_rms: render.total_rms,
        metrics: render.metrics,
        tonal_pass_fraction: measure_tonal_pass_fraction(
            SubsetCandidate::ThreeSingles,
            &render.samples,
            render.sample_rate,
        ),
        spectral_flatness: measure_spectral_flatness(&render.samples, 0, render.samples.len() / 2),
        attack_rms: render.attack_rms,
        sustain_rms: render.sustain_rms,
        release_early_rms: render.release_early_rms,
        release_late_rms: render.release_late_rms,
        returns_to_zero: render.samples[render.samples.len() - 2..]
            .iter()
            .all(|value| value.abs() <= 1.0e-7),
    }
}

fn apply_gain(samples: &[f32], gain: f32) -> Vec<f32> {
    samples
        .iter()
        .map(|value| (value * gain).clamp(-OUTPUT_CEILING, OUTPUT_CEILING))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn profiles_match_the_approved_envelope_and_motion_contract() {
        assert_eq!(
            CoupledMotionProfile::ALL,
            [
                CoupledMotionProfile::Reference,
                CoupledMotionProfile::WarmHold,
                CoupledMotionProfile::SlowOrbit,
                CoupledMotionProfile::FastOrbit,
            ]
        );
        let specs = CoupledMotionProfile::ALL.map(CoupledMotionProfile::spec);
        assert_eq!(
            specs.map(|spec| spec.duration_ms),
            [1_600, 3_000, 3_300, 3_300]
        );
        assert_eq!(specs.map(|spec| spec.attack_ms), [0, 200, 200, 200]);
        assert_eq!(specs.map(|spec| spec.decay_ms), [0, 650, 900, 500]);
        assert_eq!(specs.map(|spec| spec.sustain), [0.0, 0.78, 0.84, 0.74]);
        assert_eq!(specs.map(|spec| spec.note_off_ms), [0, 1_900, 2_100, 1_800]);
        assert_eq!(specs.map(|spec| spec.release_ms), [0, 1_100, 1_200, 1_500]);
        assert_eq!(specs.map(|spec| spec.pan_rate_hz), [0.0, 0.0, 2.6, 6.2]);
        assert_eq!(specs.map(|spec| spec.pan_depth), [0.0, 0.0, 0.10, 0.055]);
    }

    #[test]
    fn developed_profiles_rise_hold_release_and_sample_without_allocation() {
        let sample_rate = 8_000;
        for profile in CoupledMotionProfile::DEVELOPED {
            let spec = profile.spec();
            let mut voice = CoupledWireEnvelopeVoice::new(profile, sample_rate).unwrap();
            let mut samples = Vec::with_capacity(2 * voice.duration_frames());
            for _ in 0..voice.duration_frames() {
                let frame = assert_no_alloc(|| voice.sample());
                assert!(frame.left.is_finite() && frame.right.is_finite());
                samples.extend([frame.left, frame.right]);
            }
            assert_eq!(assert_no_alloc(|| voice.sample()), StereoFrame::default());
            assert_eq!(samples.len(), 2 * spec.duration_ms as usize * 8);

            let first = window_rms(&samples, 0, 60, sample_rate);
            let second = window_rms(&samples, 80, 140, sample_rate);
            let third = window_rms(&samples, 160, 220, sample_rate);
            assert!(second > first, "{} {first} {second}", profile.slug());
            assert!(third > second, "{} {second} {third}", profile.slug());

            let release_early = window_rms(
                &samples,
                spec.note_off_ms as usize,
                spec.note_off_ms as usize + 200,
                sample_rate,
            );
            let release_late = window_rms(
                &samples,
                spec.duration_ms as usize - 250,
                spec.duration_ms as usize - 50,
                sample_rate,
            );
            assert!(
                release_late < release_early,
                "{} {release_early} {release_late}",
                profile.slug()
            );
            assert_eq!(&samples[samples.len() - 2..], &[0.0, 0.0]);
        }
    }

    #[test]
    fn high_motion_preserves_the_complete_mono_sum() {
        let sample_rate = 8_000;
        for profile in [
            CoupledMotionProfile::SlowOrbit,
            CoupledMotionProfile::FastOrbit,
        ] {
            let moving = render_unpresented(profile, sample_rate, true).unwrap();
            let still = render_unpresented(profile, sample_rate, false).unwrap();
            assert_eq!(moving.len(), still.len());
            let maximum_difference = moving
                .chunks_exact(2)
                .zip(still.chunks_exact(2))
                .map(|(moving, still)| ((moving[0] + moving[1]) - (still[0] + still[1])).abs())
                .fold(0.0_f32, f32::max);
            assert!(
                maximum_difference <= 1.0e-6,
                "{} {maximum_difference}",
                profile.slug()
            );
        }
    }

    #[test]
    fn one_shared_gain_keeps_every_development_inside_the_gate() {
        let previews = preview_developed(48_000).unwrap();
        let gain = select_shared_gain(&previews).unwrap();
        for preview in &previews {
            let render = render_preview(preview, gain).unwrap();
            let evidence = evaluate(&render);
            assert!(
                evidence.rejection_reasons().is_empty(),
                "{} {gain} {evidence:?}",
                render.profile.slug()
            );
        }
    }

    #[test]
    fn weak_development_is_rejected_by_whole_file_rms() {
        let preview = preview(CoupledMotionProfile::WarmHold, 8_000).unwrap();
        let render = render_preview(&preview, 0.01).unwrap();
        assert!(
            evaluate(&render)
                .rejection_reasons()
                .contains(&CoupledMotionRejection::TotalRms)
        );
    }
}
