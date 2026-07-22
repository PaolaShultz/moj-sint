use moj_sint::analysis::{AnalysisSpec, WaveformMetrics, measure_candidate};
use moj_sint::dsp::oscillator::OscillatorMethod;
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
        _ => {
            eprintln!("Usage: oscillator-lab compare <output-directory>");
            ExitCode::FAILURE
        }
    }
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
        || ((polyblep.1 - integrated.1).abs() <= WORST_CASE_TIE_DB
            && polyblep.0 <= integrated.0)
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
