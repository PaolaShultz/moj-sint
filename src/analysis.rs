use crate::dsp::oscillator::{BandlimitedOscillator, OscillatorMethod};
use std::f32::consts::{PI, TAU};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalysisSpec {
    pub sample_rate: f32,
    pub frequency_hz: f32,
    pub shape: f32,
    pub sample_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformMetrics {
    pub alias_error_db: f64,
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HarmonicSystemMetrics {
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub fundamental_db: f64,
    pub third_db: f64,
    pub nonharmonic_error_db: f64,
    pub pitch_retained: bool,
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum AnalysisError {
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
    #[error("frequency must be finite, positive, and below Nyquist")]
    InvalidFrequency,
    #[error("shape must be finite and between zero and one")]
    InvalidShape,
    #[error("analysis requires at least 64 samples")]
    TooFewSamples,
}

pub fn measure_candidate(
    method: OscillatorMethod,
    spec: AnalysisSpec,
) -> Result<WaveformMetrics, AnalysisError> {
    validate_spec(spec)?;
    let reference = bandlimited_reference(spec)?;
    let mut oscillator = BandlimitedOscillator::new(spec.sample_rate, method)
        .map_err(|_| AnalysisError::InvalidSampleRate)?;
    oscillator.set_frequency(spec.frequency_hz);
    let output: Vec<_> = (0..spec.sample_count)
        .map(|_| oscillator.sample(spec.shape))
        .collect();
    Ok(measure_samples(&output, &reference))
}

pub fn bandlimited_reference(spec: AnalysisSpec) -> Result<Vec<f32>, AnalysisError> {
    validate_spec(spec)?;
    let phase_increment = spec.frequency_hz / spec.sample_rate;
    let harmonic_count = (0.5 * spec.sample_rate / spec.frequency_hz).floor() as usize;
    let mut reference = Vec::with_capacity(spec.sample_count);
    for sample_index in 0..spec.sample_count {
        let phase = ((sample_index as f32 + 0.5) * phase_increment).fract();
        let mut saw = 0.0;
        let mut square = 0.0;
        for harmonic in 1..=harmonic_count {
            let harmonic_f32 = harmonic as f32;
            let sinusoid = (TAU * harmonic_f32 * phase).sin();
            saw -= 2.0 * sinusoid / (PI * harmonic_f32);
            if harmonic % 2 == 1 {
                square += 4.0 * sinusoid / (PI * harmonic_f32);
            }
        }
        reference.push(saw + spec.shape * (square - saw));
    }
    Ok(reference)
}

pub fn measure_harmonic_system(
    samples: &[f32],
    sample_rate: f32,
    frequency_hz: f32,
) -> Result<HarmonicSystemMetrics, AnalysisError> {
    let spec = AnalysisSpec {
        sample_rate,
        frequency_hz,
        shape: 0.0,
        sample_count: samples.len(),
    };
    validate_spec(spec)?;
    let count = samples.len() as f64;
    let dc = samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / count;
    let mut peak = 0.0_f64;
    let mut energy = 0.0;
    let mut finite = true;
    let mut sample_hash = 0xcbf2_9ce4_8422_2325_u64;
    for &sample in samples {
        finite &= sample.is_finite();
        let centered = f64::from(sample) - dc;
        peak = peak.max(f64::from(sample).abs());
        energy += centered * centered;
        sample_hash ^= u64::from(sample.to_bits());
        sample_hash = sample_hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    let harmonic_count = (0.5 * sample_rate / frequency_hz).floor() as usize;
    let mut coefficients = Vec::with_capacity(harmonic_count);
    let mut fundamental_amplitude = 0.0;
    let mut third_amplitude = 0.0;
    for harmonic in 1..=harmonic_count {
        let angle = TAU as f64 * frequency_hz as f64 * harmonic as f64 / sample_rate as f64;
        let (rotation_sine, rotation_cosine) = angle.sin_cos();
        let mut sine = 0.0_f64;
        let mut cosine = 1.0_f64;
        let mut sine_sum = 0.0;
        let mut cosine_sum = 0.0;
        for &sample in samples {
            let centered = f64::from(sample) - dc;
            sine_sum += centered * sine;
            cosine_sum += centered * cosine;
            let next_sine = sine * rotation_cosine + cosine * rotation_sine;
            cosine = cosine * rotation_cosine - sine * rotation_sine;
            sine = next_sine;
        }
        let sine_amplitude = 2.0 * sine_sum / count;
        let cosine_amplitude = 2.0 * cosine_sum / count;
        let amplitude = sine_amplitude.hypot(cosine_amplitude);
        coefficients.push((sine_amplitude, cosine_amplitude));
        if harmonic == 1 {
            fundamental_amplitude = amplitude;
        } else if harmonic == 3 {
            third_amplitude = amplitude;
        }
    }

    let mut phases: Vec<_> = (1..=harmonic_count)
        .map(|harmonic| {
            let angle = TAU as f64 * frequency_hz as f64 * harmonic as f64 / sample_rate as f64;
            let (rotation_sine, rotation_cosine) = angle.sin_cos();
            (0.0_f64, 1.0_f64, rotation_sine, rotation_cosine)
        })
        .collect();
    let mut residual_energy = 0.0;
    for &sample in samples {
        let mut reconstructed = 0.0;
        for ((sine_amplitude, cosine_amplitude), phase) in coefficients.iter().zip(&mut phases) {
            reconstructed += sine_amplitude * phase.0 + cosine_amplitude * phase.1;
            let next_sine = phase.0 * phase.3 + phase.1 * phase.2;
            phase.1 = phase.1 * phase.3 - phase.0 * phase.2;
            phase.0 = next_sine;
        }
        let residual = (f64::from(sample) - dc) - reconstructed;
        residual_energy += residual * residual;
    }
    let residual_ratio = if energy > 0.0 {
        (residual_energy / energy).max(1.0e-12)
    } else {
        1.0
    };
    let rms = (samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / count)
        .sqrt();
    let amplitude_db = |amplitude: f64| 20.0 * amplitude.max(1.0e-12).log10();
    let fundamental_db = amplitude_db(fundamental_amplitude);
    let third_db = amplitude_db(third_amplitude);
    Ok(HarmonicSystemMetrics {
        peak,
        rms,
        dc,
        fundamental_db,
        third_db,
        nonharmonic_error_db: 10.0 * residual_ratio.log10(),
        pitch_retained: fundamental_db >= third_db - 30.0,
        sample_hash,
        finite,
    })
}

fn validate_spec(spec: AnalysisSpec) -> Result<(), AnalysisError> {
    if !spec.sample_rate.is_finite() || spec.sample_rate <= 0.0 {
        return Err(AnalysisError::InvalidSampleRate);
    }
    if !spec.frequency_hz.is_finite()
        || spec.frequency_hz <= 0.0
        || spec.frequency_hz >= 0.5 * spec.sample_rate
    {
        return Err(AnalysisError::InvalidFrequency);
    }
    if !spec.shape.is_finite() || !(0.0..=1.0).contains(&spec.shape) {
        return Err(AnalysisError::InvalidShape);
    }
    if spec.sample_count < 64 {
        return Err(AnalysisError::TooFewSamples);
    }
    Ok(())
}

fn measure_samples(output: &[f32], reference: &[f32]) -> WaveformMetrics {
    assert_eq!(output.len(), reference.len());
    let count = output.len() as f64;
    let output_mean = output.iter().map(|sample| f64::from(*sample)).sum::<f64>() / count;
    let reference_mean = reference
        .iter()
        .map(|sample| f64::from(*sample))
        .sum::<f64>()
        / count;
    let mut output_energy = 0.0;
    let mut reference_energy = 0.0;
    let mut cross_energy = 0.0;
    let mut raw_energy = 0.0;
    let mut peak = 0.0_f64;
    let mut finite = true;
    let mut sample_hash = 0xcbf2_9ce4_8422_2325_u64;
    for (&output_sample, &reference_sample) in output.iter().zip(reference) {
        finite &= output_sample.is_finite();
        let output_f64 = f64::from(output_sample);
        let output_centered = output_f64 - output_mean;
        let reference_centered = f64::from(reference_sample) - reference_mean;
        output_energy += output_centered * output_centered;
        reference_energy += reference_centered * reference_centered;
        cross_energy += output_centered * reference_centered;
        raw_energy += output_f64 * output_f64;
        peak = peak.max(output_f64.abs());
        sample_hash ^= u64::from(output_sample.to_bits());
        sample_hash = sample_hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let fitted_gain = if output_energy > 0.0 {
        (cross_energy / output_energy).clamp(0.0, 4.0)
    } else {
        0.0
    };
    let mut residual_energy = 0.0;
    for (&output_sample, &reference_sample) in output.iter().zip(reference) {
        let output_centered = f64::from(output_sample) - output_mean;
        let reference_centered = f64::from(reference_sample) - reference_mean;
        let residual = fitted_gain * output_centered - reference_centered;
        residual_energy += residual * residual;
    }
    let error_ratio = if reference_energy > 0.0 {
        (residual_energy / reference_energy).max(1.0e-30)
    } else {
        1.0
    };
    WaveformMetrics {
        alias_error_db: 10.0 * error_ratio.log10(),
        peak,
        rms: (raw_energy / count).sqrt(),
        dc: output_mean,
        sample_hash,
        finite,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::oscillator::OscillatorMethod;
    use std::f32::consts::TAU;

    #[test]
    fn identical_signal_and_reference_have_negligible_residual() {
        let signal: Vec<_> = (0..2_048)
            .map(|index| (TAU * index as f32 / 64.0).sin())
            .collect();
        let metrics = measure_samples(&signal, &signal);
        assert!(metrics.finite);
        assert!(metrics.alias_error_db < -140.0, "{metrics:?}");
        assert!((metrics.peak - 1.0).abs() < 1.0e-6);
        assert!((metrics.rms - 0.5_f64.sqrt()).abs() < 1.0e-6);
        assert!(metrics.dc.abs() < 1.0e-6);
    }

    #[test]
    fn candidate_measurements_are_exactly_deterministic() {
        let spec = AnalysisSpec {
            sample_rate: 48_000.0,
            frequency_hz: 1_975.53,
            shape: 0.5,
            sample_count: 8_192,
        };
        for method in [
            OscillatorMethod::PolyBlep,
            OscillatorMethod::IntegratedWavetable,
        ] {
            let first = measure_candidate(method, spec).unwrap();
            let second = measure_candidate(method, spec).unwrap();
            assert_eq!(first, second);
            assert!(first.finite);
            assert!(first.peak <= 1.5);
        }
    }

    #[test]
    fn corrected_candidates_improve_on_a_naive_discontinuous_saw() {
        let spec = AnalysisSpec {
            sample_rate: 48_000.0,
            frequency_hz: 4_186.01,
            shape: 0.0,
            sample_count: 8_192,
        };
        let reference = bandlimited_reference(spec).unwrap();
        let increment = spec.frequency_hz / spec.sample_rate;
        let mut phase = 0.5 * increment;
        let naive: Vec<_> = (0..spec.sample_count)
            .map(|_| {
                let sample = 2.0 * phase - 1.0;
                phase += increment;
                phase -= phase.floor();
                sample
            })
            .collect();
        let naive_metrics = measure_samples(&naive, &reference);
        let polyblep = measure_candidate(OscillatorMethod::PolyBlep, spec).unwrap();
        let integrated = measure_candidate(OscillatorMethod::IntegratedWavetable, spec).unwrap();
        assert!(
            polyblep.alias_error_db < naive_metrics.alias_error_db - 3.0
                || integrated.alias_error_db < naive_metrics.alias_error_db - 3.0,
            "naive={naive_metrics:?}, polyblep={polyblep:?}, integrated={integrated:?}"
        );
    }
}
