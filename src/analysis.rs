use crate::dsp::{
    character::{CharacterLayer, CharacterMethod, CharacterWaveshaper},
    oscillator::{BandlimitedOscillator, OscillatorMethod},
};
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterAnalysisSpec {
    pub sample_rate: f32,
    pub frequency_hz: f32,
    pub edge: f32,
    pub couple: f32,
    pub sample_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterMetrics {
    pub alias_error_db: f64,
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub fundamental_db: f64,
    pub harmonics_db: [f64; 8],
    pub pitch_retained: bool,
    pub sample_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntermodulationMetrics {
    pub per_voice_imd_db: f64,
    pub post_mix_imd_db: f64,
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
    #[error("character amounts must be finite and between zero and one")]
    InvalidCharacterAmount,
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

pub fn measure_character_method(
    method: CharacterMethod,
    spec: CharacterAnalysisSpec,
) -> Result<CharacterMetrics, AnalysisError> {
    validate_character_spec(spec)?;
    const WARMUP: usize = 2_048;
    let output = render_character_waveshaper(method, spec, WARMUP, 1);
    let reference = render_character_reference(method, spec, WARMUP);
    let residual = measure_samples(&output, &reference);
    let harmonic = measure_harmonic_system(&output, spec.sample_rate, spec.frequency_hz)?;
    let mut harmonics_db = [0.0; 8];
    for (index, level) in harmonics_db.iter_mut().enumerate() {
        *level = amplitude_db(project_amplitude(
            &output,
            spec.sample_rate,
            spec.frequency_hz * (index + 1) as f32,
        ));
    }
    let strongest_overtone = harmonics_db[1..]
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    Ok(CharacterMetrics {
        alias_error_db: residual.alias_error_db,
        peak: harmonic.peak,
        rms: harmonic.rms,
        dc: harmonic.dc,
        fundamental_db: harmonics_db[0],
        harmonics_db,
        pitch_retained: harmonics_db[0] >= strongest_overtone - 30.0,
        sample_hash: harmonic.sample_hash,
        finite: harmonic.finite && reference.iter().all(|sample| sample.is_finite()),
    })
}

pub fn measure_character_intermodulation(
    method: CharacterMethod,
    sample_rate: f32,
    first_hz: f32,
    second_hz: f32,
    sample_count: usize,
) -> Result<IntermodulationMetrics, AnalysisError> {
    let validation = CharacterAnalysisSpec {
        sample_rate,
        frequency_hz: first_hz,
        edge: 1.0,
        couple: 1.0,
        sample_count,
    };
    validate_character_spec(validation)?;
    if !second_hz.is_finite() || second_hz <= 0.0 || second_hz >= 0.5 * sample_rate {
        return Err(AnalysisError::InvalidFrequency);
    }
    const WARMUP: usize = 2_048;
    let mut first_layer =
        CharacterLayer::new(sample_rate, method).map_err(|_| AnalysisError::InvalidSampleRate)?;
    let mut second_layer =
        CharacterLayer::new(sample_rate, method).map_err(|_| AnalysisError::InvalidSampleRate)?;
    let mut post_layer =
        CharacterLayer::new(sample_rate, method).map_err(|_| AnalysisError::InvalidSampleRate)?;
    let total = WARMUP + sample_count;
    let mut per_voice = Vec::with_capacity(sample_count);
    let mut post_mix = Vec::with_capacity(sample_count);
    for index in 0..total {
        let time = index as f32 / sample_rate;
        let first = 0.35 * (TAU * first_hz * time).sin();
        let second = 0.35 * (TAU * second_hz * time).sin();
        let separated = first_layer.sample(first, 1.0, 1.0) + second_layer.sample(second, 1.0, 1.0);
        let combined = post_layer.sample(first + second, 1.0, 1.0);
        if index >= WARMUP {
            per_voice.push(separated);
            post_mix.push(combined);
        }
    }
    let intermodulation_frequencies = [
        (2.0 * first_hz - second_hz).abs(),
        (2.0 * second_hz - first_hz).abs(),
        2.0 * first_hz + second_hz,
        2.0 * second_hz + first_hz,
    ];
    let desired = |samples: &[f32]| {
        project_amplitude(samples, sample_rate, first_hz)
            + project_amplitude(samples, sample_rate, second_hz)
    };
    let imd = |samples: &[f32]| {
        intermodulation_frequencies
            .iter()
            .map(|frequency| project_amplitude(samples, sample_rate, *frequency))
            .sum::<f64>()
    };
    let ratio_db = |samples: &[f32]| {
        20.0 * (imd(samples).max(1.0e-12) / desired(samples).max(1.0e-12)).log10()
    };
    Ok(IntermodulationMetrics {
        per_voice_imd_db: ratio_db(&per_voice),
        post_mix_imd_db: ratio_db(&post_mix),
        finite: per_voice
            .iter()
            .chain(&post_mix)
            .all(|sample| sample.is_finite()),
    })
}

pub fn measure_harmonic_distribution(
    samples: &[f32],
    sample_rate: f32,
    frequency_hz: f32,
) -> Result<[f64; 12], AnalysisError> {
    validate_spec(AnalysisSpec {
        sample_rate,
        frequency_hz,
        shape: 0.0,
        sample_count: samples.len(),
    })?;
    let mut levels = [0.0; 12];
    for (index, level) in levels.iter_mut().enumerate() {
        *level = amplitude_db(project_amplitude(
            samples,
            sample_rate,
            frequency_hz * (index + 1) as f32,
        ));
    }
    Ok(levels)
}

fn render_character_waveshaper(
    method: CharacterMethod,
    spec: CharacterAnalysisSpec,
    warmup: usize,
    rate_factor: usize,
) -> Vec<f32> {
    let sample_rate = spec.sample_rate * rate_factor as f32;
    let mut waveshaper = CharacterWaveshaper::new(method);
    let total = (warmup + spec.sample_count) * rate_factor;
    let mut output = Vec::with_capacity(spec.sample_count * rate_factor);
    let intensity = 1.0 - (1.0 - spec.edge).powi(3);
    for index in 0..total {
        let dry = 0.8 * (TAU * spec.frequency_hz * index as f32 / sample_rate).sin();
        let generated = waveshaper.sample(dry, spec.edge);
        if index >= warmup * rate_factor {
            output.push(dry - 1.2 * spec.couple * intensity * generated);
        }
    }
    output
}

fn render_character_reference(
    method: CharacterMethod,
    spec: CharacterAnalysisSpec,
    warmup: usize,
) -> Vec<f32> {
    const FACTOR: usize = 8;
    const RADIUS: isize = 63;
    let padding = RADIUS as usize + FACTOR;
    let high_sample_rate = spec.sample_rate * FACTOR as f32;
    let first_stored_index = warmup * FACTOR - padding;
    let final_index = (warmup + spec.sample_count) * FACTOR + RADIUS as usize;
    let mut waveshaper = CharacterWaveshaper::new(CharacterMethod::Direct);
    let intensity = 1.0 - (1.0 - spec.edge).powi(3);
    let mut high_dry = Vec::with_capacity(final_index - first_stored_index);
    let mut high_generated = Vec::with_capacity(final_index - first_stored_index);
    for index in 0..final_index {
        let dry = 0.8 * (TAU * spec.frequency_hz * index as f32 / high_sample_rate).sin();
        let generated = waveshaper.sample(dry, spec.edge);
        if index >= first_stored_index {
            high_dry.push(dry);
            high_generated.push(generated);
        }
    }

    let cutoff = 0.45 / FACTOR as f64;
    let mut kernel = Vec::with_capacity((2 * RADIUS + 1) as usize);
    for offset in -RADIUS..=RADIUS {
        let position = offset as f64;
        let sinc = if offset == 0 {
            2.0 * cutoff
        } else {
            (2.0 * PI as f64 * cutoff * position).sin() / (PI as f64 * position)
        };
        let window = 0.5 + 0.5 * (PI as f64 * position / (RADIUS as f64 + 1.0)).cos();
        kernel.push(sinc * window);
    }
    let kernel_sum = kernel.iter().sum::<f64>();
    for coefficient in &mut kernel {
        *coefficient /= kernel_sum;
    }
    (0..spec.sample_count)
        .map(|index| {
            let center = (padding + index * FACTOR) as isize;
            let filtered = |source_samples: &[f32], offset: isize| {
                kernel
                    .iter()
                    .enumerate()
                    .map(|(tap, coefficient)| {
                        let source = center + offset + tap as isize - RADIUS;
                        coefficient * f64::from(source_samples[source as usize])
                    })
                    .sum::<f64>() as f32
            };
            let dry = filtered(&high_dry, 0);
            let generated = match method {
                CharacterMethod::Direct => filtered(&high_generated, 0),
                CharacterMethod::Adaa1 => filtered(&high_generated, -(FACTOR as isize / 2)),
                CharacterMethod::Oversampled2x => {
                    0.5 * (filtered(&high_generated, 0)
                        + filtered(&high_generated, -(FACTOR as isize / 2)))
                }
            };
            dry - 1.2 * spec.couple * intensity * generated
        })
        .collect()
}

fn project_amplitude(samples: &[f32], sample_rate: f32, frequency_hz: f32) -> f64 {
    if frequency_hz <= 0.0 || frequency_hz >= 0.5 * sample_rate {
        return 0.0;
    }
    let count = samples.len() as f64;
    let mean = samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / count;
    let mut sine_sum = 0.0;
    let mut cosine_sum = 0.0;
    for (index, sample) in samples.iter().enumerate() {
        let phase = TAU as f64 * frequency_hz as f64 * index as f64 / sample_rate as f64;
        let centered = f64::from(*sample) - mean;
        sine_sum += centered * phase.sin();
        cosine_sum += centered * phase.cos();
    }
    (2.0 * sine_sum / count).hypot(2.0 * cosine_sum / count)
}

fn amplitude_db(amplitude: f64) -> f64 {
    20.0 * amplitude.max(1.0e-12).log10()
}

fn validate_character_spec(spec: CharacterAnalysisSpec) -> Result<(), AnalysisError> {
    validate_spec(AnalysisSpec {
        sample_rate: spec.sample_rate,
        frequency_hz: spec.frequency_hz,
        shape: 0.0,
        sample_count: spec.sample_count,
    })?;
    if !spec.edge.is_finite()
        || !spec.couple.is_finite()
        || !(0.0..=1.0).contains(&spec.edge)
        || !(0.0..=1.0).contains(&spec.couple)
    {
        return Err(AnalysisError::InvalidCharacterAmount);
    }
    Ok(())
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
    use crate::dsp::character::CharacterMethod;
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

    #[test]
    fn character_measurements_are_deterministic_finite_and_pitch_retaining() {
        let spec = CharacterAnalysisSpec {
            sample_rate: 48_000.0,
            frequency_hz: 750.0,
            edge: 0.65,
            couple: 0.8,
            sample_count: 8_192,
        };
        for method in CharacterMethod::ALL {
            let first = measure_character_method(method, spec).unwrap();
            let second = measure_character_method(method, spec).unwrap();
            assert_eq!(first, second);
            assert!(
                first.finite && first.pitch_retained,
                "{method:?}: {first:?}"
            );
            assert!(
                first.peak <= 2.0 && first.dc.abs() < 0.01,
                "{method:?}: {first:?}"
            );
            assert!(first.fundamental_db > -12.0, "{method:?}: {first:?}");
            assert!(first.harmonics_db[2] > -80.0, "{method:?}: {first:?}");
            assert!(first.alias_error_db.is_finite() && first.alias_error_db < 0.0);
        }
    }

    #[test]
    fn post_mix_character_processing_creates_more_intermodulation() {
        let metrics = measure_character_intermodulation(
            CharacterMethod::Adaa1,
            48_000.0,
            600.0,
            900.0,
            16_000,
        )
        .unwrap();
        assert!(metrics.finite, "{metrics:?}");
        assert!(
            metrics.post_mix_imd_db > metrics.per_voice_imd_db + 20.0,
            "{metrics:?}"
        );
    }
}
