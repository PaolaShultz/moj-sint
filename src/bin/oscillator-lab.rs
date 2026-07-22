use moj_sint::analysis::{AnalysisSpec, WaveformMetrics, measure_candidate};
use moj_sint::control::Normalized;
use moj_sint::dsp::oscillator::OscillatorMethod;
use moj_sint::offline::{RenderSpec, render_note};
use moj_sint::preset::Preset;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

const SAMPLE_RATE: f32 = 48_000.0;
const ANALYSIS_CYCLES: usize = 32;
const NOTES: [u8; 4] = [36, 60, 84, 96];
const SHAPES: [f32; 3] = [0.0, 0.5, 1.0];

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, output] if command == "compare" => match compare(Path::new(output)) {
            Ok(()) => {
                println!("wrote oscillator comparison to {output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        [command, output] if command == "render" => {
            match render_listening_matrix(Path::new(output)) {
                Ok(()) => {
                    println!("wrote listening matrix to {output}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!(
                "Usage:\n  oscillator-lab compare <output-directory>\n  oscillator-lab render <output-directory>"
            );
            ExitCode::FAILURE
        }
    }
}

fn render_listening_matrix(output_directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_directory)?;
    let base_preset = Preset::parse(include_str!("../../presets/reference.mojsint"))?;
    let manifest_path = output_directory.join("listening-manifest.tsv");
    let mut manifest = BufWriter::new(File::create(manifest_path)?);
    writeln!(
        manifest,
        "file\tnote\tfrequency_hz\tshape\tcolor\tpeak\trms\tdc\tsample_hash"
    )?;
    for note in [36_u8, 60, 84] {
        for shape in [0.0_f32, 0.5, 1.0] {
            for color in [0.0_f32, 0.5, 1.0] {
                let mut preset = base_preset.clone();
                preset.macros.shape = Normalized::new(shape)?;
                preset.macros.color = Normalized::new(color)?;
                let spec = RenderSpec {
                    sample_rate: 48_000,
                    note,
                    velocity: 0.8,
                    seconds: 1.25,
                };
                let samples = render_note(&preset, spec)?;
                let filename = format!(
                    "note{note:03}_shape{:03}_color{:03}.wav",
                    (shape * 100.0).round() as u8,
                    (color * 100.0).round() as u8
                );
                write_listening_wav(
                    &output_directory.join(&filename),
                    &samples,
                    spec.sample_rate,
                )?;
                let (peak, rms, dc, sample_hash) = listening_metrics(&samples);
                writeln!(
                    manifest,
                    "{filename}\t{note}\t{:.6}\t{shape:.2}\t{color:.2}\t{peak:.6}\t{rms:.6}\t{dc:.9}\t{sample_hash:016x}",
                    midi_frequency(note)
                )?;
            }
        }
    }
    Ok(())
}

fn write_listening_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), hound::Error> {
    let specification = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, specification)?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()
}

fn listening_metrics(samples: &[f32]) -> (f64, f64, f64, u64) {
    let mut peak = 0.0_f64;
    let mut energy = 0.0;
    let mut sum = 0.0;
    let mut sample_hash = 0xcbf2_9ce4_8422_2325_u64;
    for sample in samples {
        let sample_f64 = f64::from(*sample);
        peak = peak.max(sample_f64.abs());
        energy += sample_f64 * sample_f64;
        sum += sample_f64;
        sample_hash ^= u64::from(sample.to_bits());
        sample_hash = sample_hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let count = samples.len() as f64;
    (peak, (energy / count).sqrt(), sum / count, sample_hash)
}

fn compare(output_directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_directory)?;
    let rows = comparison_rows()?;
    let selection = select_method(&rows);
    write_tsv(&output_directory.join("oscillator-comparison.tsv"), &rows)?;
    write_markdown(
        &output_directory.join("oscillator-comparison.md"),
        &rows,
        selection,
    )?;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct ComparisonRow {
    method: OscillatorMethod,
    note: u8,
    frequency_hz: f32,
    shape: f32,
    metrics: WaveformMetrics,
}

fn comparison_rows() -> Result<Vec<ComparisonRow>, Box<dyn std::error::Error>> {
    let mut rows = Vec::with_capacity(2 * NOTES.len() * SHAPES.len());
    for method in [
        OscillatorMethod::PolyBlep,
        OscillatorMethod::IntegratedWavetable,
    ] {
        for note in NOTES {
            let nominal_frequency_hz = midi_frequency(note);
            let period_samples = (SAMPLE_RATE / nominal_frequency_hz).round() as usize;
            let frequency_hz = SAMPLE_RATE / period_samples as f32;
            for shape in SHAPES {
                let metrics = measure_candidate(
                    method,
                    AnalysisSpec {
                        sample_rate: SAMPLE_RATE,
                        frequency_hz,
                        shape,
                        sample_count: period_samples * ANALYSIS_CYCLES,
                    },
                )?;
                rows.push(ComparisonRow {
                    method,
                    note,
                    frequency_hz,
                    shape,
                    metrics,
                });
            }
        }
    }
    Ok(rows)
}

fn select_method(rows: &[ComparisonRow]) -> OscillatorMethod {
    let polyblep = aggregate(rows, OscillatorMethod::PolyBlep);
    let integrated = aggregate(rows, OscillatorMethod::IntegratedWavetable);
    const WORST_CASE_TIE_DB: f64 = 0.1;
    if polyblep.1 < integrated.1 - WORST_CASE_TIE_DB
        || ((polyblep.1 - integrated.1).abs() <= WORST_CASE_TIE_DB && polyblep.0 <= integrated.0)
    {
        OscillatorMethod::PolyBlep
    } else {
        OscillatorMethod::IntegratedWavetable
    }
}

fn aggregate(rows: &[ComparisonRow], method: OscillatorMethod) -> (f64, f64) {
    let selected: Vec<_> = rows.iter().filter(|row| row.method == method).collect();
    let mean_error_energy = selected
        .iter()
        .map(|row| 10.0_f64.powf(row.metrics.alias_error_db / 10.0))
        .sum::<f64>()
        / selected.len() as f64;
    let worst_db = selected
        .iter()
        .map(|row| row.metrics.alias_error_db)
        .fold(f64::NEG_INFINITY, f64::max);
    (10.0 * mean_error_energy.log10(), worst_db)
}

fn write_tsv(path: &Path, rows: &[ComparisonRow]) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(
        output,
        "method\tnote\tfrequency_hz\tshape\talias_error_db\tpeak\trms\tdc\tfinite\tsample_hash"
    )?;
    for row in rows {
        writeln!(
            output,
            "{}\t{}\t{:.6}\t{:.2}\t{:.6}\t{:.6}\t{:.6}\t{:.9}\t{}\t{:016x}",
            method_name(row.method),
            row.note,
            row.frequency_hz,
            row.shape,
            row.metrics.alias_error_db,
            row.metrics.peak,
            row.metrics.rms,
            row.metrics.dc,
            row.metrics.finite,
            row.metrics.sample_hash,
        )?;
    }
    Ok(())
}

fn write_markdown(
    path: &Path,
    rows: &[ComparisonRow],
    selection: OscillatorMethod,
) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "# Oscillator comparison\n")?;
    writeln!(
        output,
        "`alias_error_db` is residual energy relative to a finite band-limited Fourier reference after DC removal and one fitted scalar gain. It conservatively includes aliasing plus amplitude and phase error; more-negative values are better.\n"
    )?;
    writeln!(
        output,
        "| Method | Mean error (dB) | Worst error (dB) | Selected |"
    )?;
    writeln!(output, "| --- | ---: | ---: | :---: |")?;
    for method in [
        OscillatorMethod::PolyBlep,
        OscillatorMethod::IntegratedWavetable,
    ] {
        let (mean, worst) = aggregate(rows, method);
        writeln!(
            output,
            "| {} | {:.3} | {:.3} | {} |",
            method_name(method),
            mean,
            worst,
            if method == selection { "yes" } else { "no" }
        )?;
    }
    writeln!(
        output,
        "\nSelection rule: lower worst-case residual wins; values within 0.1 dB are treated as tied and use lower aggregate residual, then the smaller/no-table method. Evidence selects **{}**.\n",
        method_name(selection)
    )?;
    writeln!(
        output,
        "Raw conditions: 48 kHz, 32 coherent periods at the nearest integer-sample period to MIDI notes 36/60/84/96, and shape 0.00/0.50/1.00. See `oscillator-comparison.tsv`."
    )?;
    Ok(())
}

fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

fn method_name(method: OscillatorMethod) -> &'static str {
    match method {
        OscillatorMethod::PolyBlep => "polyblep",
        OscillatorMethod::IntegratedWavetable => "integrated_wavetable",
    }
}
