use thiserror::Error;

use super::{ListeningPatch, RenderError, render};
use crate::dsp::six_op_pm::algorithm::CLASSIC_ALGORITHMS;
use crate::dsp::six_op_pm::operator::SharedSineTable;
use crate::dsp::six_op_pm::{SineTable, SixOpPatch, SixOpVoice, VoiceError};
use crate::research::{fitted_residual_db, midi_frequency};

const SAMPLE_RATE: u32 = 48_000;
const REFERENCE_FACTOR: usize = 8;
const PROBE_WARMUP: usize = 8_192;
const ALIAS_SAMPLES: usize = 32_768;
const PITCH_SAMPLES: usize = 65_536;
const SPECTRAL_SAMPLES: usize = 8_192;
const ACTIVE_THRESHOLD: f32 = 1.0e-5;
const NOTES: [u8; 3] = [36, 60, 84];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceStatus {
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderMetrics {
    pub peak: f64,
    pub active_rms: f64,
    pub active_rms_dbfs: f64,
    pub crest_factor: f64,
    pub dc: f64,
    pub maximum_jump: f64,
    pub ceiling_contacts: u64,
    pub finite: bool,
    pub sample_hash: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchRow {
    pub patch: ListeningPatch,
    pub note: u8,
    pub measured_frequency_hz: Option<f64>,
    pub anchor_frequency_hz: f64,
    pub anchor_db: f64,
    pub pitch_error_cents: Option<f64>,
    pub inharmonic_rule: bool,
    pub status: EvidenceStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpectralStage {
    Attack,
    Sustain,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpectralRow {
    pub patch: ListeningPatch,
    pub note: u8,
    pub stage: SpectralStage,
    pub harmonic_energy_db: f64,
    pub inharmonic_residual_db: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AliasRow {
    pub patch: ListeningPatch,
    pub note: u8,
    pub alias_error_db: f64,
    pub floor_db: f64,
    pub status: EvidenceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchEvidence {
    pub patch: ListeningPatch,
    pub metrics: RenderMetrics,
    pub pitch_rows: Vec<PitchRow>,
    pub spectral_rows: Vec<SpectralRow>,
    pub alias_rows: Vec<AliasRow>,
    pub status: EvidenceStatus,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PairRmsComparison {
    pub pair: u8,
    pub reference: ListeningPatch,
    pub original: ListeningPatch,
    pub difference_db: f64,
    pub status: EvidenceStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GateEvidence {
    pub patches: [PatchEvidence; 6],
    pub pair_rms: [PairRmsComparison; 3],
    pub status: EvidenceStatus,
    pub errors: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RatioKind {
    Rational,
    Inharmonic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SweepLevel {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SweepRow {
    pub patch: ListeningPatch,
    pub note: u8,
    pub ratio_kind: RatioKind,
    pub modulation: SweepLevel,
    pub feedback: SweepLevel,
    pub stage: SpectralStage,
    pub peak: f64,
    pub finite: bool,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum MeasurementError {
    #[error("measurement samples must be non-empty and finite")]
    InvalidSamples,
    #[error("active region is silent at the fixed threshold")]
    SilentRender,
    #[error("internal evidence inventory is incomplete")]
    IncompleteEvidence,
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error(transparent)]
    Voice(#[from] VoiceError),
}

pub fn measure_listening_gate() -> Result<GateEvidence, MeasurementError> {
    let sine_table: SharedSineTable = SineTable::new().into();
    let mut patches = Vec::with_capacity(ListeningPatch::ALL.len());
    let mut errors = Vec::new();
    for patch in ListeningPatch::ALL {
        let musical = render(patch, SAMPLE_RATE)?;
        let mono: Vec<f32> = musical.samples.iter().step_by(2).copied().collect();
        let metrics = measure_mono(
            &mono,
            musical.diagnostics.clamp_contacts,
            musical.sample_hash,
        )?;

        let mut pitch_rows = Vec::with_capacity(NOTES.len());
        let mut spectral_rows = Vec::with_capacity(NOTES.len() * 3);
        let mut alias_rows = Vec::with_capacity(NOTES.len());
        for note in NOTES {
            let sustained = render_probe(
                patch.patch(),
                SAMPLE_RATE as f32,
                note,
                ProbeStage::Sustain,
                PITCH_SAMPLES,
                sine_table.clone(),
            )?;
            let mut pitch = measure_pitch(patch, note, &sustained.samples);
            pitch.status = probe_status(pitch.status, sustained.finite);
            if pitch.status == EvidenceStatus::Fail {
                errors.push(format!("pitch:{patch:?}:{note}"));
            }
            pitch_rows.push(pitch);

            let mut spectral_probes_finite = sustained.finite;
            for stage in [
                SpectralStage::Attack,
                SpectralStage::Sustain,
                SpectralStage::Release,
            ] {
                let row = if stage == SpectralStage::Sustain {
                    measure_spectrum(patch, note, stage, &sustained.samples[..SPECTRAL_SAMPLES])
                } else {
                    let probe = render_probe(
                        patch.patch(),
                        SAMPLE_RATE as f32,
                        note,
                        ProbeStage::from(stage),
                        SPECTRAL_SAMPLES,
                        sine_table.clone(),
                    )?;
                    spectral_probes_finite &= probe.finite;
                    measure_spectrum(patch, note, stage, &probe.samples)
                };
                spectral_rows.push(row);
            }
            if !spectral_probes_finite {
                errors.push(format!("spectral-non-finite:{patch:?}:{note}"));
            }

            let alias = measure_alias(patch, note, sine_table.clone())?;
            if alias.status == EvidenceStatus::Fail {
                errors.push(format!(
                    "alias:{patch:?}:{note}:{:.3}",
                    alias.alias_error_db
                ));
            }
            alias_rows.push(alias);
        }

        let metrics_pass = metrics.finite
            && metrics.peak <= 0.95
            && metrics.ceiling_contacts == 0
            && metrics.dc.abs() <= 0.0002
            && metrics.maximum_jump <= 0.25
            && metrics.crest_factor.is_finite();
        let spectral_pass = !errors
            .iter()
            .any(|error| error.starts_with(&format!("spectral-non-finite:{patch:?}:")))
            && spectral_rows.iter().all(|row| {
                row.harmonic_energy_db.is_finite() && row.inharmonic_residual_db.is_finite()
            });
        let rows_pass = pitch_rows
            .iter()
            .all(|row| row.status == EvidenceStatus::Pass)
            && alias_rows
                .iter()
                .all(|row| row.status == EvidenceStatus::Pass)
            && spectral_pass;
        let status = status(metrics_pass && rows_pass);
        if !metrics_pass {
            errors.push(format!("metrics:{patch:?}:{metrics:?}"));
        }
        patches.push(PatchEvidence {
            patch,
            metrics,
            pitch_rows,
            spectral_rows,
            alias_rows,
            status,
        });
    }

    let patches: [PatchEvidence; 6] = patches
        .try_into()
        .map_err(|_| MeasurementError::IncompleteEvidence)?;
    let pair_rms = std::array::from_fn(|pair| {
        let reference = &patches[pair * 2];
        let original = &patches[pair * 2 + 1];
        let difference_db =
            (reference.metrics.active_rms_dbfs - original.metrics.active_rms_dbfs).abs();
        PairRmsComparison {
            pair: pair as u8,
            reference: reference.patch,
            original: original.patch,
            difference_db,
            status: status(difference_db.is_finite() && difference_db <= 3.0),
        }
    });
    for pair in &pair_rms {
        if pair.status == EvidenceStatus::Fail {
            errors.push(format!("pair-rms:{}:{:.3}", pair.pair, pair.difference_db));
        }
    }
    let all_patches_pass = patches
        .iter()
        .all(|patch| patch.status == EvidenceStatus::Pass);
    let all_pairs_pass = pair_rms
        .iter()
        .all(|pair| pair.status == EvidenceStatus::Pass);
    Ok(GateEvidence {
        patches,
        pair_rms,
        status: status(all_patches_pass && all_pairs_pass),
        errors,
    })
}

pub fn engineering_sweep() -> Result<Vec<SweepRow>, MeasurementError> {
    let sine_table: SharedSineTable = SineTable::new().into();
    let mut rows = Vec::with_capacity(18);
    for (patch_id, ratio_kind) in [
        (ListeningPatch::ElectricPianoMallet, RatioKind::Rational),
        (ListeningPatch::FracturedMetal, RatioKind::Inharmonic),
    ] {
        for (note_index, note) in NOTES.into_iter().enumerate() {
            for (stage_index, stage) in [
                SpectralStage::Attack,
                SpectralStage::Sustain,
                SpectralStage::Release,
            ]
            .into_iter()
            .enumerate()
            {
                let modulation =
                    [SweepLevel::Low, SweepLevel::Medium, SweepLevel::High][note_index];
                let feedback = [SweepLevel::Low, SweepLevel::Medium, SweepLevel::High][stage_index];
                let mut patch = patch_id.patch();
                apply_modulation_level(&mut patch, modulation);
                patch.feedback = match feedback {
                    SweepLevel::Low => 0.0,
                    SweepLevel::Medium => patch_id.patch().feedback,
                    SweepLevel::High => 1.0,
                };
                let probe = render_probe(
                    patch,
                    SAMPLE_RATE as f32,
                    note,
                    ProbeStage::from(stage),
                    2_048,
                    sine_table.clone(),
                )?;
                let peak = probe
                    .samples
                    .iter()
                    .map(|sample| f64::from(sample.abs()))
                    .fold(0.0, f64::max);
                rows.push(SweepRow {
                    patch: patch_id,
                    note,
                    ratio_kind,
                    modulation,
                    feedback,
                    stage,
                    peak,
                    finite: probe.finite && peak.is_finite(),
                });
            }
        }
    }
    Ok(rows)
}

fn measure_mono(
    samples: &[f32],
    ceiling_contacts: u64,
    sample_hash: u64,
) -> Result<RenderMetrics, MeasurementError> {
    if samples.is_empty() || samples.iter().any(|sample| !sample.is_finite()) {
        return Err(MeasurementError::InvalidSamples);
    }
    let first = samples
        .iter()
        .position(|sample| sample.abs() > ACTIVE_THRESHOLD)
        .ok_or(MeasurementError::SilentRender)?;
    let last = samples
        .iter()
        .rposition(|sample| sample.abs() > ACTIVE_THRESHOLD)
        .ok_or(MeasurementError::SilentRender)?;
    let active = &samples[first..=last];
    let peak = samples
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0, f64::max);
    let active_rms = (active
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / active.len() as f64)
        .sqrt();
    if !active_rms.is_finite() || active_rms <= 0.0 {
        return Err(MeasurementError::SilentRender);
    }
    let dc = samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / samples.len() as f64;
    let maximum_jump = samples
        .windows(2)
        .map(|pair| f64::from((pair[1] - pair[0]).abs()))
        .fold(0.0, f64::max);
    Ok(RenderMetrics {
        peak,
        active_rms,
        active_rms_dbfs: amplitude_db(active_rms),
        crest_factor: peak / active_rms,
        dc,
        maximum_jump,
        ceiling_contacts,
        finite: true,
        sample_hash,
    })
}

fn measure_pitch(patch: ListeningPatch, note: u8, samples: &[f32]) -> PitchRow {
    let anchor_frequency_hz = f64::from(midi_frequency(note));
    let anchor_amplitude = projected_amplitude(samples, SAMPLE_RATE as f64, anchor_frequency_hz);
    let anchor_db = amplitude_db(anchor_amplitude);
    let inharmonic_rule = matches!(
        patch,
        ListeningPatch::BellMetal | ListeningPatch::FracturedMetal
    );
    if inharmonic_rule {
        // Cauchy-Schwarz bounds every normalized complex projection by twice
        // the capture RMS. Using that upper bound prevents a sparse search
        // from understating the strongest partial and admitting a weak anchor.
        let strongest_partial_bound_db = amplitude_db(strongest_partial_upper_bound(samples));
        PitchRow {
            patch,
            note,
            measured_frequency_hz: None,
            anchor_frequency_hz,
            anchor_db,
            pitch_error_cents: None,
            inharmonic_rule: true,
            status: status(
                anchor_db.is_finite()
                    && strongest_partial_bound_db.is_finite()
                    && anchor_db >= strongest_partial_bound_db - 36.0,
            ),
        }
    } else {
        let measured_frequency_hz = fitted_pitch_frequency(samples, anchor_frequency_hz);
        let pitch_error_cents = 1_200.0 * (measured_frequency_hz / anchor_frequency_hz).log2();
        PitchRow {
            patch,
            note,
            measured_frequency_hz: Some(measured_frequency_hz),
            anchor_frequency_hz,
            anchor_db,
            pitch_error_cents: Some(pitch_error_cents),
            inharmonic_rule: false,
            status: status(
                measured_frequency_hz.is_finite()
                    && anchor_db.is_finite()
                    && pitch_error_cents.is_finite()
                    && pitch_error_cents.abs() <= 20.0,
            ),
        }
    }
}

fn fitted_pitch_frequency(samples: &[f32], expected_hz: f64) -> f64 {
    let mut best_cents = 0.0;
    let mut best_amplitude = -1.0;
    for step in -15..=15 {
        let cents = f64::from(step) * 4.0;
        let frequency = expected_hz * 2.0_f64.powf(cents / 1_200.0);
        let amplitude = projected_amplitude(samples, SAMPLE_RATE as f64, frequency);
        if amplitude > best_amplitude {
            best_amplitude = amplitude;
            best_cents = cents;
        }
    }
    let coarse = best_cents;
    for step in -24..=24 {
        let cents = coarse + f64::from(step) * 0.25;
        let frequency = expected_hz * 2.0_f64.powf(cents / 1_200.0);
        let amplitude = projected_amplitude(samples, SAMPLE_RATE as f64, frequency);
        if amplitude > best_amplitude {
            best_amplitude = amplitude;
            best_cents = cents;
        }
    }
    expected_hz * 2.0_f64.powf(best_cents / 1_200.0)
}

fn measure_spectrum(
    patch: ListeningPatch,
    note: u8,
    stage: SpectralStage,
    samples: &[f32],
) -> SpectralRow {
    let total_energy = samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    let fundamental = f64::from(midi_frequency(note));
    let mut harmonic_energy = 0.0;
    for harmonic in 1..=16 {
        let frequency = fundamental * f64::from(harmonic);
        if frequency >= SAMPLE_RATE as f64 * 0.5 {
            break;
        }
        let amplitude = projected_amplitude(samples, SAMPLE_RATE as f64, frequency);
        harmonic_energy += 0.5 * amplitude * amplitude;
    }
    harmonic_energy = harmonic_energy.min(total_energy.max(0.0));
    let residual_energy = (total_energy - harmonic_energy).max(1.0e-24);
    SpectralRow {
        patch,
        note,
        stage,
        harmonic_energy_db: power_db(harmonic_energy),
        inharmonic_residual_db: power_db(residual_energy),
    }
}

fn measure_alias(
    patch: ListeningPatch,
    note: u8,
    sine_table: SharedSineTable,
) -> Result<AliasRow, MeasurementError> {
    let target = render_probe(
        patch.patch(),
        SAMPLE_RATE as f32,
        note,
        ProbeStage::AliasTarget,
        ALIAS_SAMPLES,
        sine_table.clone(),
    )?;
    let reference_rate = SAMPLE_RATE as f32 * REFERENCE_FACTOR as f32;
    let reference = render_probe(
        patch.patch(),
        reference_rate,
        note,
        ProbeStage::AliasReference,
        ALIAS_SAMPLES * REFERENCE_FACTOR,
        sine_table,
    )?;
    let probes_finite = target.finite && reference.finite;
    let reference: Vec<f32> = reference
        .samples
        .chunks_exact(REFERENCE_FACTOR)
        .map(|chunk| {
            chunk.iter().map(|sample| f64::from(*sample)).sum::<f64>() as f32
                / REFERENCE_FACTOR as f32
        })
        .collect();
    let alias_error_db = if probes_finite {
        fitted_residual_db(&target.samples, &reference)
    } else {
        0.0
    };
    let floor_db = if note == 84 { -12.0 } else { -20.0 };
    Ok(AliasRow {
        patch,
        note,
        alias_error_db,
        floor_db,
        status: alias_status(alias_error_db, floor_db),
    })
}

fn alias_status(alias_error_db: f64, floor_db: f64) -> EvidenceStatus {
    status(alias_error_db.is_finite() && floor_db.is_finite() && alias_error_db <= floor_db)
}

const fn probe_status(current: EvidenceStatus, finite: bool) -> EvidenceStatus {
    if finite && matches!(current, EvidenceStatus::Pass) {
        EvidenceStatus::Pass
    } else {
        EvidenceStatus::Fail
    }
}

fn projected_amplitude(samples: &[f32], sample_rate: f64, frequency_hz: f64) -> f64 {
    if samples.is_empty() || !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return 0.0;
    }
    let angle = std::f64::consts::TAU * frequency_hz / sample_rate;
    let (rotation_sine, rotation_cosine) = angle.sin_cos();
    let mut sine = 0.0;
    let mut cosine = 1.0;
    let mut sine_sum = 0.0;
    let mut cosine_sum = 0.0;
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

fn strongest_partial_upper_bound(samples: &[f32]) -> f64 {
    let rms = (samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / samples.len() as f64)
        .sqrt();
    2.0 * rms
}

fn amplitude_db(amplitude: f64) -> f64 {
    20.0 * amplitude.max(1.0e-12).log10()
}

fn power_db(power: f64) -> f64 {
    10.0 * power.max(1.0e-24).log10()
}

const fn status(pass: bool) -> EvidenceStatus {
    if pass {
        EvidenceStatus::Pass
    } else {
        EvidenceStatus::Fail
    }
}

#[derive(Clone, Copy)]
enum ProbeStage {
    Attack,
    Sustain,
    Release,
    AliasTarget,
    AliasReference,
}

impl From<SpectralStage> for ProbeStage {
    fn from(stage: SpectralStage) -> Self {
        match stage {
            SpectralStage::Attack => Self::Attack,
            SpectralStage::Sustain => Self::Sustain,
            SpectralStage::Release => Self::Release,
        }
    }
}

struct Probe {
    samples: Vec<f32>,
    finite: bool,
}

fn render_probe(
    patch: SixOpPatch,
    sample_rate: f32,
    note: u8,
    stage: ProbeStage,
    sample_count: usize,
    sine_table: SharedSineTable,
) -> Result<Probe, MeasurementError> {
    let mut voice = SixOpVoice::new_with_sine_table(patch, sample_rate, note, 0.8, sine_table)?;
    voice.note_on();
    let warmup = match stage {
        ProbeStage::Attack => 0,
        ProbeStage::Sustain => SAMPLE_RATE as usize,
        ProbeStage::Release => SAMPLE_RATE as usize / 2,
        ProbeStage::AliasTarget => PROBE_WARMUP,
        // An eight-sample box has its midpoint half a high-rate sample before
        // the target instant when started four high-rate frames earlier.
        ProbeStage::AliasReference => PROBE_WARMUP * REFERENCE_FACTOR - 4,
    };
    for _ in 0..warmup {
        voice.sample();
    }
    if matches!(stage, ProbeStage::Release) {
        voice.note_off();
    }
    let mut samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        samples.push(voice.sample());
    }
    let finite = samples.iter().all(|sample| sample.is_finite()) && !voice.non_finite_seen();
    Ok(Probe { samples, finite })
}

fn apply_modulation_level(patch: &mut SixOpPatch, level: SweepLevel) {
    let gain = match level {
        SweepLevel::Low => 0.25,
        SweepLevel::Medium => 0.625,
        SweepLevel::High => 1.0,
    };
    let carriers = CLASSIC_ALGORITHMS[patch.algorithm].carriers;
    for (index, operator) in patch.operators.iter_mut().enumerate() {
        if carriers & (1 << index) == 0 {
            operator.output_level *= gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::six_op_pm::ListeningPatch;

    #[test]
    fn alias_floor_is_an_inclusive_upper_bound() {
        assert_eq!(alias_status(-20.01, -20.0), EvidenceStatus::Pass);
        assert_eq!(alias_status(-20.0, -20.0), EvidenceStatus::Pass);
        assert_eq!(alias_status(-19.99, -20.0), EvidenceStatus::Fail);
        assert_eq!(alias_status(f64::NAN, -20.0), EvidenceStatus::Fail);
    }

    #[test]
    fn strongest_partial_bound_never_understates_a_projection() {
        let samples: Vec<f32> = (0..257)
            .map(|index| {
                let phase = std::f32::consts::TAU * index as f32 / 19.0;
                0.4 + 0.6 * phase.sin()
            })
            .collect();
        let bound = strongest_partial_upper_bound(&samples);
        for frequency in [40.0, 2_526.0, 17_913.0] {
            assert!(projected_amplitude(&samples, SAMPLE_RATE as f64, frequency) <= bound);
        }
    }

    #[test]
    fn recovered_non_finite_probe_cannot_retain_a_passing_status() {
        assert_eq!(
            probe_status(EvidenceStatus::Pass, false),
            EvidenceStatus::Fail
        );
        assert_eq!(
            probe_status(EvidenceStatus::Pass, true),
            EvidenceStatus::Pass
        );
        assert_eq!(
            probe_status(EvidenceStatus::Fail, true),
            EvidenceStatus::Fail
        );
    }

    #[test]
    fn active_render_metrics_reject_silence_and_describe_samples_without_mutation() {
        assert_eq!(
            measure_mono(&[0.0; 64], 0, 7),
            Err(MeasurementError::SilentRender)
        );
        let samples = [-0.25, 0.5, -0.125, 0.0];
        let before = samples;
        let metrics = measure_mono(&samples, 0, 7).unwrap();
        assert_eq!(samples, before);
        assert_eq!(metrics.peak, 0.5);
        assert_eq!(metrics.maximum_jump, 0.75);
        assert_eq!(metrics.sample_hash, 7);
        assert!(metrics.active_rms > 0.0);
        assert!(metrics.active_rms_dbfs.is_finite());
        assert!(metrics.crest_factor.is_finite());
        assert!(metrics.finite);
    }

    #[test]
    fn brass_score_dc_regression_stays_inside_gate() {
        let rendered = render(ListeningPatch::BrassBass, SAMPLE_RATE).unwrap();
        let mono: Vec<f32> = rendered.samples.iter().step_by(2).copied().collect();
        let metrics = measure_mono(
            &mono,
            rendered.diagnostics.clamp_contacts,
            rendered.sample_hash,
        )
        .unwrap();
        assert!(metrics.dc.abs() <= 0.0002, "{metrics:#?}");
    }

    #[test]
    fn listening_gate_reports_all_rows_in_stable_order_and_enforces_admission() {
        let evidence = measure_listening_gate().unwrap();
        assert_eq!(
            evidence.patches.each_ref().map(|patch| patch.patch),
            ListeningPatch::ALL
        );
        assert_eq!(evidence.pair_rms.len(), 3);
        for (index, patch) in evidence.patches.iter().enumerate() {
            assert_eq!(patch.pitch_rows.len(), 3, "patch={:?}", patch.patch);
            assert_eq!(patch.alias_rows.len(), 3, "patch={:?}", patch.patch);
            assert_eq!(patch.spectral_rows.len(), 9, "patch={:?}", patch.patch);
            assert_eq!(
                patch
                    .pitch_rows
                    .iter()
                    .map(|row| row.note)
                    .collect::<Vec<_>>(),
                [36, 60, 84],
                "patch={:?}",
                patch.patch
            );
            assert_eq!(
                patch
                    .alias_rows
                    .iter()
                    .map(|row| row.note)
                    .collect::<Vec<_>>(),
                [36, 60, 84],
                "patch={:?}",
                patch.patch
            );
            assert!(patch.metrics.finite, "patch={:?}", patch.patch);
            assert!(patch.metrics.peak <= 0.95, "patch={patch:#?}");
            assert_eq!(patch.metrics.ceiling_contacts, 0, "patch={:?}", patch.patch);
            assert!(patch.metrics.dc.abs() <= 0.0002, "patch={patch:#?}");
            assert!(patch.metrics.maximum_jump <= 0.25, "patch={patch:#?}");
            assert!(patch.metrics.crest_factor.is_finite());
            assert!(
                patch
                    .spectral_rows
                    .iter()
                    .all(|row| row.harmonic_energy_db.is_finite()
                        && row.inharmonic_residual_db.is_finite())
            );
            assert_eq!(
                patch.status,
                EvidenceStatus::Pass,
                "index={index} patch={patch:#?}"
            );
        }
        assert!(
            evidence
                .pair_rms
                .iter()
                .all(|pair| pair.difference_db <= 3.0 && pair.status == EvidenceStatus::Pass)
        );
        assert_eq!(evidence.status, EvidenceStatus::Pass, "{evidence:#?}");
        assert!(evidence.errors.is_empty(), "{:#?}", evidence.errors);
    }

    #[test]
    fn engineering_sweep_covers_required_corners_with_finite_bounded_renders() {
        let rows = engineering_sweep().unwrap();
        assert!(rows.iter().any(|row| row.note == 36));
        assert!(rows.iter().any(|row| row.note == 60));
        assert!(rows.iter().any(|row| row.note == 84));
        assert!(rows.iter().any(|row| row.ratio_kind == RatioKind::Rational));
        assert!(
            rows.iter()
                .any(|row| row.ratio_kind == RatioKind::Inharmonic)
        );
        for level in [SweepLevel::Low, SweepLevel::Medium, SweepLevel::High] {
            assert!(rows.iter().any(|row| row.modulation == level));
        }
        for feedback in [SweepLevel::Low, SweepLevel::Medium, SweepLevel::High] {
            assert!(rows.iter().any(|row| row.feedback == feedback));
        }
        for stage in [
            SpectralStage::Attack,
            SpectralStage::Sustain,
            SpectralStage::Release,
        ] {
            assert!(rows.iter().any(|row| row.stage == stage));
        }
        assert!(rows.iter().all(|row| row.finite && row.peak <= 1.0));
    }
}
