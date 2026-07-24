use moj_sint::envelope_audition::{
    BODY_ATTACK_MS, BODY_FAST_DECAY_MS, BODY_SILENT_FROM_MS, DURATION_MS, EnvelopeEvidence,
    EnvelopeRender, LAYER_OFFSETS_MS, MIN_TOTAL_RMS, MONOPHONIC_LAYERS, STRIKE_ATTACK_MS,
    STRIKE_MIX, evaluate_render, preview_all, render_profile, select_shared_gain,
};
use moj_sint::hybrid_subset::classify_noise_like;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, output] if command == "render" => finish(render_lab(Path::new(output), 48_000)),
        [command, output] if command == "render-test" => {
            finish(render_lab(Path::new(output), 4_000))
        }
        _ => {
            eprintln!("Usage: hybrid-subset-lab <render|render-test> <output-directory>");
            ExitCode::FAILURE
        }
    }
}

fn finish(result: Result<(), Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

struct AuditionRow {
    number: usize,
    render: EnvelopeRender,
    evidence: EnvelopeEvidence,
}

impl AuditionRow {
    fn filename(&self) -> &'static str {
        self.render.profile.filename()
    }

    fn passing(&self) -> bool {
        self.evidence.rejection_reasons().is_empty()
    }
}

fn render_lab(output: &Path, sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    let started = Instant::now();
    let previews = preview_all(sample_rate)?;
    let shared_gain = select_shared_gain(&previews)?;
    let mut rows = Vec::with_capacity(previews.len());
    for (index, preview) in previews.iter().enumerate() {
        let render = render_profile(preview, shared_gain)?;
        let evidence = evaluate_render(&render);
        if evidence.rejection_reasons().is_empty() {
            write_wav(
                &output.join(render.profile.filename()),
                sample_rate,
                &render.samples,
            )?;
        }
        rows.push(AuditionRow {
            number: index + 1,
            render,
            evidence,
        });
    }

    write_manifest(output, shared_gain, &rows)?;
    write_metrics(output, &rows)?;
    write_rejections(output, &rows)?;
    write_hashes(output, &rows)?;
    write_generation_summary(output, sample_rate, shared_gain, &rows)?;
    write_readme(output, sample_rate, shared_gain, &rows)?;
    write_cost(output, started.elapsed().as_secs_f64())?;

    println!(
        "wrote piano strike envelope audition at {} Hz to {}",
        sample_rate,
        output.display()
    );
    Ok(())
}

fn write_manifest(output: &Path, shared_gain: f32, rows: &[AuditionRow]) -> std::io::Result<()> {
    let mut file = writer(output, "manifest.tsv")?;
    writeln!(
        file,
        "number\tprofile\tfilename\tbody_layers\toffsets_ms\tstrike_source\tstrike_ms\tstrike_attack_ms\tstrike_mix\tbody_attack_ms\tbody_fast_decay_ms\tbody_silent_from_ms\tduration_ms\tshared_gain\ttotal_rms_dbfs\tstatus"
    )?;
    let layers = MONOPHONIC_LAYERS
        .iter()
        .map(|layer| layer.label())
        .collect::<Vec<_>>()
        .join(",");
    let offsets = LAYER_OFFSETS_MS
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    for row in rows {
        writeln!(
            file,
            "{:02}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{}",
            row.number,
            row.render.profile.slug(),
            row.filename(),
            layers,
            offsets,
            row.render.profile.strike_layer().label(),
            row.render.profile.strike_ms(),
            STRIKE_ATTACK_MS,
            STRIKE_MIX,
            BODY_ATTACK_MS,
            BODY_FAST_DECAY_MS,
            BODY_SILENT_FROM_MS,
            DURATION_MS,
            shared_gain,
            dbfs(row.render.total_rms),
            if row.passing() { "present" } else { "rejected" }
        )?;
    }
    Ok(())
}

fn write_metrics(output: &Path, rows: &[AuditionRow]) -> std::io::Result<()> {
    let mut file = writer(output, "metrics.tsv")?;
    writeln!(
        file,
        "profile\ttotal_rms\ttotal_rms_dbfs\tactive_rms\tactive_rms_dbfs\tpeak\tdc\tmaximum_jump\tcorrelation\tmono_loss_db\tceiling_proportion\ttonal_pass_fraction\tspectral_flatness\tnoise_like\treturns_to_zero\tfinite"
    )?;
    for row in rows {
        let metrics = row.render.metrics;
        writeln!(
            file,
            "{}\t{:.9}\t{:.6}\t{:.9}\t{:.6}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.9}\t{:.9}\t{:.9}\t{}\t{}\t{}",
            row.render.profile.slug(),
            row.render.total_rms,
            dbfs(row.render.total_rms),
            metrics.active_rms,
            dbfs(metrics.active_rms),
            metrics.peak,
            metrics.dc,
            metrics.maximum_jump,
            metrics.correlation,
            metrics.mono_loss_db,
            metrics.ceiling_proportion,
            row.evidence.tonal_pass_fraction,
            row.evidence.spectral_flatness,
            classify_noise_like(
                row.evidence.spectral_flatness,
                row.evidence.tonal_pass_fraction,
            ),
            row.evidence.returns_to_zero,
            metrics.finite,
        )?;
    }
    Ok(())
}

fn write_rejections(output: &Path, rows: &[AuditionRow]) -> std::io::Result<()> {
    let mut file = writer(output, "rejections.tsv")?;
    writeln!(file, "profile\tfilename\tstatus\treasons")?;
    for row in rows {
        let reasons = row.evidence.rejection_reasons();
        writeln!(
            file,
            "{}\t{}\t{}\t{}",
            row.render.profile.slug(),
            row.filename(),
            if reasons.is_empty() { "pass" } else { "reject" },
            if reasons.is_empty() {
                "none".to_owned()
            } else {
                reasons
                    .iter()
                    .map(|reason| reason.slug())
                    .collect::<Vec<_>>()
                    .join(",")
            }
        )?;
    }
    Ok(())
}

fn write_hashes(output: &Path, rows: &[AuditionRow]) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    writeln!(file, "profile\tsample_hash")?;
    for row in rows {
        writeln!(
            file,
            "{}\t{:016x}",
            row.render.profile.slug(),
            row.render.metrics.sample_hash
        )?;
    }
    Ok(())
}

fn write_generation_summary(
    output: &Path,
    sample_rate: u32,
    shared_gain: f32,
    rows: &[AuditionRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "generation-summary.tsv")?;
    writeln!(file, "metric\tvalue\tunit")?;
    writeln!(file, "sample_rate\t{sample_rate}\thz")?;
    writeln!(file, "profile_count\t{}\tcount", rows.len())?;
    writeln!(
        file,
        "passing_profile_count\t{}\tcount",
        rows.iter().filter(|row| row.passing()).count()
    )?;
    writeln!(
        file,
        "rejected_profile_count\t{}\tcount",
        rows.iter().filter(|row| !row.passing()).count()
    )?;
    writeln!(file, "shared_gain\t{shared_gain:.6}\tlinear")?;
    writeln!(file, "minimum_total_rms\t{MIN_TOTAL_RMS:.9}\tlinear")?;
    writeln!(file, "minimum_total_rms_dbfs\t-14.000000\tdbfs")?;
    writeln!(file, "duration\t{DURATION_MS}\tms")?;
    writeln!(file, "body_attack\t{BODY_ATTACK_MS}\tms")?;
    writeln!(file, "body_fast_decay\t{BODY_FAST_DECAY_MS}\tms")?;
    writeln!(file, "body_silent_from\t{BODY_SILENT_FROM_MS}\tms")?;
    writeln!(file, "strike_attack\t{STRIKE_ATTACK_MS}\tms")?;
    writeln!(file, "strike_mix\t{STRIKE_MIX:.2}\tlinear")?;
    writeln!(file, "output_ceiling\t-0.300000\tdbfs")
}

fn write_readme(
    output: &Path,
    sample_rate: u32,
    shared_gain: f32,
    rows: &[AuditionRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "README.md")?;
    writeln!(file, "# Moj Sint piano-strike envelope comparison\n")?;
    writeln!(
        file,
        "**Please start with playback volume low.** Digital level is not acoustic SPL.\n"
    )?;
    writeln!(
        file,
        "Every file keeps the same piano-like one-shot body: the exact Cross, Spectral, and Dual single-note D2 layers summed with fixed 0/2/5 ms offsets. The result is musically monophonic while retaining its two-channel stereo construction.\n"
    )?;
    writeln!(
        file,
        "The body has a 2 ms rise, 60 ms fast decay, a curved fade to zero at 700 ms, and no flat sustain. Each 800 ms file varies only the brief pitched D2 strike source, mixed at 0.30 before shared gain {:.6} and a static -0.3 dBFS ceiling. There is no individual normalization. Any result below -14 dBFS whole-file RMS receives no WAV.\n",
        shared_gain
    )?;
    writeln!(file, "Listen in this order:\n")?;
    for row in rows.iter().filter(|row| row.passing()) {
        writeln!(
            file,
            "{}. `{}` — {}, exact {} source, {} ms strike; whole-file RMS {:.3} dBFS, active RMS {:.3} dBFS, ceiling {:.3}%.",
            row.number,
            row.filename(),
            row.render.profile.label(),
            row.render.profile.strike_layer().label(),
            row.render.profile.strike_ms(),
            dbfs(row.render.total_rms),
            dbfs(row.render.metrics.active_rms),
            100.0 * row.render.metrics.ceiling_proportion,
        )?;
    }
    writeln!(
        file,
        "\nRejected profiles have no WAV and are listed in `rejections.tsv`. The body is fixed; the exact-source strike mechanism changes. Automated checks discard obvious failures; human listening decides whether any strike behavior is useful.\n"
    )?;
    writeln!(
        file,
        "This batch is {sample_rate} Hz. Workstation generation is not Raspberry Pi callback, latency, polyphony, or sound-quality evidence."
    )
}

fn dbfs(rms: f64) -> f64 {
    20.0 * rms.max(1.0e-12).log10()
}

fn writer(directory: &Path, name: &str) -> std::io::Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create(directory.join(name))?))
}

fn write_wav(path: &Path, sample_rate: u32, samples: &[f32]) -> Result<(), hound::Error> {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()
}

fn write_cost(output: &Path, seconds: f64) -> std::io::Result<()> {
    let mut file = writer(output, "workstation-cost.txt")?;
    writeln!(file, "scope=complete_offline_generation")?;
    writeln!(file, "wall_clock_seconds={seconds:.6}")?;
    writeln!(
        file,
        "limitation=x86_64 workstation generation only; not Raspberry Pi callback latency polyphony or memory evidence"
    )
}
