use moj_sint::coupled_wire_thump::{
    CONFIGS, ControlledThumpVoice, LOW_CREST_END_MS, LOW_RECOVERY_END_MS, LOW_SPLIT_HZ,
    LOW_UNITY_MS, PRESENTATION_GAINS, THUMP_SPLIT_HZ, ThumpConfig, ThumpEvidence, ThumpRender,
    UPPER_STRIKE_GAIN, WET_END_MS, WET_FULL_MS, WET_HOLD_END_MS, WET_START_MS,
    evaluate_with_rate_residuals, measure_high_rate_residual,
    measure_rejected_reference_high_rate_residual, preview, render,
};
use moj_sint::hybrid_subset::measure_subset;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const SAMPLE_RATE: u32 = 48_000;
const WAV_NAME: &str = "01_coupled_wire_controlled_thump.wav";
const ALIAS_LIMIT_DB: f64 = -50.0;

struct Attempt {
    config: ThumpConfig,
    gain: f32,
    evidence: ThumpEvidence,
    reasons: String,
}

impl Attempt {
    fn passes(&self) -> bool {
        self.reasons == "none"
    }
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [command, output] if command == "render" || command == "render-test" => {
            render_lab(Path::new(output))
        }
        _ => {
            eprintln!("Usage: coupled-wire-thump-lab <render|render-test> <output-directory>");
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
    fs::create_dir_all(output)?;
    let reference_alias = measure_rejected_reference_high_rate_residual(SAMPLE_RATE)?;
    let mut attempts = Vec::new();
    let mut selected = None;

    'configs: for config in CONFIGS {
        let candidate_alias = measure_high_rate_residual(config, SAMPLE_RATE)?;
        let candidate_preview = preview(config, SAMPLE_RATE)?;
        for gain in PRESENTATION_GAINS {
            let candidate = render(&candidate_preview, gain)?;
            let evidence =
                evaluate_with_rate_residuals(&candidate, candidate_alias, reference_alias);
            let reasons = rejection_text(evidence);
            let passes = reasons == "none";
            attempts.push(Attempt {
                config,
                gain,
                evidence,
                reasons,
            });
            if passes {
                selected = Some(candidate);
                break 'configs;
            }
        }
    }

    write_selection(output, &attempts)?;
    let selected = selected.ok_or("no Coupled Wire controlled-thump candidate passed")?;
    let evidence = *attempts
        .last()
        .filter(|attempt| attempt.passes())
        .map(|attempt| &attempt.evidence)
        .ok_or("selected candidate evidence is missing")?;

    write_manifest(output, &selected)?;
    write_metrics(output, evidence)?;
    write_bands(output, evidence)?;
    write_alias(output, evidence)?;
    write_rejections(output, &selected, evidence)?;
    write_hashes(output, &selected, evidence)?;
    write_summary(output, &attempts, &selected)?;
    write_readme(output)?;
    write_cost(output, selected.config)?;

    if !evidence.rejection_reasons().is_empty() {
        return Err("selected Coupled Wire controlled-thump candidate failed re-evaluation".into());
    }
    write_wav(&output.join(WAV_NAME), &selected.samples)?;
    Ok(())
}

fn rejection_text(evidence: ThumpEvidence) -> String {
    let reasons = evidence.rejection_reasons();
    if reasons.is_empty() {
        "none".to_owned()
    } else {
        reasons
            .into_iter()
            .map(|reason| reason.slug())
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn write_manifest(output: &Path, selected: &ThumpRender) -> std::io::Result<()> {
    let mut file = writer(output, "manifest.tsv")?;
    writeln!(file, "file\tsource\tduration_ms\tconfig\tgain\tstatus")?;
    writeln!(
        file,
        "{WAV_NAME}\tcoupled_wire_unpresented\t1600\t{}\t{:.2}\tpass",
        selected.config.slug, selected.gain
    )
}

fn write_metrics(output: &Path, evidence: ThumpEvidence) -> std::io::Result<()> {
    let mut file = writer(output, "metrics.tsv")?;
    writeln!(
        file,
        "file\ttotal_rms_dbfs\tsample_peak_dbfs\ttrue_peak_dbfs\tpeak_linear\tdc\tmaximum_jump\tceiling_proportion\tcorrelation\tmono_loss_db\ttonal_pass_fraction\tspectral_flatness\tfinite\treturns_to_zero"
    )?;
    writeln!(
        file,
        "{WAV_NAME}\t{:.6}\t{:.6}\t{:.6}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.6}\t{:.9}\t{}\t{}",
        evidence.total_rms_dbfs,
        evidence.sample_peak_dbfs,
        evidence.true_peak_dbfs,
        evidence.metrics.peak,
        evidence.metrics.dc,
        evidence.metrics.maximum_jump,
        evidence.metrics.ceiling_proportion,
        evidence.metrics.correlation,
        evidence.metrics.mono_loss_db,
        evidence.tonal_pass_fraction,
        evidence.spectral_flatness,
        evidence.metrics.finite,
        evidence.returns_to_zero
    )
}

fn write_bands(output: &Path, evidence: ThumpEvidence) -> std::io::Result<()> {
    let mut file = writer(output, "bands.tsv")?;
    writeln!(file, "field\tvalue\tunit\tbound")?;
    writeln!(
        file,
        "low_onset_reduction\t{:.6}\tdB\t-4.0..=-1.5",
        evidence.low_onset_reduction_db
    )?;
    writeln!(
        file,
        "low_nonlinear_residual\t{:.6}\tdB\t<=-40.0",
        evidence.low_nonlinear_residual_db
    )?;
    writeln!(
        file,
        "thump_residual\t{:.6}\tdB\t-30.0..=-12.0",
        evidence.thump_residual_db
    )?;
    writeln!(file, "low_split\t{LOW_SPLIT_HZ:.1}\tHz\tlinear_below")?;
    writeln!(file, "thump_split\t{THUMP_SPLIT_HZ:.1}\tHz\tclean_above")?;
    writeln!(
        file,
        "wet_window\t{WET_START_MS}/{WET_FULL_MS}/{WET_HOLD_END_MS}/{WET_END_MS}\tms\tstart/full/hold/end"
    )?;
    writeln!(
        file,
        "low_contour\t8/12/{LOW_CREST_END_MS}/{LOW_UNITY_MS}\tms\tstrike/fall/hold/unity"
    )?;
    writeln!(
        file,
        "upper_contour\t{UPPER_STRIKE_GAIN:.2}/80/{LOW_RECOVERY_END_MS}\tgain/ms/ms\tstrike/hold/unity"
    )
}

fn write_alias(output: &Path, evidence: ThumpEvidence) -> std::io::Result<()> {
    let mut file = writer(output, "alias.tsv")?;
    writeln!(
        file,
        "path\tresidual_db_relative_to_probe_input\tacceptance_bound_db\trole"
    )?;
    writeln!(
        file,
        "candidate\t{:.6}\t{ALIAS_LIMIT_DB:.1}\tacceptance",
        evidence.high_rate_residual_db
    )?;
    writeln!(
        file,
        "rejected_full_band_clamp\t{:.6}\tNA\tcomparison_only",
        evidence.reference_high_rate_residual_db
    )
}

fn write_selection(output: &Path, attempts: &[Attempt]) -> std::io::Result<()> {
    let mut file = writer(output, "selection.tsv")?;
    writeln!(
        file,
        "attempt\tconfig\tgain\ttotal_rms_dbfs\tsample_peak_dbfs\ttrue_peak_dbfs\tlow_onset_reduction_db\talias_residual_db\tstatus\treasons"
    )?;
    for (index, attempt) in attempts.iter().enumerate() {
        writeln!(
            file,
            "{}\t{}\t{:.2}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}\t{}",
            index + 1,
            attempt.config.slug,
            attempt.gain,
            attempt.evidence.total_rms_dbfs,
            attempt.evidence.sample_peak_dbfs,
            attempt.evidence.true_peak_dbfs,
            attempt.evidence.low_onset_reduction_db,
            attempt.evidence.high_rate_residual_db,
            if attempt.passes() { "pass" } else { "reject" },
            attempt.reasons
        )?;
    }
    Ok(())
}

fn write_rejections(
    output: &Path,
    selected: &ThumpRender,
    evidence: ThumpEvidence,
) -> std::io::Result<()> {
    let mut file = writer(output, "rejections.tsv")?;
    writeln!(file, "config\tfile\tstatus\treasons")?;
    writeln!(
        file,
        "{}\t{WAV_NAME}\t{}\t{}",
        selected.config.slug,
        if evidence.rejection_reasons().is_empty() {
            "pass"
        } else {
            "reject"
        },
        rejection_text(evidence)
    )
}

fn write_hashes(
    output: &Path,
    selected: &ThumpRender,
    evidence: ThumpEvidence,
) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    let source = measure_subset(&selected.source, 0, selected.source.len() / 2);
    writeln!(file, "material\tfile\tfnv1a_sample_hash")?;
    writeln!(file, "unpresented_source\tNA\t{:016x}", source.sample_hash)?;
    writeln!(
        file,
        "selected_output\t{WAV_NAME}\t{:016x}",
        evidence.metrics.sample_hash
    )
}

fn write_summary(
    output: &Path,
    attempts: &[Attempt],
    selected: &ThumpRender,
) -> std::io::Result<()> {
    let mut file = writer(output, "generation-summary.tsv")?;
    writeln!(file, "field\tvalue")?;
    writeln!(file, "sample_rate\t{SAMPLE_RATE}")?;
    writeln!(file, "candidates_presented\t1")?;
    writeln!(file, "passing_wavs\t1")?;
    writeln!(file, "selection_attempts\t{}", attempts.len())?;
    writeln!(file, "selected_config\t{}", selected.config.slug)?;
    writeln!(file, "selected_gain\t{:.2}", selected.gain)?;
    writeln!(file, "full_band_limiter\tfalse")?;
    writeln!(file, "production_integration\tfalse")
}

fn write_readme(output: &Path) -> std::io::Result<()> {
    let mut file = writer(output, "README.md")?;
    writeln!(file, "# Coupled Wire controlled thump\n")?;
    writeln!(
        file,
        "This directory contains one listening file: `{WAV_NAME}`. It keeps the accepted 1.6-second Coupled Wire source and changes only its deterministic presentation.\n"
    )?;
    writeln!(
        file,
        "The centered sub and fundamental remain linear. A short, bounded 105-500 Hz nonlinear residual supplies the hard crest; material above 500 Hz remains clean. There is no final full-band limiter, compressor, normalization pass, reverb, delay, or chord layer.\n"
    )?;
    writeln!(
        file,
        "The reports reject weak, noise-like, aliased, low-band-distorted, over-peak, mono-unsafe, non-finite, or non-decaying output. Human listening remains the acceptance gate.\n"
    )?;
    writeln!(
        file,
        "All levels here are digital dBFS/dBTP measurements. The external playback chain—line output, amplifier, loudspeaker, room, and listening level—can still overload independently."
    )
}

fn write_cost(output: &Path, config: ThumpConfig) -> Result<(), Box<dyn std::error::Error>> {
    let mut voice = ControlledThumpVoice::new(config, SAMPLE_RATE)?;
    let frames = voice.duration_frames();
    let start = Instant::now();
    let mut accumulator = 0.0_f32;
    for _ in 0..frames {
        let frame = voice.sample();
        accumulator += frame.left + frame.right;
    }
    std::hint::black_box(accumulator);
    let mut file = writer(output, "workstation-cost.txt")?;
    writeln!(file, "scope=isolated_scalar_controlled_thump_voice")?;
    writeln!(
        file,
        "nanoseconds_per_frame={:.3}",
        start.elapsed().as_nanos() as f64 / frames as f64
    )?;
    writeln!(
        file,
        "limitation=scalar x86_64 workstation generation evidence; not callback or Raspberry Pi evidence"
    )?;
    Ok(())
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
