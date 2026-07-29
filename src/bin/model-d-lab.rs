use moj_sint::model_d_lab::{
    AuditionKind, MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB, MODEL_D_FILTER_CUTOFF_ERROR_MAX,
    MODEL_D_FILTER_RESONANCE_RATIO_MIN, MODEL_D_FILTER_SLOPE_MAX_DB_PER_OCTAVE,
    MODEL_D_FILTER_SLOPE_MIN_DB_PER_OCTAVE, MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MAX_DB,
    MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MIN_DB, ModelDAliasEvidence, ModelDFilterEvidence,
    ModelDRender, hash_sample_stream, measure_alias_evidence, measure_filter_evidence,
    render_audition,
};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

const SAMPLE_RATE: u32 = 48_000;
const MAX_PEAK_FOR_ONE_DB_HEADROOM: f64 = 0.891_250_938_133_745_6;
const ALIAS_NOTES_AND_BOUNDS: [(u8, f64); 3] = [(36, -45.0), (60, -45.0), (84, -35.0)];

struct Audition {
    kind: AuditionKind,
    render: ModelDRender,
}

struct AliasProbe {
    note: u8,
    bound_db: f64,
    evidence: ModelDAliasEvidence,
}

struct PublishPaths {
    destination: PathBuf,
    stage: PathBuf,
    backup: PathBuf,
}

impl PublishPaths {
    fn new(destination: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file_name = destination
            .file_name()
            .filter(|name| !name.is_empty())
            .ok_or("output directory must have a final path component")?;
        let parent = destination
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let sibling = |suffix: &str| {
            let mut name = OsString::from(".");
            name.push(file_name);
            name.push(suffix);
            parent.join(name)
        };
        let paths = Self {
            destination: destination.to_owned(),
            stage: sibling(".model-d-lab-stage"),
            backup: sibling(".model-d-lab-backup"),
        };
        if path_exists(&paths.stage) {
            return Err(format!(
                "refusing pre-existing staging path {}",
                paths.stage.display()
            )
            .into());
        }
        if path_exists(&paths.backup) {
            return Err(format!(
                "refusing pre-existing backup path {}",
                paths.backup.display()
            )
            .into());
        }
        Ok(paths)
    }

    fn promote(&self, test_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
        let had_destination = path_exists(&self.destination);
        if had_destination {
            let backup_result = if test_failure_enabled(
                test_mode,
                "MOJ_SINT_MODEL_D_LAB_TEST_FAIL_BACKUP_RENAME",
            ) {
                Err(std::io::Error::other(
                    "injected destination-to-backup rename failure",
                ))
            } else {
                fs::rename(&self.destination, &self.backup)
            };
            if let Err(backup_error) = backup_result {
                return cleanup_stage_after_error(&self.stage, backup_error.to_string());
            }
        }
        if let Err(promotion_error) = fs::rename(&self.stage, &self.destination) {
            let rollback_result = if had_destination {
                fs::rename(&self.backup, &self.destination)
            } else {
                Ok(())
            };
            let cleanup_result = remove_exact_path(&self.stage);
            return match (rollback_result, cleanup_result) {
                (Ok(()), Ok(())) => Err(promotion_error.into()),
                (rollback, cleanup) => Err(format!(
                    "promotion failed: {promotion_error}; rollback: {rollback:?}; staging cleanup: {cleanup:?}"
                )
                .into()),
            };
        }
        if had_destination {
            remove_exact_path(&self.backup)?;
        }
        Ok(())
    }
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn remove_exact_path(path: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn cleanup_stage_after_error(
    stage: &Path,
    error: String,
) -> Result<(), Box<dyn std::error::Error>> {
    match remove_exact_path(stage) {
        Ok(()) => Err(error.into()),
        Err(cleanup_error) => Err(format!(
            "{error}; failed to remove staging directory {}: {cleanup_error}",
            stage.display()
        )
        .into()),
    }
}

fn test_failure_enabled(test_mode: bool, name: &str) -> bool {
    test_mode && std::env::var_os(name).is_some_and(|value| value == "1")
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [command, output] if command == "render" || command == "render-test" => {
            render_lab(Path::new(output), command == "render-test")
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

fn render_lab(output: &Path, test_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
    let publish_paths = PublishPaths::new(output)?;
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

    fs::create_dir(&publish_paths.stage)?;
    let write_result = write_batch(
        &publish_paths.stage,
        &auditions,
        &aliases,
        &filter,
        started,
        test_mode,
    );
    if let Err(write_error) = write_result {
        return cleanup_stage_after_error(&publish_paths.stage, write_error.to_string());
    }
    publish_paths.promote(test_mode)
}

fn write_batch(
    stage: &Path,
    auditions: &[Audition],
    aliases: &[AliasProbe],
    filter: &ModelDFilterEvidence,
    started: Instant,
    test_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    write_manifest(stage, auditions)?;
    write_metrics(stage, auditions)?;
    write_ablations(stage, auditions)?;
    write_oscillators(stage, auditions)?;
    write_filter(stage, filter)?;
    write_alias(stage, aliases)?;
    write_hashes(stage, auditions)?;
    write_summary(stage, auditions)?;
    write_readme(stage)?;
    for audition in auditions {
        write_wav(
            &stage.join(audition.kind.filename()),
            &audition.render.samples,
        )?;
    }
    write_cost(stage, started.elapsed().as_secs_f64(), auditions)?;
    if test_failure_enabled(
        test_mode,
        "MOJ_SINT_MODEL_D_LAB_TEST_FAIL_AFTER_COST_REPORT",
    ) {
        return Err("injected failure after staged cost report".into());
    }
    Ok(())
}

fn validate_all_evidence(
    auditions: &[Audition],
    aliases: &[AliasProbe],
    filter: &ModelDFilterEvidence,
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

    if !filter.passes() {
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
    finish_report(file)
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
    finish_report(file)
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
    finish_report(file)
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
    finish_report(file)
}

fn write_filter(output: &Path, evidence: &ModelDFilterEvidence) -> std::io::Result<()> {
    let mut file = writer(output, "filter.tsv")?;
    writeln!(file, "gate\ttarget\tmeasurement\tbound\tstatus")?;
    for probe in &evidence.cutoffs {
        writeln!(
            file,
            "cutoff_calibration_hz\t{:.6}\t{:.6}\terror<={MODEL_D_FILTER_CUTOFF_ERROR_MAX:.6}\tpass",
            probe.target_hz, probe.measured_hz
        )?;
    }
    writeln!(
        file,
        "stop_band_slope_db_per_octave\tNA\t{:.6}\t{MODEL_D_FILTER_SLOPE_MIN_DB_PER_OCTAVE:.6}..={MODEL_D_FILTER_SLOPE_MAX_DB_PER_OCTAVE:.6}\tpass",
        evidence.slope_db_per_octave
    )?;
    writeln!(
        file,
        "resonance_middle_over_low\tNA\t{:.6}\t>{MODEL_D_FILTER_RESONANCE_RATIO_MIN:.6}\tpass",
        evidence.resonance_middle_over_low
    )?;
    writeln!(
        file,
        "resonance_high_over_middle\tNA\t{:.6}\t>{MODEL_D_FILTER_RESONANCE_RATIO_MIN:.6}\tpass",
        evidence.resonance_high_over_middle
    )?;
    finish_report(file)
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
    finish_report(file)
}

fn write_hashes(output: &Path, auditions: &[Audition]) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    writeln!(
        file,
        "file\tfnv1a_interleaved_stereo_sample_hash\tfnv1a_score_hash"
    )?;
    for audition in auditions {
        writeln!(
            file,
            "{}\t{:016x}\t{:016x}",
            audition.kind.filename(),
            hash_sample_stream(&audition.render.samples),
            audition.render.metrics.score_hash
        )?;
    }
    finish_report(file)
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
    writeln!(file, "production_integration\tfalse\tpass")?;
    finish_report(file)
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
    )?;
    finish_report(file)
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
    )?;
    finish_report(file)
}

fn writer(output: &Path, name: &str) -> std::io::Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create(output.join(name))?))
}

fn finish_report(mut file: BufWriter<File>) -> std::io::Result<()> {
    file.flush()?;
    file.get_ref().sync_all()
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
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
    writer.finalize()?;
    File::open(path)?.sync_all()?;
    Ok(())
}
