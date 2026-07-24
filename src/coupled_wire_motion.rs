use crate::dsp::hybrid::HybridFrame as StereoFrame;
use crate::struck_object::{StruckError, StruckObject, StruckTopology};
use std::f32::consts::TAU;

const DEVELOPED_DECAY_SCALE: f32 = 2.6;
const MOTION_SPLIT_HZ: f32 = 320.0;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::struck_object::window_rms;
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
}
