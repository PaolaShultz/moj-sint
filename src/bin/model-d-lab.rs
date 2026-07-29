use moj_sint::model_d::ladder::{LadderConfig, LadderMode, ModelDLadder};
use moj_sint::model_d_lab::{
    AuditionKind, MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB,
    MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MAX_DB, MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MIN_DB,
    ModelDAliasEvidence, ModelDRender, measure_alias_evidence, render_audition,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const SAMPLE_RATE: u32 = 48_000;
const MAX_PEAK_FOR_ONE_DB_HEADROOM: f64 = 0.891_250_938_133_745_6;
const ALIAS_NOTES_AND_BOUNDS: [(u8, f64); 3] = [(36, -45.0), (60, -45.0), (84, -35.0)];
const MIN_CUTOFF_HZ: f32 = 20.0;
const MAX_CUTOFF_HZ: f32 = 8_000.0;

struct Audition {
    kind: AuditionKind,
    render: ModelDRender,
}

struct AliasProbe {
    note: u8,
    bound_db: f64,
    evidence: ModelDAliasEvidence,
}

struct CutoffProbe {
    target_hz: f32,
    measured_hz: f32,
    error_fraction: f32,
}

struct FilterEvidence {
    cutoffs: Vec<CutoffProbe>,
    slope_db_per_octave: f32,
    resonance_middle_over_low: f32,
    resonance_high_over_middle: f32,
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [command, output] if command == "render" || command == "render-test" => {
            render_lab(Path::new(output))
        }
        _ => {
            eprintln!("Usage: model-d-lab <render|render-test> <output-directory>");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn render_lab(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let started = Instant::now();
    let auditions = AuditionKind::ALL
        .into_iter()
        .map(|kind| {
            Ok(Audition {
                kind,
                render: render_audition(kind, SAMPLE_RATE)?,
            })
        })
        .collect::<Result<Vec<_>, moj_sint::model_d::ModelDError>>()?;
    let aliases = ALIAS_NOTES_AND_BOUNDS
        .into_iter()
        .map(|(note, bound_db)| {
            Ok(AliasProbe {
                note,
                bound_db,
                evidence: measure_alias_evidence(note)?,
            })
        })
        .collect::<Result<Vec<_>, moj_sint::model_d::ModelDError>>()?;
    let filter = measure_filter_evidence()?;

    // Nothing from a rejected candidate may become a listening WAV.
    validate_all_evidence(&auditions, &aliases, &filter)?;

    fs::create_dir_all(output)?;
    write_manifest(output, &auditions)?;
    write_metrics(output, &auditions)?;
    write_ablations(output, &auditions)?;
    write_oscillators(output, &auditions)?;
    write_filter(output, &filter)?;
    write_alias(output, &aliases)?;
    write_hashes(output, &auditions)?;
    write_summary(output, &auditions)?;
    write_readme(output)?;
    for audition in &auditions {
        write_wav(
            &output.join(audition.kind.filename()),
            &audition.render.samples,
        )?;
    }
    write_cost(output, started.elapsed().as_secs_f64(), &auditions)?;
    Ok(())
}

fn validate_all_evidence(
    auditions: &[Audition],
    aliases: &[AliasProbe],
    filter: &FilterEvidence,
) -> Result<(), Box<dyn std::error::Error>> {
    if auditions.len() != AuditionKind::ALL.len() {
        return Err("audition set is incomplete".into());
    }
    for audition in auditions {
        let metrics = audition.render.metrics;
        if !render_passes(&audition.render) {
            return Err(format!(
                "{} failed render gates: {metrics:?}",
                audition.kind.filename()
            )
            .into());
        }
    }

    let full = &auditions[0];
    for audition in &auditions[3..] {
        let metrics = audition.render.metrics;
        if audition.render.samples.len() != full.render.samples.len()
            || metrics.presentation_gain != full.render.metrics.presentation_gain
            || metrics.score_hash != full.render.metrics.score_hash
            || metrics.ablation_residual_rms <= 1.0e-7
        {
            return Err(
                format!("{} failed matched-ablation gates", audition.kind.filename()).into(),
            );
        }
    }

    for probe in aliases {
        let evidence = &probe.evidence;
        let conservative_proxy = evidence
            .nonharmonic_foldback_proxy_db
            .max(evidence.reference_nonharmonic_floor_db);
        let floor_classification_consistent = if evidence.nonharmonic_proxy_floor_limited {
            evidence.excess_nonharmonic_foldback_proxy_db.is_none()
                && evidence.nonharmonic_foldback_proxy_db
                    < evidence.reference_nonharmonic_floor_db
                        + MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB
        } else {
            evidence.reference_nonharmonic_floor_db
                <= evidence.nonharmonic_foldback_proxy_db
                    - MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB
                && evidence
                    .excess_nonharmonic_foldback_proxy_db
                    .is_some_and(|value| value <= probe.bound_db)
        };
        if !conservative_proxy.is_finite()
            || conservative_proxy > probe.bound_db
            || !floor_classification_consistent
            || evidence.harmonic_mask_coverage > 0.15
            || evidence.harmonic_mask_half_width_bins != 4
            || !(MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MIN_DB
                ..=MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MAX_DB)
                .contains(&evidence.nonlinear_overtone_magnitude_difference_db)
            || !evidence.transfer_conflated_residual_db.is_finite()
        {
            return Err(format!("note {} failed alias evidence: {evidence:?}", probe.note).into());
        }
    }

    if filter
        .cutoffs
        .iter()
        .any(|probe| probe.error_fraction > 0.08)
        || !filter
            .cutoffs
            .windows(2)
            .all(|pair| pair[0].measured_hz < pair[1].measured_hz)
        || !(20.0..=28.0).contains(&filter.slope_db_per_octave)
        || filter.resonance_middle_over_low <= 1.05
        || filter.resonance_high_over_middle <= 1.05
    {
        return Err("filter evidence failed".into());
    }
    Ok(())
}

fn render_passes(render: &ModelDRender) -> bool {
    let metrics = render.metrics;
    metrics.finite
        && metrics.peak > 1.0e-4
        && metrics.peak <= MAX_PEAK_FOR_ONE_DB_HEADROOM
        && metrics.headroom_db >= 1.0
        && metrics.dc.abs() <= 0.005
        && metrics.maximum_jump <= 0.25
        && metrics.zero_tail_frames >= SAMPLE_RATE as usize / 2
        && metrics.pitch_hz.is_finite()
        && metrics.pitch_hz > 0.0
        && metrics.pitch_error_cents.abs() <= 10.0
}

fn write_manifest(output: &Path, auditions: &[Audition]) -> std::io::Result<()> {
    let mut file = writer(output, "manifest.tsv")?;
    writeln!(
        file,
        "file\tscore\tduration_frames\tpresentation_gain\tstatus"
    )?;
    for audition in auditions {
        writeln!(
            file,
            "{}\t{}\t{}\t{:.6}\tpass",
            audition.kind.filename(),
            audition.kind.score_name(),
            audition.kind.duration_frames_48k(),
            audition.kind.presentation_gain()
        )?;
    }
    Ok(())
}

fn write_metrics(output: &Path, auditions: &[Audition]) -> std::io::Result<()> {
    let mut file = writer(output, "metrics.tsv")?;
    writeln!(
        file,
        "file\tpeak\trms\tdc\tmaximum_jump\theadroom_db\tzero_tail_frames\tfinite\tstatus"
    )?;
    for audition in auditions {
        let metrics = audition.render.metrics;
        writeln!(
            file,
            "{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}\tpass",
            audition.kind.filename(),
            metrics.peak,
            metrics.rms,
            metrics.dc,
            metrics.maximum_jump,
            metrics.headroom_db,
            metrics.zero_tail_frames,
            metrics.finite
        )?;
    }
    Ok(())
}

fn write_ablations(output: &Path, auditions: &[Audition]) -> std::io::Result<()> {
    let mut file = writer(output, "ablations.tsv")?;
    writeln!(
        file,
        "file\treference\tmatched_frames\tmatched_gain\tmatched_score\tresidual_rms\tstatus"
    )?;
    let full = &auditions[0];
    for audition in &auditions[3..] {
        let metrics = audition.render.metrics;
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{}\t{:.6}\tpass",
            audition.kind.filename(),
            full.kind.filename(),
            audition.render.samples.len() == full.render.samples.len(),
            metrics.presentation_gain == full.render.metrics.presentation_gain,
            metrics.score_hash == full.render.metrics.score_hash,
            metrics.ablation_residual_rms
        )?;
    }
    Ok(())
}

fn write_oscillators(output: &Path, auditions: &[Audition]) -> std::io::Result<()> {
    let mut file = writer(output, "oscillators.tsv")?;
    writeln!(
        file,
        "file\tpitch_hz\tpitch_error_cents\tacceptance_bound_cents\tstatus"
    )?;
    for audition in &auditions[..3] {
        let metrics = audition.render.metrics;
        writeln!(
            file,
            "{}\t{:.6}\t{:.6}\t10.000000\tpass",
            audition.kind.filename(),
            metrics.pitch_hz,
            metrics.pitch_error_cents
        )?;
    }
    Ok(())
}

fn write_filter(output: &Path, evidence: &FilterEvidence) -> std::io::Result<()> {
    let mut file = writer(output, "filter.tsv")?;
    writeln!(file, "gate\ttarget\tmeasurement\tbound\tstatus")?;
    for probe in &evidence.cutoffs {
        writeln!(
            file,
            "cutoff_calibration_hz\t{:.6}\t{:.6}\terror<=0.080000\tpass",
            probe.target_hz, probe.measured_hz
        )?;
    }
    writeln!(
        file,
        "stop_band_slope_db_per_octave\tNA\t{:.6}\t20.000000..=28.000000\tpass",
        evidence.slope_db_per_octave
    )?;
    writeln!(
        file,
        "resonance_middle_over_low\tNA\t{:.6}\t>1.050000\tpass",
        evidence.resonance_middle_over_low
    )?;
    writeln!(
        file,
        "resonance_high_over_middle\tNA\t{:.6}\t>1.050000\tpass",
        evidence.resonance_high_over_middle
    )
}

fn write_alias(output: &Path, probes: &[AliasProbe]) -> std::io::Result<()> {
    let mut file = writer(output, "alias.tsv")?;
    writeln!(
        file,
        "note\tnative_48k_nonharmonic_out_of_mask_foldback_proxy_db\tnative_192k_nonharmonic_out_of_mask_foldback_proxy_floor_db\treference_floor_6db_classification\texcess_nonharmonic_foldback_proxy_db\tharmonic_mask_half_width_bins\tharmonic_mask_coverage\tharmonic_mask_blind_spot\traw_transfer_residual_diagnostic_only_db\tnonlinear_overtone_magnitude_difference_db\tacceptance_bound_db\tstatus"
    )?;
    for probe in probes {
        let evidence = &probe.evidence;
        let excess = evidence
            .excess_nonharmonic_foldback_proxy_db
            .map(|value| format!("{value:.6}"))
            .unwrap_or_else(|| "floor_limited".to_owned());
        writeln!(
            file,
            "{}\t{:.6}\t{:.6}\t{}\t{}\t{}\t{:.6}\tenergy_inside_masks_not_bounded\t{:.6}\t{:.6}\t{:.6}\tpass",
            probe.note,
            evidence.nonharmonic_foldback_proxy_db,
            evidence.reference_nonharmonic_floor_db,
            if evidence.nonharmonic_proxy_floor_limited {
                "floor_limited"
            } else {
                "resolved_above_floor"
            },
            excess,
            evidence.harmonic_mask_half_width_bins,
            evidence.harmonic_mask_coverage,
            evidence.transfer_conflated_residual_db,
            evidence.nonlinear_overtone_magnitude_difference_db,
            probe.bound_db
        )?;
    }
    Ok(())
}

fn write_hashes(output: &Path, auditions: &[Audition]) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    writeln!(file, "file\tfnv1a_sample_hash\tfnv1a_score_hash")?;
    for audition in auditions {
        writeln!(
            file,
            "{}\t{:016x}\t{:016x}",
            audition.kind.filename(),
            audition.render.metrics.sample_hash,
            audition.render.metrics.score_hash
        )?;
    }
    Ok(())
}

fn write_summary(output: &Path, auditions: &[Audition]) -> std::io::Result<()> {
    let mut file = writer(output, "generation-summary.tsv")?;
    let total_frames = auditions
        .iter()
        .map(|audition| audition.render.samples.len() / 2)
        .sum::<usize>();
    writeln!(file, "field\tvalue\tstatus")?;
    writeln!(file, "sample_rate\t{SAMPLE_RATE}\tpass")?;
    writeln!(file, "channels\t2\tpass")?;
    writeln!(file, "sample_format\tfloat32\tpass")?;
    writeln!(file, "wav_count\t{}\tpass", auditions.len())?;
    writeln!(file, "passing_wavs\t{}\tpass", auditions.len())?;
    writeln!(file, "total_frames\t{total_frames}\tpass")?;
    writeln!(file, "per_file_normalization\tfalse\tpass")?;
    writeln!(file, "full_band_limiter\tfalse\tpass")?;
    writeln!(file, "copied_factory_preset\tfalse\tpass")?;
    writeln!(file, "hardware_equivalence_claim\tfalse\tpass")?;
    writeln!(file, "production_integration\tfalse\tpass")
}

fn write_readme(output: &Path) -> std::io::Result<()> {
    let mut file = writer(output, "README.md")?;
    writeln!(file, "# Model D character audition\n")?;
    writeln!(
        file,
        "This disposable set contains three authored coverage phrases and four matched causal ablations. The first three files are different uses of one modeled instrument, not distinct synthesis families; human listening remains the musical acceptance gate.\n"
    )?;
    writeln!(
        file,
        "- No per-file normalization is applied; every file uses the fixed gain in `manifest.tsv`.\n- No full-band limiter, compressor, reverb, or delay is present.\n- No copied factory preset or copyrighted recording was used.\n- No hardware-equivalence claim is made.\n- No production integration, stable preset, macro mapping, JACK/ALSA work, or Raspberry Pi performance claim is made.\n"
    )?;
    writeln!(
        file,
        "Aliasing evidence uses a native 192 kHz nonharmonic out-of-mask foldback proxy floor in the same physical 0–24 kHz band, with an explicit 6 dB floor classification. The report gives harmonic-mask coverage and blind spot: energy inside masked bins is not bounded. The raw transfer residual is diagnostic only because it conflates transfer and phase differences with alias energy.\n"
    )?;
    writeln!(
        file,
        "All levels are digital measurements. Playback hardware, amplifier gain, speakers, room, and exposure remain outside this render evidence."
    )
}

fn write_cost(output: &Path, elapsed_seconds: f64, auditions: &[Audition]) -> std::io::Result<()> {
    let audio_seconds = auditions
        .iter()
        .map(|audition| audition.render.samples.len() as f64 / 2.0 / f64::from(SAMPLE_RATE))
        .sum::<f64>();
    let realtime_multiple = audio_seconds / elapsed_seconds.max(f64::MIN_POSITIVE);
    let mut file = writer(output, "workstation-cost.txt")?;
    writeln!(file, "scope=offline_model_d_lab_complete_generation")?;
    writeln!(file, "total_audio_seconds={audio_seconds:.3}")?;
    writeln!(file, "wall_seconds={elapsed_seconds:.3}")?;
    writeln!(file, "realtime_multiple={realtime_multiple:.3}")?;
    writeln!(
        file,
        "equivalent_two_second_render_seconds={:.3}",
        2.0 / realtime_multiple.max(f64::MIN_POSITIVE)
    )?;
    writeln!(
        file,
        "limitation=x86_64 workstation offline evidence; not callback or Raspberry Pi evidence"
    )
}

fn measure_filter_evidence() -> Result<FilterEvidence, moj_sint::model_d::ModelDError> {
    let targets = [125.0_f32, 500.0, 2_000.0, 8_000.0];
    let cutoffs = targets
        .into_iter()
        .map(|target_hz| {
            let measured_hz = measured_cutoff_hz(target_hz)?;
            Ok(CutoffProbe {
                target_hz,
                measured_hz,
                error_fraction: (measured_hz - target_hz).abs() / target_hz,
            })
        })
        .collect::<Result<Vec<_>, moj_sint::model_d::ModelDError>>()?;
    let lower = filter_response(250.0, 2_000.0, 0.0)?;
    let upper = filter_response(250.0, 4_000.0, 0.0)?;
    let resonance_peak = |resonance| -> Result<f32, moj_sint::model_d::ModelDError> {
        let passband = filter_response(1_000.0, 100.0, resonance)?;
        let peak = [1_000.0_f32, 1_500.0, 2_000.0, 2_300.0, 2_500.0, 3_000.0]
            .into_iter()
            .map(|frequency| filter_response(1_000.0, frequency, resonance))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .fold(0.0_f32, f32::max);
        Ok(peak / passband)
    };
    let low = resonance_peak(0.0)?;
    let middle = resonance_peak(0.45)?;
    let high = resonance_peak(0.8)?;
    Ok(FilterEvidence {
        cutoffs,
        slope_db_per_octave: 20.0 * (lower / upper).log10(),
        resonance_middle_over_low: middle / low,
        resonance_high_over_middle: high / middle,
    })
}

fn measured_cutoff_hz(target_hz: f32) -> Result<f32, moj_sint::model_d::ModelDError> {
    let threshold = core::f32::consts::FRAC_1_SQRT_2;
    let mut lower = 0.72 * target_hz;
    let mut upper = 1.28 * target_hz;
    for _ in 0..12 {
        let middle = 0.5 * (lower + upper);
        if filter_response(target_hz, middle, 0.0)? > threshold {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Ok(0.5 * (lower + upper))
}

fn filter_response(
    cutoff_hz: f32,
    frequency_hz: f32,
    resonance: f32,
) -> Result<f32, moj_sint::model_d::ModelDError> {
    let mut ladder = ModelDLadder::new(
        SAMPLE_RATE as f32,
        LadderConfig {
            resonance,
            drive: 1.0,
            mode: LadderMode::Linear,
        },
    )?;
    let cutoff = (cutoff_hz / MIN_CUTOFF_HZ).ln() / (MAX_CUTOFF_HZ / MIN_CUTOFF_HZ).ln();
    let phase_step = core::f32::consts::TAU * frequency_hz / SAMPLE_RATE as f32;
    let mut phase = 0.0_f32;
    for _ in 0..8_192 {
        ladder.sample(0.05 * phase.sin(), cutoff);
        phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
    }
    let mut input_sine = 0.0_f64;
    let mut input_cosine = 0.0_f64;
    let mut output_sine = 0.0_f64;
    let mut output_cosine = 0.0_f64;
    for _ in 0..16_384 {
        let input = 0.05 * phase.sin();
        let output = ladder.sample(input, cutoff);
        let sine = f64::from(phase.sin());
        let cosine = f64::from(phase.cos());
        input_sine += f64::from(input) * sine;
        input_cosine += f64::from(input) * cosine;
        output_sine += f64::from(output) * sine;
        output_cosine += f64::from(output) * cosine;
        phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
    }
    Ok(output_sine.hypot(output_cosine) as f32 / input_sine.hypot(input_cosine) as f32)
}

fn writer(output: &Path, name: &str) -> std::io::Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create(output.join(name))?))
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()
}
