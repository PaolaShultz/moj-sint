//! Non-real-time evidence for the isolated Strange Oscillator gate.

use crate::research::{ResearchMeasureError, ResearchMetrics, fitted_residual_db, measure_stereo};
use crate::strange::{StrangeControls, StrangeError, StrangeType, StrangeVoice};
use thiserror::Error;

const SAMPLE_RATE: f32 = 48_000.0;
const NOTES: [u8; 3] = [36, 60, 84];

#[derive(Clone, Debug, PartialEq)]
pub struct StrangeRender {
    pub samples: Vec<f32>,
    pub metrics: ResearchMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrangeNoteEvidence {
    pub note: u8,
    pub metrics: ResearchMetrics,
    pub high_rate_residual_db: Option<f64>,
    pub cyclic_envelope_depth_db: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrangeControlEvidence {
    pub slot: usize,
    pub residual_rms: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrangeGateEvidence {
    pub kind: StrangeType,
    pub notes: Vec<StrangeNoteEvidence>,
    pub control_rows: Vec<StrangeControlEvidence>,
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralMacroMetric {
    SpectralShapeDb,
    CouplingTopologyDistance,
    MotionRateRatio,
    CycleIrregularity,
    BrightnessRatio,
    StereoWidthDb,
}

impl StructuralMacroMetric {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SpectralShapeDb => "spectral_shape_db",
            Self::CouplingTopologyDistance => "coupling_topology_distance",
            Self::MotionRateRatio => "motion_rate_ratio",
            Self::CycleIrregularity => "cycle_irregularity",
            Self::BrightnessRatio => "brightness_ratio",
            Self::StereoWidthDb => "stereo_width_db",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StructuralMacroEvidence {
    pub slot: usize,
    pub metric: StructuralMacroMetric,
    pub value: f64,
    pub floor: f64,
}

#[derive(Debug, Error)]
pub enum StrangeLabError {
    #[error("sample count must be at least 256")]
    TooFewSamples,
    #[error(transparent)]
    Voice(#[from] StrangeError),
    #[error(transparent)]
    Measure(#[from] ResearchMeasureError),
}

pub fn render_and_measure(
    kind: StrangeType,
    controls: StrangeControls,
    note: u8,
    sample_count: usize,
) -> Result<StrangeRender, StrangeLabError> {
    if sample_count < 256 {
        return Err(StrangeLabError::TooFewSamples);
    }
    let frequency_hz = midi_frequency(note);
    let mut voice = StrangeVoice::new(kind, SAMPLE_RATE, frequency_hz, seed(kind), controls)?;
    let mut samples = Vec::with_capacity(2 * sample_count);
    for _ in 0..sample_count {
        let frame = voice.sample();
        samples.push(frame.left);
        samples.push(frame.right);
    }
    let metrics = measure_stereo(&samples, SAMPLE_RATE, frequency_hz)?;
    Ok(StrangeRender { samples, metrics })
}

pub fn measure_high_rate_residual(
    kind: StrangeType,
    controls: StrangeControls,
    note: u8,
    sample_count: usize,
) -> Result<Option<f64>, StrangeLabError> {
    if sample_count < 256 {
        return Err(StrangeLabError::TooFewSamples);
    }
    const FACTOR: usize = 8;
    const WARMUP: usize = 4_096;
    const HALF_TAPS: usize = 64;
    let frequency_hz = midi_frequency(note);
    let mut target = StrangeVoice::new(kind, SAMPLE_RATE, frequency_hz, seed(kind), controls)?;
    let mut reference = StrangeVoice::new(
        kind,
        SAMPLE_RATE * FACTOR as f32,
        frequency_hz,
        seed(kind),
        controls,
    )?;
    for _ in 0..WARMUP {
        target.sample();
    }
    let mut target_samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        target_samples.push(target.sample().left);
    }
    let high_count = (WARMUP + sample_count + HALF_TAPS + 1) * FACTOR;
    let mut high_samples = Vec::with_capacity(high_count);
    for _ in 0..high_count {
        high_samples.push(reference.sample().left);
    }
    let coefficients = decimation_coefficients::<HALF_TAPS, FACTOR>();
    let mut reduced = Vec::with_capacity(sample_count);
    for output_index in 0..sample_count {
        let center = (WARMUP + output_index) * FACTOR;
        let mut sample = 0.0_f64;
        for (tap, coefficient) in coefficients.iter().enumerate() {
            let offset = tap as isize - HALF_TAPS as isize;
            let index = (center as isize + offset) as usize;
            sample += f64::from(high_samples[index]) * coefficient;
        }
        reduced.push(sample as f32);
    }
    Ok(Some(fitted_residual_db(&target_samples, &reduced)))
}

pub fn measure_cyclic_envelope_depth_db(
    kind: StrangeType,
    controls: StrangeControls,
    note: u8,
    sample_count: usize,
) -> Result<Option<f64>, StrangeLabError> {
    if sample_count < 256 {
        return Err(StrangeLabError::TooFewSamples);
    }
    if kind != StrangeType::ModulatedResonator {
        return Ok(None);
    }
    let rendered = render_and_measure(kind, controls, note, sample_count)?;
    const WINDOW_FRAMES: usize = 1_024;
    let mut minimum = f64::INFINITY;
    let mut maximum = 0.0_f64;
    for window in rendered.samples.chunks_exact(2 * WINDOW_FRAMES).skip(2) {
        let energy = window
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / window.len() as f64;
        let rms = energy.sqrt();
        minimum = minimum.min(rms);
        maximum = maximum.max(rms);
    }
    Ok(Some(20.0 * (maximum / minimum.max(1.0e-12)).log10()))
}

pub fn evaluate_type(kind: StrangeType) -> Result<StrangeGateEvidence, StrangeLabError> {
    let mut notes = Vec::with_capacity(NOTES.len());
    let mut failures = Vec::new();
    for note in NOTES {
        let render = render_and_measure(kind, StrangeControls::MIDPOINT, note, 16_384)?;
        let residual = measure_high_rate_residual(kind, StrangeControls::MIDPOINT, note, 8_192)?;
        let cyclic_envelope_depth_db =
            measure_cyclic_envelope_depth_db(kind, StrangeControls::MIDPOINT, note, 96_000)?;
        if !render.metrics.finite {
            failures.push(format!("note {note} produced non-finite output"));
        }
        if render.metrics.peak > 1.0 {
            failures.push(format!("note {note} peak exceeded 1.0"));
        }
        if render.metrics.rms <= 0.001 {
            failures.push(format!("note {note} was effectively silent"));
        }
        if render.metrics.dc.abs() >= 0.02 {
            failures.push(format!("note {note} DC exceeded 0.02"));
        }
        if render.metrics.maximum_jump >= 1.25 {
            failures.push(format!("note {note} jump exceeded 1.25"));
        }
        if render.metrics.mono_rms <= 0.001 {
            failures.push(format!("note {note} mono fold was effectively silent"));
        }
        if let Some(residual_db) = residual {
            let floor = if note == 84 { -12.0 } else { -18.0 };
            if residual_db > floor {
                failures.push(format!(
                    "note {note} high-rate residual {residual_db:.3} dB exceeded {floor:.0} dB"
                ));
            }
        }
        if let Some(depth_db) = cyclic_envelope_depth_db
            && !(3.0..=24.0).contains(&depth_db)
        {
            failures.push(format!(
                "note {note} cyclic envelope depth {depth_db:.3} dB was outside 3-24 dB"
            ));
        }
        notes.push(StrangeNoteEvidence {
            note,
            metrics: render.metrics,
            high_rate_residual_db: residual,
            cyclic_envelope_depth_db,
        });
    }

    let mut control_rows = Vec::with_capacity(7);
    for slot in 0..7 {
        let low = render_and_measure(kind, StrangeControls::MIDPOINT.with(slot, 0.1), 60, 96_000)?;
        let high = render_and_measure(kind, StrangeControls::MIDPOINT.with(slot, 0.9), 60, 96_000)?;
        let residual_rms = low
            .samples
            .iter()
            .zip(&high.samples)
            .map(|(&left, &right)| f64::from(left - right).powi(2))
            .sum::<f64>()
            / low.samples.len() as f64;
        let residual_rms = residual_rms.sqrt();
        let floor = if matches!(
            kind,
            StrangeType::Triangle
                | StrangeType::Saw
                | StrangeType::Pulse
                | StrangeType::ModulatedResonator
        ) {
            0.02
        } else {
            0.002
        };
        if residual_rms <= floor {
            failures.push(format!(
                "macro slot {slot} residual {residual_rms:.6} did not exceed {floor:.3}"
            ));
        }
        control_rows.push(StrangeControlEvidence { slot, residual_rms });
    }

    let (status, reason) = if failures.is_empty() {
        let policy = if kind == StrangeType::ModulatedResonator {
            "tonal high-rate residual and 0.05-50 Hz cyclic-envelope policies passed"
        } else {
            "tonal high-rate residual policy passed"
        };
        ("pass".to_string(), policy.to_string())
    } else {
        ("reject".to_string(), failures.join("; "))
    };
    Ok(StrangeGateEvidence {
        kind,
        notes,
        control_rows,
        status,
        reason,
    })
}

pub fn evaluate_structural_macros(
    kind: StrangeType,
) -> Result<Vec<StructuralMacroEvidence>, StrangeLabError> {
    let mut rows = Vec::with_capacity(7);
    for slot in 0..7 {
        let (metric, value, floor) = match slot {
            0..=1 => {
                let low = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.1),
                    60,
                    96_000,
                )?;
                let high = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.9),
                    60,
                    96_000,
                )?;
                (
                    StructuralMacroMetric::SpectralShapeDb,
                    harmonic_profile_distance_db(&low.metrics, &high.metrics),
                    6.0,
                )
            }
            2 => {
                let low = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.1),
                    60,
                    96_000,
                )?;
                let high = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.9),
                    60,
                    96_000,
                )?;
                let cycle_decorrelation = (cycle_correlation(&low.samples, midi_frequency(60))
                    - cycle_correlation(&high.samples, midi_frequency(60)))
                .max(0.0);
                let spectral_distance =
                    harmonic_profile_distance_db(&low.metrics, &high.metrics) / 24.0;
                (
                    StructuralMacroMetric::CouplingTopologyDistance,
                    cycle_decorrelation.max(spectral_distance),
                    0.25,
                )
            }
            3 => {
                let low = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.1),
                    60,
                    576_000,
                )?;
                let high = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.9),
                    60,
                    96_000,
                )?;
                let low_depth = window_envelope_depth_db(&low.samples, 4_096);
                let high_depth = window_envelope_depth_db(&high.samples, 256);
                let rate_ratio = StrangeControls::MIDPOINT.with(slot, 0.9).lfo_rate_hz()
                    / StrangeControls::MIDPOINT.with(slot, 0.1).lfo_rate_hz();
                (
                    StructuralMacroMetric::MotionRateRatio,
                    if low_depth >= 6.0 && high_depth >= 6.0 {
                        f64::from(rate_ratio)
                    } else {
                        0.0
                    },
                    200.0,
                )
            }
            4 => {
                let low = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.1),
                    60,
                    384_000,
                )?;
                let high = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.9),
                    60,
                    384_000,
                )?;
                let cycle_frames =
                    (SAMPLE_RATE / StrangeControls::MIDPOINT.lfo_rate_hz()).round() as usize;
                (
                    StructuralMacroMetric::CycleIrregularity,
                    (cycle_envelope_difference(&high.samples, cycle_frames)
                        - cycle_envelope_difference(&low.samples, cycle_frames))
                    .max(0.0),
                    0.08,
                )
            }
            5 => {
                let low = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.1),
                    60,
                    96_000,
                )?;
                let high = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.9),
                    60,
                    96_000,
                )?;
                (
                    StructuralMacroMetric::BrightnessRatio,
                    harmonic_brightness(&high.metrics)
                        / harmonic_brightness(&low.metrics).max(1.0e-12),
                    2.0,
                )
            }
            6 => {
                let low = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.1),
                    60,
                    96_000,
                )?;
                let high = render_and_measure(
                    kind,
                    StrangeControls::MIDPOINT.with(slot, 0.9),
                    60,
                    96_000,
                )?;
                let low_width = low.metrics.side_to_mid.max(1.0e-12);
                let high_width = high.metrics.side_to_mid.max(1.0e-12);
                (
                    StructuralMacroMetric::StereoWidthDb,
                    10.0 * (high_width / low_width).log10(),
                    12.0,
                )
            }
            _ => unreachable!(),
        };
        rows.push(StructuralMacroEvidence {
            slot,
            metric,
            value,
            floor,
        });
    }
    Ok(rows)
}

fn harmonic_profile_distance_db(left: &ResearchMetrics, right: &ResearchMetrics) -> f64 {
    let left_peak = left
        .harmonics_db
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let right_peak = right
        .harmonics_db
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    (left
        .harmonics_db
        .iter()
        .zip(right.harmonics_db)
        .map(|(left, right)| {
            let left = (left - left_peak).max(-72.0);
            let right = (right - right_peak).max(-72.0);
            (left - right).powi(2)
        })
        .sum::<f64>()
        / left.harmonics_db.len() as f64)
        .sqrt()
}

fn cycle_correlation(samples: &[f32], frequency_hz: f32) -> f64 {
    let period = (SAMPLE_RATE / frequency_hz).round() as usize;
    let mono = samples
        .chunks_exact(2)
        .map(|frame| 0.5 * f64::from(frame[0] + frame[1]))
        .collect::<Vec<_>>();
    let start = 4 * period;
    if mono.len() <= start + period {
        return 0.0;
    }
    let current = &mono[start..mono.len() - period];
    let delayed = &mono[start + period..];
    let cross = current
        .iter()
        .zip(delayed)
        .map(|(current, delayed)| current * delayed)
        .sum::<f64>();
    let current_energy = current.iter().map(|sample| sample * sample).sum::<f64>();
    let delayed_energy = delayed.iter().map(|sample| sample * sample).sum::<f64>();
    cross / (current_energy * delayed_energy).sqrt().max(1.0e-30)
}

fn window_envelope_depth_db(samples: &[f32], window_frames: usize) -> f64 {
    let mut minimum = f64::INFINITY;
    let mut maximum = 0.0_f64;
    for window in samples.chunks_exact(2 * window_frames).skip(1) {
        let rms = (window
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / window.len() as f64)
            .sqrt();
        minimum = minimum.min(rms);
        maximum = maximum.max(rms);
    }
    20.0 * (maximum / minimum.max(1.0e-12)).log10()
}

fn cycle_envelope_difference(samples: &[f32], cycle_frames: usize) -> f64 {
    const BINS: usize = 128;
    let bin_frames = cycle_frames / BINS;
    let cycles = samples
        .chunks_exact(2 * cycle_frames)
        .skip(1)
        .map(|cycle| {
            cycle
                .chunks_exact(2 * bin_frames)
                .take(BINS)
                .map(|bin| {
                    (bin.iter()
                        .map(|sample| f64::from(*sample).powi(2))
                        .sum::<f64>()
                        / bin.len() as f64)
                        .sqrt()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if cycles.len() < 2 {
        return 0.0;
    }
    cycles
        .windows(2)
        .map(|pair| {
            let difference = pair[0]
                .iter()
                .zip(&pair[1])
                .map(|(left, right)| (left - right).powi(2))
                .sum::<f64>();
            let energy = pair[0]
                .iter()
                .chain(&pair[1])
                .map(|sample| sample * sample)
                .sum::<f64>();
            (2.0 * difference / energy.max(1.0e-30)).sqrt()
        })
        .sum::<f64>()
        / (cycles.len() - 1) as f64
}

fn harmonic_brightness(metrics: &ResearchMetrics) -> f64 {
    let mut weighted = 0.0_f64;
    let mut energy = 0.0_f64;
    for (index, level_db) in metrics.harmonics_db.iter().enumerate() {
        let amplitude = 10.0_f64.powf(level_db.max(-120.0) / 20.0);
        let bin_energy = amplitude * amplitude;
        weighted += (index + 1) as f64 * bin_energy;
        energy += bin_energy;
    }
    weighted / energy.max(1.0e-30)
}

fn decimation_coefficients<const HALF: usize, const FACTOR: usize>() -> Vec<f64> {
    let tap_count = 2 * HALF + 1;
    let cutoff = 0.45 / FACTOR as f64;
    let mut coefficients = Vec::with_capacity(tap_count);
    for tap in 0..tap_count {
        let offset = tap as f64 - HALF as f64;
        let sinc = if offset == 0.0 {
            2.0 * cutoff
        } else {
            (2.0 * std::f64::consts::PI * cutoff * offset).sin() / (std::f64::consts::PI * offset)
        };
        let phase = tap as f64 / (tap_count - 1) as f64;
        let window = 0.42 - 0.5 * (std::f64::consts::TAU * phase).cos()
            + 0.08 * (2.0 * std::f64::consts::TAU * phase).cos();
        coefficients.push(sinc * window);
    }
    let sum = coefficients.iter().sum::<f64>();
    for coefficient in &mut coefficients {
        *coefficient /= sum;
    }
    coefficients
}

pub fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

fn seed(kind: StrangeType) -> u32 {
    0x51a7_2026 ^ ((kind as u32 + 1) * 0x1020_3041)
}
