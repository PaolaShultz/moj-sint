use crate::dsp::research::{ResearchError, ResearchFamily, ResearchSource};
use std::f32::consts::TAU;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResearchRenderSpec {
    pub sample_rate: u32,
    pub note: u8,
    pub seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResearchMetrics {
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub fundamental_db: f64,
    pub harmonics_db: [f64; 16],
    pub pitch_retained: bool,
    pub correlation: f64,
    pub side_to_mid: f64,
    pub mono_rms: f64,
    pub maximum_jump: f64,
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResearchRender {
    pub samples: Vec<f32>,
    pub metrics: ResearchMetrics,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ResearchMeasureError {
    #[error("sample rate and duration must be finite and positive")]
    InvalidSpec,
    #[error("stereo samples must contain complete non-empty frames")]
    InvalidSamples,
    #[error(transparent)]
    Source(#[from] ResearchError),
}

pub fn render_and_measure(
    family: ResearchFamily,
    spec: ResearchRenderSpec,
) -> Result<ResearchRender, ResearchMeasureError> {
    if spec.sample_rate == 0 || !spec.seconds.is_finite() || spec.seconds <= 0.0 {
        return Err(ResearchMeasureError::InvalidSpec);
    }
    let sample_rate = spec.sample_rate as f32;
    let frequency_hz = midi_frequency(spec.note);
    let frame_count = (sample_rate * spec.seconds).round() as usize;
    if frame_count == 0 {
        return Err(ResearchMeasureError::InvalidSpec);
    }
    let mut source = ResearchSource::new(family, sample_rate, frequency_hz)?;
    let mut samples = Vec::with_capacity(2 * frame_count);
    for _ in 0..frame_count {
        let frame = source.sample();
        samples.push(frame.left);
        samples.push(frame.right);
    }
    let metrics = measure_stereo(&samples, sample_rate, frequency_hz)?;
    Ok(ResearchRender { samples, metrics })
}

pub fn measure_stereo(
    samples: &[f32],
    sample_rate: f32,
    frequency_hz: f32,
) -> Result<ResearchMetrics, ResearchMeasureError> {
    if samples.is_empty()
        || samples.len() % 2 != 0
        || !sample_rate.is_finite()
        || sample_rate <= 0.0
        || !frequency_hz.is_finite()
        || frequency_hz <= 0.0
    {
        return Err(ResearchMeasureError::InvalidSamples);
    }
    let frame_count = samples.len() / 2;
    let mut peak = 0.0_f64;
    let mut stereo_energy = 0.0_f64;
    let mut mono_energy = 0.0_f64;
    let mut side_energy = 0.0_f64;
    let mut left_energy = 0.0_f64;
    let mut right_energy = 0.0_f64;
    let mut cross = 0.0_f64;
    let mut dc_sum = 0.0_f64;
    let mut maximum_jump = 0.0_f64;
    let mut previous_left = 0.0_f64;
    let mut previous_right = 0.0_f64;
    let mut finite = true;
    let mut sample_hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut mono = Vec::with_capacity(frame_count);
    for frame in samples.chunks_exact(2) {
        let left = f64::from(frame[0]);
        let right = f64::from(frame[1]);
        finite &= left.is_finite() && right.is_finite();
        peak = peak.max(left.abs()).max(right.abs());
        stereo_energy += left * left + right * right;
        left_energy += left * left;
        right_energy += right * right;
        cross += left * right;
        let mid = 0.5 * (left + right);
        let side = 0.5 * (left - right);
        mono_energy += mid * mid;
        side_energy += side * side;
        dc_sum += mid;
        maximum_jump = maximum_jump
            .max((left - previous_left).abs())
            .max((right - previous_right).abs());
        previous_left = left;
        previous_right = right;
        mono.push(mid as f32);
        for sample in frame {
            sample_hash ^= u64::from(sample.to_bits());
            sample_hash = sample_hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    let dc = dc_sum / frame_count as f64;
    let rms = (stereo_energy / samples.len() as f64).sqrt();
    let mono_rms = (mono_energy / frame_count as f64).sqrt();
    let correlation_denominator = (left_energy * right_energy).sqrt();
    let correlation = if correlation_denominator > 0.0 {
        cross / correlation_denominator
    } else {
        0.0
    };
    let side_to_mid = if mono_energy > 0.0 {
        side_energy / mono_energy
    } else {
        0.0
    };
    let mut harmonics_db = [0.0_f64; 16];
    for (index, level) in harmonics_db.iter_mut().enumerate() {
        let harmonic_hz = frequency_hz * (index + 1) as f32;
        *level = if harmonic_hz < 0.5 * sample_rate {
            amplitude_db(project_amplitude(&mono, sample_rate, harmonic_hz))
        } else {
            -240.0
        };
    }
    let strongest_overtone = harmonics_db[1..]
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    Ok(ResearchMetrics {
        peak,
        rms,
        dc,
        fundamental_db: harmonics_db[0],
        harmonics_db,
        pitch_retained: harmonics_db[0] >= strongest_overtone - 30.0,
        correlation,
        side_to_mid,
        mono_rms,
        maximum_jump,
        sample_hash,
        finite,
    })
}

pub fn apply_fades(samples: &mut [f32], sample_rate: u32, seconds: f32) {
    if samples.len() < 2 || sample_rate == 0 || !seconds.is_finite() || seconds <= 0.0 {
        return;
    }
    let frame_count = samples.len() / 2;
    let fade_frames =
        ((sample_rate as f32 * seconds).round() as usize).clamp(1, (frame_count / 2).max(1));
    for frame in 0..frame_count {
        let fade_in = (frame as f32 / fade_frames as f32).min(1.0);
        let fade_out = ((frame_count - 1 - frame) as f32 / fade_frames as f32).min(1.0);
        let gain = fade_in.min(fade_out);
        samples[2 * frame] *= gain;
        samples[2 * frame + 1] *= gain;
    }
}

pub fn loudness_match(samples: &mut [f32], _sample_rate: u32, target_rms: f64) {
    if samples.is_empty() || !target_rms.is_finite() || target_rms <= 0.0 {
        return;
    }
    let energy = samples
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    let current_rms = (energy / samples.len() as f64).sqrt();
    let peak = samples
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0_f64, f64::max);
    if current_rms <= 0.0 || peak <= 0.0 {
        return;
    }
    // Leave one millipercent of margin so the f64 gain-to-f32 sample conversion
    // cannot round a nominal 0.98 peak above the declared ceiling.
    let gain = (target_rms / current_rms).min(0.979 / peak);
    for sample in samples {
        *sample *= gain as f32;
    }
}

pub fn measure_alias_error(
    family: ResearchFamily,
    note: u8,
    sample_count: usize,
) -> Result<f64, ResearchMeasureError> {
    if sample_count < 64 {
        return Err(ResearchMeasureError::InvalidSpec);
    }
    const FACTOR: usize = 8;
    const SAMPLE_RATE: f32 = 48_000.0;
    const WARMUP: usize = 2_048;
    let frequency_hz = midi_frequency(note);
    let mut target = ResearchSource::new(family, SAMPLE_RATE, frequency_hz)?;
    let mut reference = ResearchSource::new(family, SAMPLE_RATE * FACTOR as f32, frequency_hz)?;
    for _ in 0..WARMUP {
        target.sample();
    }
    for _ in 0..WARMUP * FACTOR {
        reference.sample();
    }
    let mut target_samples = Vec::with_capacity(sample_count);
    let mut reference_samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        target_samples.push(target.sample().left);
        let mut sum = 0.0_f32;
        for _ in 0..FACTOR {
            sum += reference.sample().left;
        }
        reference_samples.push(sum / FACTOR as f32);
    }
    Ok(fitted_residual_db(&target_samples, &reference_samples))
}

pub fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

fn project_amplitude(samples: &[f32], sample_rate: f32, frequency_hz: f32) -> f64 {
    let angle = f64::from(TAU * frequency_hz / sample_rate);
    let (rotation_sine, rotation_cosine) = angle.sin_cos();
    let mut sine = 0.0_f64;
    let mut cosine = 1.0_f64;
    let mut sine_sum = 0.0_f64;
    let mut cosine_sum = 0.0_f64;
    for sample in samples {
        let value = f64::from(*sample);
        sine_sum += value * sine;
        cosine_sum += value * cosine;
        let next_sine = sine * rotation_cosine + cosine * rotation_sine;
        cosine = cosine * rotation_cosine - sine * rotation_sine;
        sine = next_sine;
    }
    2.0 * sine_sum.hypot(cosine_sum) / samples.len() as f64
}

fn amplitude_db(amplitude: f64) -> f64 {
    20.0 * amplitude.max(1.0e-12).log10()
}

pub fn fitted_residual_db(target: &[f32], reference: &[f32]) -> f64 {
    if target.is_empty()
        || target.len() != reference.len()
        || target
            .iter()
            .chain(reference)
            .any(|sample| !sample.is_finite())
    {
        return 0.0;
    }
    let target_dc =
        target.iter().map(|sample| f64::from(*sample)).sum::<f64>() / target.len() as f64;
    let reference_dc = reference
        .iter()
        .map(|sample| f64::from(*sample))
        .sum::<f64>()
        / reference.len() as f64;
    let mut dot = 0.0_f64;
    let mut reference_energy = 0.0_f64;
    let mut target_energy = 0.0_f64;
    for (&target, &reference) in target.iter().zip(reference) {
        let target = f64::from(target) - target_dc;
        let reference = f64::from(reference) - reference_dc;
        dot += target * reference;
        reference_energy += reference * reference;
        target_energy += target * target;
    }
    if !target_energy.is_finite() || target_energy <= 1.0e-24 {
        return 0.0;
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
    10.0 * (residual_energy / target_energy).max(1.0e-24).log10()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::research::ResearchFamily;

    #[test]
    fn fitted_residual_removes_dc_and_finite_gain() {
        let reference = [0.25_f32, 0.75, -0.5, 0.125, -0.25, 0.5];
        let identical = fitted_residual_db(&reference, &reference);
        let half_gain = reference.map(|sample| sample * 0.5);
        let scaled = fitted_residual_db(&half_gain, &reference);

        assert!(identical < -200.0, "identical={identical}");
        assert!(scaled < -200.0, "scaled={scaled}");
    }

    #[test]
    fn fitted_residual_rejects_unsafe_or_unmeasurable_input_deterministically() {
        for (target, reference) in [
            (&[][..], &[][..]),
            (&[1.0][..], &[1.0, 2.0][..]),
            (&[0.0, 0.0][..], &[0.0, 0.0][..]),
            (&[f32::NAN, 1.0][..], &[0.0, 1.0][..]),
            (&[0.0, 1.0][..], &[f32::INFINITY, 1.0][..]),
        ] {
            assert_eq!(fitted_residual_db(target, reference), 0.0);
        }
    }

    #[test]
    fn research_metrics_are_deterministic_finite_and_distinct() {
        let spec = ResearchRenderSpec {
            sample_rate: 48_000,
            note: 60,
            seconds: 0.75,
        };
        let mut hashes = Vec::new();
        for family in ResearchFamily::ALL {
            let first = render_and_measure(family, spec).unwrap();
            let second = render_and_measure(family, spec).unwrap();
            assert_eq!(first.metrics, second.metrics, "family={family:?}");
            assert_eq!(first.samples, second.samples, "family={family:?}");
            assert_eq!(first.samples.len(), 72_000);
            assert!(first.metrics.finite, "family={family:?}");
            assert!(first.metrics.peak > 0.01 && first.metrics.peak <= 1.0);
            assert!(first.metrics.rms > 0.001);
            assert!(first.metrics.dc.abs() < 0.1);
            assert!(first.metrics.mono_rms > 0.001);
            hashes.push(first.metrics.sample_hash);
        }
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), ResearchFamily::ALL.len());
    }

    #[test]
    fn spatial_metrics_describe_stereo_and_mono_behavior() {
        let result = render_and_measure(
            ResearchFamily::SpatialMicroDelay,
            ResearchRenderSpec {
                sample_rate: 48_000,
                note: 60,
                seconds: 1.0,
            },
        )
        .unwrap();
        assert!(
            (0.15..0.98).contains(&result.metrics.correlation),
            "correlation={}",
            result.metrics.correlation
        );
        assert!((0.01..0.8).contains(&result.metrics.side_to_mid));
        assert!(result.metrics.mono_rms > 0.01);
        assert!(result.metrics.maximum_jump < 0.5);
    }

    #[test]
    fn offline_loudness_matching_reaches_target_without_clipping() {
        let mut result = render_and_measure(
            ResearchFamily::IntegerSwarm,
            ResearchRenderSpec {
                sample_rate: 48_000,
                note: 60,
                seconds: 1.0,
            },
        )
        .unwrap();
        apply_fades(&mut result.samples, 48_000, 0.01);
        loudness_match(&mut result.samples, 48_000, 0.08);
        let metrics = measure_stereo(&result.samples, 48_000.0, midi_frequency(60)).unwrap();
        assert!((metrics.rms - 0.08).abs() < 0.002, "rms={}", metrics.rms);
        assert!(metrics.peak <= 0.98, "peak={}", metrics.peak);
    }

    #[test]
    fn nonlinear_and_integer_alias_error_reports_are_deterministic() {
        for family in [ResearchFamily::NonlinearPm, ResearchFamily::IntegerSwarm] {
            let first = measure_alias_error(family, 84, 8_192).unwrap();
            let second = measure_alias_error(family, 84, 8_192).unwrap();
            assert_eq!(first, second);
            assert!(
                first.is_finite() && first <= 20.0,
                "family={family:?} value={first}"
            );
        }
    }

    #[test]
    fn spectral_traversal_keeps_the_requested_fundamental_over_a_long_render() {
        let result = render_and_measure(
            ResearchFamily::SpectralTraversal,
            ResearchRenderSpec {
                sample_rate: 48_000,
                note: 60,
                seconds: 1.5,
            },
        )
        .unwrap();
        assert!(
            result.metrics.fundamental_db > -12.0,
            "fundamental_db={}",
            result.metrics.fundamental_db
        );
        let residual = measure_alias_error(ResearchFamily::SpectralTraversal, 60, 8_192).unwrap();
        assert!(residual < -15.0, "alias_error_db={residual}");
    }

    #[test]
    fn high_comb_note_is_dc_controlled_at_common_listening_level() {
        let mut result = render_and_measure(
            ResearchFamily::ExcitedComb,
            ResearchRenderSpec {
                sample_rate: 48_000,
                note: 84,
                seconds: 1.5,
            },
        )
        .unwrap();
        apply_fades(&mut result.samples, 48_000, 0.01);
        loudness_match(&mut result.samples, 48_000, 0.06);
        let metrics = measure_stereo(&result.samples, 48_000.0, midi_frequency(84)).unwrap();
        assert!((metrics.rms - 0.06).abs() < 0.002, "rms={}", metrics.rms);
        assert!(metrics.dc.abs() < 0.005, "dc={}", metrics.dc);
        assert!(metrics.peak <= 0.98, "peak={}", metrics.peak);
    }

    #[test]
    fn integer_swarm_keeps_a_measurable_base_note_under_machine_roughness() {
        let result = render_and_measure(
            ResearchFamily::IntegerSwarm,
            ResearchRenderSpec {
                sample_rate: 48_000,
                note: 60,
                seconds: 0.75,
            },
        )
        .unwrap();
        assert!(
            result.metrics.fundamental_db > -36.0,
            "fundamental_db={}",
            result.metrics.fundamental_db
        );
    }
}
