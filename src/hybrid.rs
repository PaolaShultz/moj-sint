use crate::dsp::hybrid::{HybridError, HybridFamily, HybridFrame, HybridVoice};
use std::f32::consts::TAU;
use thiserror::Error;

pub const PROGRESSION: [[u8; 3]; 4] = [
    [50, 53, 57],
    [46, 50, 53],
    [48, 52, 55],
    [45, 48, 52],
];
const HELD_CHORD: [u8; 3] = [50, 53, 57];
const SINGLE_NOTE: u8 = 38;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HybridCondition {
    Single,
    HeldChord,
    Progression,
    ProgressionMono,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HybridRenderSpec {
    pub sample_rate: u32,
    pub family: HybridFamily,
    pub condition: HybridCondition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HybridMetrics {
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub crest_factor: f64,
    pub maximum_jump: f64,
    pub correlation: f64,
    pub side_to_mid: f64,
    pub mono_rms: f64,
    pub difference_rms: f64,
    pub target_levels_db: [f64; 3],
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HybridRender {
    pub samples: Vec<f32>,
    pub metrics: HybridMetrics,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum HybridRenderError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error("stereo sample buffer must be non-empty and complete")]
    InvalidSamples,
    #[error(transparent)]
    Voice(#[from] HybridError),
}

#[derive(Debug)]
pub struct HybridEnsemble {
    voices: [HybridVoice; 3],
    notes: [u8; 3],
}

impl HybridEnsemble {
    pub fn new(
        family: HybridFamily,
        sample_rate: f32,
        notes: [u8; 3],
        seed: u32,
    ) -> Result<Self, HybridError> {
        let voices = [
            HybridVoice::new(
                family,
                sample_rate,
                midi_frequency(notes[0]),
                seed ^ 0x243f_6a88,
            )?,
            HybridVoice::new(
                family,
                sample_rate,
                midi_frequency(notes[1]),
                seed ^ 0x85a3_08d3,
            )?,
            HybridVoice::new(
                family,
                sample_rate,
                midi_frequency(notes[2]),
                seed ^ 0x1319_8a2e,
            )?,
        ];
        Ok(Self { voices, notes })
    }

    #[inline]
    pub fn sample(&mut self) -> HybridFrame {
        let mut left = 0.0;
        let mut right = 0.0;
        for voice in &mut self.voices {
            let frame = voice.sample();
            left += frame.left;
            right += frame.right;
        }
        HybridFrame {
            left: (0.28 * left).clamp(-1.0, 1.0),
            right: (0.28 * right).clamp(-1.0, 1.0),
        }
    }

    pub fn notes(&self) -> [u8; 3] {
        self.notes
    }
}

pub fn render_hybrid(spec: HybridRenderSpec) -> Result<HybridRender, HybridRenderError> {
    if spec.sample_rate == 0 {
        return Err(HybridRenderError::InvalidSampleRate);
    }
    let sample_rate = spec.sample_rate as f32;
    let mut samples = Vec::new();
    let target_notes = match spec.condition {
        HybridCondition::Single => {
            let mut voice = HybridVoice::new(
                spec.family,
                sample_rate,
                midi_frequency(SINGLE_NOTE),
                0x5eed_0038,
            )?;
            render_voice(&mut samples, &mut voice, spec.sample_rate, 10.0);
            [SINGLE_NOTE; 3]
        }
        HybridCondition::HeldChord => {
            let mut ensemble =
                HybridEnsemble::new(spec.family, sample_rate, HELD_CHORD, 0x5eed_0057)?;
            render_ensemble(&mut samples, &mut ensemble, spec.sample_rate, 12.0);
            HELD_CHORD
        }
        HybridCondition::Progression | HybridCondition::ProgressionMono => {
            for (index, notes) in PROGRESSION.iter().copied().enumerate() {
                let mut ensemble = HybridEnsemble::new(
                    spec.family,
                    sample_rate,
                    notes,
                    0x5eed_1000 ^ index as u32 * 0x9e37,
                )?;
                let start = samples.len();
                render_ensemble(&mut samples, &mut ensemble, spec.sample_rate, 4.0);
                apply_segment_fade(&mut samples[start..], spec.sample_rate, 0.025);
            }
            PROGRESSION[0]
        }
    };
    apply_segment_fade(&mut samples, spec.sample_rate, 0.01);
    if spec.condition == HybridCondition::ProgressionMono {
        for frame in samples.chunks_exact_mut(2) {
            let mono = 0.5 * (frame[0] + frame[1]);
            frame[0] = mono;
            frame[1] = mono;
        }
    }
    let metrics = measure_hybrid(&samples, sample_rate, target_notes)?;
    Ok(HybridRender { samples, metrics })
}

fn render_voice(samples: &mut Vec<f32>, voice: &mut HybridVoice, sample_rate: u32, seconds: f32) {
    let frames = (sample_rate as f32 * seconds).round() as usize;
    samples.reserve(2 * frames);
    for _ in 0..frames {
        let frame = voice.sample();
        samples.push(frame.left);
        samples.push(frame.right);
    }
}

fn render_ensemble(
    samples: &mut Vec<f32>,
    ensemble: &mut HybridEnsemble,
    sample_rate: u32,
    seconds: f32,
) {
    let frames = (sample_rate as f32 * seconds).round() as usize;
    samples.reserve(2 * frames);
    for _ in 0..frames {
        let frame = ensemble.sample();
        samples.push(frame.left);
        samples.push(frame.right);
    }
}

fn apply_segment_fade(samples: &mut [f32], sample_rate: u32, seconds: f32) {
    let frames = samples.len() / 2;
    if frames == 0 {
        return;
    }
    let fade = ((sample_rate as f32 * seconds).round() as usize).clamp(1, frames / 2);
    for index in 0..frames {
        let fade_in = (index as f32 / fade as f32).min(1.0);
        let fade_out = ((frames - 1 - index) as f32 / fade as f32).min(1.0);
        let gain = fade_in.min(fade_out);
        samples[2 * index] *= gain;
        samples[2 * index + 1] *= gain;
    }
}

pub fn measure_hybrid(
    samples: &[f32],
    sample_rate: f32,
    target_notes: [u8; 3],
) -> Result<HybridMetrics, HybridRenderError> {
    if samples.is_empty() || samples.len() % 2 != 0 || sample_rate <= 0.0 {
        return Err(HybridRenderError::InvalidSamples);
    }
    let frames = samples.len() / 2;
    let mut peak = 0.0_f64;
    let mut energy = 0.0_f64;
    let mut dc = 0.0_f64;
    let mut left_energy = 0.0_f64;
    let mut right_energy = 0.0_f64;
    let mut mid_energy = 0.0_f64;
    let mut side_energy = 0.0_f64;
    let mut difference_energy = 0.0_f64;
    let mut cross = 0.0_f64;
    let mut maximum_jump = 0.0_f64;
    let mut previous = [0.0_f64; 2];
    let mut finite = true;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut mono = Vec::with_capacity(frames);
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
        difference_energy += (left - right) * (left - right);
        dc += mid;
        maximum_jump = maximum_jump
            .max((left - previous[0]).abs())
            .max((right - previous[1]).abs());
        previous = [left, right];
        mono.push(mid as f32);
        for sample in frame {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let rms = (energy / samples.len() as f64).sqrt();
    let mono_rms = (mid_energy / frames as f64).sqrt();
    let target_levels_db = target_notes.map(|note| {
        amplitude_db(project_amplitude(
            &mono,
            sample_rate,
            midi_frequency(note),
        ))
    });
    Ok(HybridMetrics {
        peak,
        rms,
        dc: dc / frames as f64,
        crest_factor: if rms > 0.0 { peak / rms } else { 0.0 },
        maximum_jump,
        correlation: if left_energy > 0.0 && right_energy > 0.0 {
            cross / (left_energy * right_energy).sqrt()
        } else {
            0.0
        },
        side_to_mid: if mid_energy > 0.0 {
            side_energy / mid_energy
        } else {
            0.0
        },
        mono_rms,
        difference_rms: (difference_energy / frames as f64).sqrt(),
        target_levels_db,
        sample_hash: hash,
        finite,
    })
}

pub fn measure_target_levels(samples: &[f32], sample_rate: f32, notes: [u8; 3]) -> [f64; 3] {
    let mono: Vec<f32> = samples
        .chunks_exact(2)
        .map(|frame| 0.5 * (frame[0] + frame[1]))
        .collect();
    notes.map(|note| {
        amplitude_db(project_amplitude(
            &mono,
            sample_rate,
            midi_frequency(note),
        ))
    })
}

pub fn measure_frequency_levels(
    samples: &[f32],
    sample_rate: f32,
    frequencies: [f32; 3],
) -> [f64; 3] {
    let mono: Vec<f32> = samples
        .chunks_exact(2)
        .map(|frame| 0.5 * (frame[0] + frame[1]))
        .collect();
    frequencies.map(|frequency| {
        amplitude_db(project_amplitude(&mono, sample_rate, frequency))
    })
}

pub fn measure_hybrid_alias_error(
    family: HybridFamily,
    note: u8,
    sample_count: usize,
) -> Result<f64, HybridRenderError> {
    if sample_count < 64 {
        return Err(HybridRenderError::InvalidSamples);
    }
    const FACTOR: usize = 8;
    const SAMPLE_RATE: f32 = 48_000.0;
    const WARMUP: usize = 2_048;
    let frequency = midi_frequency(note);
    let seed = 0xa11a_5000 ^ u32::from(note);
    let mut target = HybridVoice::new(family, SAMPLE_RATE, frequency, seed)?;
    let mut reference =
        HybridVoice::new(family, SAMPLE_RATE * FACTOR as f32, frequency, seed)?;
    for _ in 0..WARMUP {
        target.sample();
    }
    for _ in 0..WARMUP * FACTOR {
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

fn project_amplitude(samples: &[f32], sample_rate: f32, frequency: f32) -> f64 {
    let mut sine = 0.0_f64;
    let mut cosine = 0.0_f64;
    for (index, sample) in samples.iter().enumerate() {
        let phase = f64::from(TAU * frequency / sample_rate) * index as f64;
        sine += f64::from(*sample) * phase.sin();
        cosine += f64::from(*sample) * phase.cos();
    }
    2.0 * (sine * sine + cosine * cosine).sqrt() / samples.len() as f64
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
    let gain = if reference_energy > 0.0 {
        dot / reference_energy
    } else {
        0.0
    };
    let residual_energy = target
        .iter()
        .zip(reference)
        .map(|(&target, &reference)| {
            let residual =
                (f64::from(target) - target_dc) - gain * (f64::from(reference) - reference_dc);
            residual * residual
        })
        .sum::<f64>();
    10.0
        * (residual_energy / target_energy.max(1.0e-24))
            .max(1.0e-24)
            .log10()
}

fn amplitude_db(amplitude: f64) -> f64 {
    if amplitude > 0.0 {
        20.0 * amplitude.log10()
    } else {
        -240.0
    }
}

pub fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::{
        HybridCondition, HybridEnsemble, HybridRenderSpec, PROGRESSION, render_hybrid,
    };
    use crate::dsp::hybrid::{HybridFamily, HybridVoice};
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn ensemble_uses_three_independent_fixed_voices_without_allocation() {
        let mut ensemble =
            HybridEnsemble::new(HybridFamily::CrossCoupledMachine, 48_000.0, [50, 53, 57], 9)
                .unwrap();
        let first = assert_no_alloc(|| ensemble.sample());
        assert!(first.left.is_finite() && first.right.is_finite());
        assert!(first.left.abs() <= 1.0 && first.right.abs() <= 1.0);
        assert_eq!(ensemble.notes(), [50, 53, 57]);

        let mut standalone = HybridVoice::new(
            HybridFamily::CrossCoupledMachine,
            48_000.0,
            super::midi_frequency(50),
            9,
        )
        .unwrap();
        assert_ne!(first, standalone.sample());
    }

    #[test]
    fn harmonic_flow_renders_single_chord_progression_and_mono_diagnostic() {
        assert_eq!(
            PROGRESSION,
            [[50, 53, 57], [46, 50, 53], [48, 52, 55], [45, 48, 52]]
        );
        for condition in [
            HybridCondition::Single,
            HybridCondition::HeldChord,
            HybridCondition::Progression,
            HybridCondition::ProgressionMono,
        ] {
            let render = render_hybrid(HybridRenderSpec {
                sample_rate: 8_000,
                family: HybridFamily::SpectralShadow,
                condition,
            })
            .unwrap();
            assert!(!render.samples.is_empty());
            assert!(render.metrics.finite);
            assert!(render.metrics.peak <= 1.0);
            assert!(render.metrics.mono_rms > 0.001);
            if condition == HybridCondition::ProgressionMono {
                assert_eq!(render.metrics.difference_rms, 0.0);
            } else {
                assert!(render.metrics.difference_rms > 0.0);
            }
        }
    }

    #[test]
    fn hybrid_metrics_are_deterministic_and_retain_held_chord_targets() {
        for family in HybridFamily::ALL {
            let spec = HybridRenderSpec {
                sample_rate: 12_000,
                family,
                condition: HybridCondition::HeldChord,
            };
            let first = render_hybrid(spec).unwrap();
            let second = render_hybrid(spec).unwrap();
            assert_eq!(first.metrics.sample_hash, second.metrics.sample_hash);
            assert_eq!(first.samples, second.samples);
            let strongest = first
                .metrics
                .target_levels_db
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            for level in first.metrics.target_levels_db {
                assert!(
                    level >= strongest - 36.0,
                    "family={family:?} levels={:?}",
                    first.metrics.target_levels_db
                );
            }
        }
    }

    #[test]
    fn conservative_alias_evidence_is_deterministic_for_every_hybrid() {
        for family in HybridFamily::ALL {
            let first = super::measure_hybrid_alias_error(family, 60, 2_048).unwrap();
            let second = super::measure_hybrid_alias_error(family, 60, 2_048).unwrap();
            assert_eq!(first, second);
            assert!(first.is_finite(), "family={family:?} residual={first}");
            assert!(first <= 3.0, "family={family:?} residual={first}");
        }
    }
}
