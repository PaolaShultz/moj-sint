use moj_sint::hybrid_subset::{
    CandidateEvidence, GroupGains, MASTER_ATTACK_MS, MASTER_DECAY_MS, MASTER_RELEASE_MS,
    MASTER_SUSTAIN, REFERENCE_OFFSETS_MS, ReferenceRender, SubsetRender, evaluate_candidate,
    evaluate_reference, fixed_offsets_ms, measure_high_rate_residual,
    measure_reference_high_rate_residual, preview_all, render_candidate, render_reference,
    select_group_gains,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, output] if command == "render" => {
            finish(render_lab(Path::new(output), 48_000, false))
        }
        [command, output] if command == "render-test" => {
            finish(render_lab(Path::new(output), 4_000, true))
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

struct CandidateRow {
    number: usize,
    filename: String,
    render: SubsetRender,
    evidence: CandidateEvidence,
}

impl CandidateRow {
    fn passing(&self) -> bool {
        self.evidence.rejection_reasons().is_empty()
    }
}

fn render_lab(
    output: &Path,
    sample_rate: u32,
    quick: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    let started = Instant::now();
    let previews = preview_all(sample_rate)?;
    let gains = select_group_gains(&previews)?;
    let reference = render_reference(sample_rate)?;
    let reference_residual = measure_reference_high_rate_residual(
        sample_rate,
        reference.gain,
        if quick { 512 } else { 2_048 },
    )?;
    let reference_evidence = evaluate_reference(&reference, sample_rate, reference_residual);
    let reference_filename = "01_reference_fixed-microdelay-master-envelope.wav";
    if reference_evidence.rejection_reasons().is_empty() {
        write_wav(
            &output.join(reference_filename),
            sample_rate,
            &reference.samples,
        )?;
    }

    let mut rows = Vec::with_capacity(previews.len());
    for (index, preview) in previews.iter().enumerate() {
        let gain = gains.for_candidate(preview.candidate);
        let render = render_candidate(preview, gain)?;
        let residual = measure_high_rate_residual(
            preview.candidate,
            sample_rate,
            gain,
            if quick { 512 } else { 2_048 },
        )?;
        let evidence = evaluate_candidate(&render, sample_rate, residual);
        let filename = format!("{:02}_{}.wav", index + 2, preview.candidate.slug());
        if evidence.rejection_reasons().is_empty() {
            write_wav(&output.join(&filename), sample_rate, &render.samples)?;
        }
        rows.push(CandidateRow {
            number: index + 2,
            filename,
            render,
            evidence,
        });
    }

    write_manifest(
        output,
        gains,
        &reference,
        reference_filename,
        &reference_evidence,
        &rows,
    )?;
    write_metrics(output, &reference, &rows)?;
    write_tonal(output, &reference_evidence, &rows)?;
    write_residual(output, &reference_evidence, &rows)?;
    write_rejections(output, reference_filename, &reference_evidence, &rows)?;
    write_hashes(output, &reference, &rows)?;
    write_composite_regression(output, sample_rate, reference.raw_hash)?;
    write_generation_summary(output, sample_rate, gains, &rows)?;
    write_readme(output, sample_rate, gains, &reference, &rows)?;
    write_cost(output, started.elapsed().as_secs_f64())?;

    println!(
        "wrote coherent hybrid composite lab at {} Hz to {}",
        sample_rate,
        output.display()
    );
    Ok(())
}

fn write_manifest(
    output: &Path,
    gains: GroupGains,
    reference: &ReferenceRender,
    reference_filename: &str,
    reference_evidence: &CandidateEvidence,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "manifest.tsv")?;
    writeln!(
        file,
        "number\tcandidate\tfilename\tlayer_count\tlayers\tfixed_offsets_ms\tshared_gain\tstatus"
    )?;
    let reference_layers = moj_sint::composite_machine::OriginalLayer::ALL
        .iter()
        .map(|layer| layer.label())
        .collect::<Vec<_>>()
        .join(",");
    let reference_offsets = REFERENCE_OFFSETS_MS
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    writeln!(
        file,
        "01\ttwelve-layer-reference\t{}\t12\t{}\t{}\t{:.6}\t{}",
        reference_filename,
        reference_layers,
        reference_offsets,
        reference.gain,
        if reference_evidence.rejection_reasons().is_empty() {
            "present"
        } else {
            "rejected"
        }
    )?;
    for row in rows {
        let candidate = row.render.candidate;
        let layers = candidate
            .layers()
            .iter()
            .map(|layer| layer.label())
            .collect::<Vec<_>>()
            .join(",");
        let offsets = fixed_offsets_ms(candidate)
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            file,
            "{:02}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{}",
            row.number,
            candidate.slug(),
            row.filename,
            candidate.layers().len(),
            layers,
            offsets,
            gains.for_candidate(candidate),
            if row.passing() { "present" } else { "rejected" }
        )?;
    }
    Ok(())
}

fn write_metrics(
    output: &Path,
    reference: &ReferenceRender,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "metrics.tsv")?;
    writeln!(
        file,
        "candidate\tgain\tpeak\tactive_rms\tactive_rms_dbfs\tdc\tmaximum_jump\tcorrelation\tmono_loss_db\tceiling_proportion\tfinite"
    )?;
    write_metric_row(
        &mut file,
        "twelve-layer-fixed-microdelay-reference",
        reference.gain,
        reference.metrics,
    )?;
    for row in rows {
        write_metric_row(
            &mut file,
            row.render.candidate.slug(),
            row.render.gain,
            row.render.metrics,
        )?;
    }
    Ok(())
}

fn write_metric_row(
    file: &mut impl Write,
    name: &str,
    gain: f32,
    metrics: moj_sint::hybrid_subset::SubsetMetrics,
) -> std::io::Result<()> {
    writeln!(
        file,
        "{}\t{:.6}\t{:.9}\t{:.9}\t{:.6}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.9}\t{}",
        name,
        gain,
        metrics.peak,
        metrics.active_rms,
        20.0 * metrics.active_rms.max(1.0e-12).log10(),
        metrics.dc,
        metrics.maximum_jump,
        metrics.correlation,
        metrics.mono_loss_db,
        metrics.ceiling_proportion,
        metrics.finite,
    )
}

fn write_tonal(
    output: &Path,
    reference: &CandidateEvidence,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "tonal.tsv")?;
    writeln!(
        file,
        "candidate\ttonal_pass_fraction\tspectral_flatness\tnoise_like"
    )?;
    write_tonal_row(&mut file, "twelve-layer-reference", reference)?;
    for row in rows {
        write_tonal_row(&mut file, row.render.candidate.slug(), &row.evidence)?;
    }
    Ok(())
}

fn write_tonal_row(
    file: &mut impl Write,
    candidate: &str,
    evidence: &CandidateEvidence,
) -> std::io::Result<()> {
    writeln!(
        file,
        "{}\t{:.9}\t{:.9}\t{}",
        candidate,
        evidence.tonal_pass_fraction,
        evidence.spectral_flatness,
        moj_sint::hybrid_subset::classify_noise_like(
            evidence.spectral_flatness,
            evidence.tonal_pass_fraction,
        )
    )
}

fn write_residual(
    output: &Path,
    reference: &CandidateEvidence,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "residual.tsv")?;
    writeln!(
        file,
        "candidate\tfactor\tpost_sum_ceiling_residual_db\tlimit_db\tstatus"
    )?;
    write_residual_row(&mut file, "twelve-layer-reference", reference)?;
    for row in rows {
        write_residual_row(&mut file, row.render.candidate.slug(), &row.evidence)?;
    }
    Ok(())
}

fn write_residual_row(
    file: &mut impl Write,
    candidate: &str,
    evidence: &CandidateEvidence,
) -> std::io::Result<()> {
    writeln!(
        file,
        "{}\t8\t{:.9}\t-1.000000\t{}",
        candidate,
        evidence.high_rate_residual_db,
        if evidence.high_rate_residual_db < -1.0 {
            "pass"
        } else {
            "reject"
        }
    )
}

fn write_rejections(
    output: &Path,
    reference_filename: &str,
    reference: &CandidateEvidence,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "rejections.tsv")?;
    writeln!(file, "candidate\tfilename\tstatus\treasons")?;
    write_rejection_row(
        &mut file,
        "twelve-layer-reference",
        reference_filename,
        reference,
    )?;
    for row in rows {
        write_rejection_row(
            &mut file,
            row.render.candidate.slug(),
            &row.filename,
            &row.evidence,
        )?;
    }
    Ok(())
}

fn write_rejection_row(
    file: &mut impl Write,
    candidate: &str,
    filename: &str,
    evidence: &CandidateEvidence,
) -> std::io::Result<()> {
    let reasons = evidence.rejection_reasons();
    writeln!(
        file,
        "{}\t{}\t{}\t{}",
        candidate,
        filename,
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
    )
}

fn write_hashes(
    output: &Path,
    reference: &ReferenceRender,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "hashes.tsv")?;
    writeln!(file, "candidate\traw_or_output\tsample_hash")?;
    writeln!(
        file,
        "twelve-layer-fixed-microdelay-reference\traw\t{:016x}",
        reference.raw_hash
    )?;
    writeln!(
        file,
        "twelve-layer-fixed-microdelay-reference\toutput\t{:016x}",
        reference.metrics.sample_hash
    )?;
    for row in rows {
        writeln!(
            file,
            "{}\toutput\t{:016x}",
            row.render.candidate.slug(),
            row.render.metrics.sample_hash
        )?;
    }
    Ok(())
}

fn write_composite_regression(
    output: &Path,
    sample_rate: u32,
    reference_hash: u64,
) -> std::io::Result<()> {
    let mut file = writer(output, "composite-regression.tsv")?;
    writeln!(
        file,
        "candidate\tsample_rate\traw_hash\ttiming_model\tmaster_envelope"
    )?;
    writeln!(
        file,
        "twelve-layer-reference\t{}\t{:016x}\tfixed_0_to_15_ms\t25_180_0.88_320_ms",
        sample_rate, reference_hash
    )
}

fn write_generation_summary(
    output: &Path,
    sample_rate: u32,
    gains: GroupGains,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "generation-summary.tsv")?;
    writeln!(file, "metric\tvalue\tunit")?;
    writeln!(file, "sample_rate\t{sample_rate}\thz")?;
    writeln!(file, "candidate_count\t{}\tcount", rows.len())?;
    writeln!(
        file,
        "passing_candidate_count\t{}\tcount",
        rows.iter().filter(|row| row.passing()).count()
    )?;
    writeln!(
        file,
        "rejected_candidate_count\t{}\tcount",
        rows.iter().filter(|row| !row.passing()).count()
    )?;
    writeln!(
        file,
        "three_layer_shared_gain\t{:.6}\tlinear",
        gains.three_layer
    )?;
    writeln!(
        file,
        "four_layer_shared_gain\t{:.6}\tlinear",
        gains.four_layer
    )?;
    writeln!(file, "three_layer_offsets\t0,2,5\tms")?;
    writeln!(file, "four_layer_offsets\t0,4,9,15\tms")?;
    writeln!(file, "reference_offsets\t0,1,3,4,5,7,8,9,11,12,14,15\tms")?;
    writeln!(file, "master_attack\t{MASTER_ATTACK_MS}\tms")?;
    writeln!(file, "master_decay\t{MASTER_DECAY_MS}\tms")?;
    writeln!(file, "master_sustain\t{MASTER_SUSTAIN:.2}\tlinear")?;
    writeln!(file, "master_release\t{MASTER_RELEASE_MS}\tms")?;
    writeln!(file, "output_ceiling\t0.96605086\tfloating_sample")?;
    writeln!(file, "maximum_ceiling_proportion\t0.010000\tfraction")
}

fn write_readme(
    output: &Path,
    sample_rate: u32,
    gains: GroupGains,
    reference: &ReferenceRender,
    rows: &[CandidateRow],
) -> std::io::Result<()> {
    let mut file = writer(output, "README.md")?;
    writeln!(file, "# Moj Sint coherent hybrid composites\n")?;
    writeln!(
        file,
        "**Please start with playback volume low.** Digital level is not acoustic SPL.\n"
    )?;
    writeln!(
        file,
        "Every candidate combines three or four exact original hybrid layers as one complete sound. The layers use fixed micro-delays of 0/2/5 ms or 0/4/9/15 ms, then the complete stereo sum follows one shared master ADSR (25 ms attack, 180 ms decay, 0.88 sustain, 320 ms release). There are no new source mechanisms, effects, compressors, soft saturators, or per-file normalization. A fixed shared group gain feeds a static -0.3 dBFS ceiling; no retained file exceeds 1% crest contact.\n"
    )?;
    writeln!(file, "Listen in this order:\n")?;
    writeln!(
        file,
        "1. `01_reference_fixed-microdelay-master-envelope.wav` — all twelve exact layers spread only from 0 to 15 ms under the same master envelope; gain {:.6}, active RMS {:.6}, ceiling {:.3}%.\n",
        reference.gain,
        reference.metrics.active_rms,
        100.0 * reference.metrics.ceiling_proportion,
    )?;
    for (position, row) in (2..).zip(rows.iter().filter(|row| row.passing())) {
        let layers = row
            .render
            .candidate
            .layers()
            .iter()
            .map(|layer| layer.label())
            .collect::<Vec<_>>()
            .join(" + ");
        writeln!(
            file,
            "{}. `{}` — {}; gain {:.6}, active RMS {:.6}, ceiling {:.3}%.",
            position,
            row.filename,
            layers,
            row.render.gain,
            row.render.metrics.active_rms,
            100.0 * row.render.metrics.ceiling_proportion,
        )?;
    }
    writeln!(
        file,
        "\nShared gains: three-layer {:.6}; four-layer {:.6}. Rejected designs have no WAV and are listed in `rejections.tsv`.\n",
        gains.three_layer, gains.four_layer
    )?;
    writeln!(
        file,
        "Automated checks discard level, crest, tonal-anchor, noise-like, residual, DC, discontinuity, stereo/mono, non-finite, and tail failures. They do not establish musical quality; human listening decides whether any smaller combination retains the fused identity of the twelve-layer reference.\n"
    )?;
    writeln!(
        file,
        "This batch is {sample_rate} Hz. Workstation generation is not Raspberry Pi callback, latency, polyphony, or sound-quality evidence."
    )
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
