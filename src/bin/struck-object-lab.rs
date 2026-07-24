use moj_sint::struck_object::{
    BASE_HZ, DURATION_MS, StruckEvidence, StruckObject, StruckRender, StruckTopology, evaluate,
    mode_specs, preview_all, render_preview, select_shared_gain,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const SAMPLE_RATE: u32 = 48_000;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [command, output] if command == "render" => render_lab(Path::new(output), SAMPLE_RATE),
        [command, output] if command == "render-test" => render_lab(Path::new(output), SAMPLE_RATE),
        _ => {
            eprintln!("Usage: struck-object-lab <render|render-test> <output-directory>");
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

fn render_lab(output: &Path, sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    let previews = preview_all(sample_rate)?;
    let gain = select_shared_gain(&previews)?;
    let renders = previews
        .iter()
        .map(|preview| render_preview(preview, gain))
        .collect::<Result<Vec<_>, _>>()?;
    let evidence = renders.iter().map(evaluate).collect::<Vec<_>>();

    write_manifest(output, &renders, &evidence)?;
    write_modes(output)?;
    write_metrics(output, &renders, &evidence)?;
    write_decay(output, &renders, &evidence)?;
    write_rejections(output, &renders, &evidence)?;
    write_hashes(output, &renders)?;
    write_summary(output, sample_rate, gain, &evidence)?;
    write_readme(output)?;
    write_cost(output, sample_rate)?;

    for (render, evidence) in renders.iter().zip(&evidence) {
        if evidence.rejection_reasons().is_empty() {
            write_wav(
                &output.join(render.topology.filename()),
                &render.samples,
                sample_rate,
            )?;
        }
    }
    let rejected = evidence
        .iter()
        .filter(|item| !item.rejection_reasons().is_empty())
        .count();
    if rejected != 0 {
        return Err(format!("{rejected} struck objects failed the engineering gate").into());
    }
    Ok(())
}

fn write_manifest(
    output: &Path,
    renders: &[StruckRender],
    evidence: &[StruckEvidence],
) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("manifest.tsv"))?);
    writeln!(
        file,
        "file\ttopology\texciter\texciter_ms\tbase_hz\tduration_ms\tshared_gain\tstatus"
    )?;
    for (render, evidence) in renders.iter().zip(evidence) {
        let status = if evidence.rejection_reasons().is_empty() {
            "pass"
        } else {
            "reject"
        };
        writeln!(
            file,
            "{}\t{}\t{}\t{}\t{BASE_HZ:.5}\t{DURATION_MS}\t{:.4}\t{status}",
            render.topology.filename(),
            render.topology.slug(),
            exciter_slug(render.topology),
            render.topology.exciter_ms(),
            render.gain
        )?;
    }
    Ok(())
}

fn write_modes(output: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("modes.tsv"))?);
    writeln!(
        file,
        "topology\tbank\tmode\tratio\tdetune\tfrequency_hz\tdecay_seconds\tcoupling\tbank_delay_ms"
    )?;
    for topology in StruckTopology::ALL {
        for mode in mode_specs(topology) {
            let delay = if mode.bank == 1 {
                topology.second_bank_delay_ms()
            } else {
                0
            };
            writeln!(
                file,
                "{}\t{}\t{}\t{:.6}\t{:.8}\t{:.5}\t{:.4}\t{:.7}\t{delay}",
                topology.slug(),
                mode.bank,
                mode.index,
                mode.ratio,
                mode.detune,
                BASE_HZ * mode.ratio * mode.detune,
                mode.decay_seconds,
                topology.coupling()
            )?;
        }
    }
    Ok(())
}

fn write_metrics(
    output: &Path,
    renders: &[StruckRender],
    evidence: &[StruckEvidence],
) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("metrics.tsv"))?);
    writeln!(
        file,
        "file\ttotal_rms\ttotal_rms_dbfs\tpeak\tdc\tmaximum_jump\tceiling_proportion\tcorrelation\tmono_loss_db\ttonal_pass_fraction\tspectral_flatness\tfinite"
    )?;
    for (render, evidence) in renders.iter().zip(evidence) {
        writeln!(
            file,
            "{}\t{:.9}\t{:.4}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.6}\t{:.6}\t{:.9}\t{}",
            render.topology.filename(),
            evidence.total_rms,
            linear_db(evidence.total_rms),
            evidence.metrics.peak,
            evidence.metrics.dc,
            evidence.metrics.maximum_jump,
            evidence.metrics.ceiling_proportion,
            evidence.metrics.correlation,
            evidence.metrics.mono_loss_db,
            evidence.tonal_pass_fraction,
            evidence.spectral_flatness,
            evidence.metrics.finite
        )?;
    }
    Ok(())
}

fn write_decay(
    output: &Path,
    renders: &[StruckRender],
    evidence: &[StruckEvidence],
) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("decay.tsv"))?);
    writeln!(
        file,
        "file\tearly_rms_50_200ms\tmiddle_rms_350_700ms\tlate_rms_1100_1450ms\tearly_to_late_db\tearly_high_ratio\tmiddle_high_ratio\treturns_to_zero"
    )?;
    for (render, evidence) in renders.iter().zip(evidence) {
        writeln!(
            file,
            "{}\t{:.9}\t{:.9}\t{:.9}\t{:.4}\t{:.9}\t{:.9}\t{}",
            render.topology.filename(),
            evidence.early_rms,
            evidence.middle_rms,
            evidence.late_rms,
            20.0 * (evidence.late_rms.max(1.0e-12) / evidence.early_rms.max(1.0e-12)).log10(),
            evidence.early_high_ratio,
            evidence.middle_high_ratio,
            evidence.returns_to_zero
        )?;
    }
    Ok(())
}

fn write_rejections(
    output: &Path,
    renders: &[StruckRender],
    evidence: &[StruckEvidence],
) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("rejections.tsv"))?);
    writeln!(file, "topology\tfile\tstatus\treasons")?;
    for (render, evidence) in renders.iter().zip(evidence) {
        let reasons = evidence.rejection_reasons();
        let status = if reasons.is_empty() { "pass" } else { "reject" };
        let reasons = reasons
            .iter()
            .map(|reason| reason.slug())
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            file,
            "{}\t{}\t{status}\t{}",
            render.topology.slug(),
            render.topology.filename(),
            if reasons.is_empty() { "none" } else { &reasons }
        )?;
    }
    Ok(())
}

fn write_hashes(output: &Path, renders: &[StruckRender]) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("hashes.tsv"))?);
    writeln!(file, "file\tfnv1a_sample_hash")?;
    for render in renders {
        writeln!(
            file,
            "{}\t{:016x}",
            render.topology.filename(),
            render.metrics.sample_hash
        )?;
    }
    Ok(())
}

fn write_summary(
    output: &Path,
    sample_rate: u32,
    gain: f32,
    evidence: &[StruckEvidence],
) -> std::io::Result<()> {
    let passed = evidence
        .iter()
        .filter(|item| item.rejection_reasons().is_empty())
        .count();
    let mut file = BufWriter::new(File::create(output.join("generation-summary.tsv"))?);
    writeln!(file, "field\tvalue")?;
    writeln!(file, "sample_rate\t{sample_rate}")?;
    writeln!(file, "duration_ms\t{DURATION_MS}")?;
    writeln!(file, "shared_gain\t{gain:.4}")?;
    writeln!(file, "candidates\t{}", evidence.len())?;
    writeln!(file, "passed\t{passed}")?;
    writeln!(file, "rejected\t{}", evidence.len() - passed)?;
    writeln!(file, "new_nonlinear_oscillator\tfalse")?;
    Ok(())
}

fn write_readme(output: &Path) -> std::io::Result<()> {
    let mut file = BufWriter::new(File::create(output.join("README.md"))?);
    writeln!(file, "# Moj Sint struck objects\n")?;
    writeln!(
        file,
        "These are three monophonic D2 objects built from a brief source-derived impulse feeding a different resonant topology. The exciter is not mixed dry. Each body creates its own natural modal decay; no sustain stage or body amplitude envelope is imposed.\n"
    )?;
    writeln!(
        file,
        "Use the numbered files as one three-way listening comparison. Start quietly, then compare attack identity, changing resonant color, and whether the tail remains alive without becoming noisy. One shared presentation gain is used, with only sparse hard crest limiting; no file is individually normalized.\n"
    )?;
    writeln!(
        file,
        "The reports are an engineering rejection gate, not musical acceptance; human listening decides whether any object is worth developing. Nothing here is routed into Engine, a preset, or a stable macro."
    )
}

fn write_cost(output: &Path, sample_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = BufWriter::new(File::create(output.join("workstation-cost.txt"))?);
    writeln!(file, "scope=isolated_scalar_struck_object")?;
    for topology in StruckTopology::ALL {
        let mut object = StruckObject::new(topology, sample_rate)?;
        let frames = object.duration_frames();
        let start = Instant::now();
        let mut accumulator = 0.0_f32;
        for _ in 0..frames {
            let frame = object.sample();
            accumulator += frame.left + frame.right;
        }
        std::hint::black_box(accumulator);
        writeln!(
            file,
            "{}_nanoseconds_per_frame={:.3}",
            topology.slug(),
            start.elapsed().as_nanos() as f64 / frames as f64
        )?;
    }
    writeln!(
        file,
        "limitation=scalar x86_64 workstation generation evidence; not callback or Raspberry Pi evidence"
    )?;
    Ok(())
}

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()
}

fn linear_db(value: f64) -> f64 {
    20.0 * value.max(1.0e-12).log10()
}

fn exciter_slug(topology: StruckTopology) -> &'static str {
    match topology {
        StruckTopology::CoupledWire => "cross-single",
        StruckTopology::SpectralPlate => "spectral-single",
        StruckTopology::DualBridge => "dual-single",
    }
}
