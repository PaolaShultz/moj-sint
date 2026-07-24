use crate::composite_machine::{
    OriginalLayer, original_composite_layer_gain, render_original_layer,
};
use crate::dsp::hybrid::HybridFrame;
use crate::hybrid::HybridRenderError;
use crate::hybrid_subset::OUTPUT_CEILING;
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
}
