use moj_sint::dsp::research::{ResearchFamily, ResearchSource, phase_period};
use moj_sint::research::{
    ResearchRenderSpec, apply_fades, loudness_match, measure_alias_error, measure_stereo,
    midi_frequency, render_and_measure,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const SAMPLE_RATE: u32 = 48_000;
const NOTES: [u8; 3] = [36, 60, 84];

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, output] if command == "render" => match render_gate(Path::new(output)) {
            Ok(()) => {
                println!("wrote five-family listening gate to {output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("Usage: five-family-lab render <output-directory>");
            ExitCode::FAILURE
        }
    }
}

fn render_gate(output_directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_directory)?;
    let mut manifest = BufWriter::new(File::create(output_directory.join("manifest.tsv"))?);
    let mut spectral = BufWriter::new(File::create(output_directory.join("spectral.tsv"))?);
    let mut spatial = BufWriter::new(File::create(output_directory.join("spatial.tsv"))?);
    writeln!(
        manifest,
        "file\tfamily\tnote\tfrequency_hz\tpeak\trms\tdc\tfundamental_db\tpitch_retained\tfinite\tsample_hash"
    )?;
    write!(spectral, "file\tfamily\tnote")?;
    for harmonic in 1..=16 {
        write!(spectral, "\th{harmonic:02}_db")?;
    }
    writeln!(spectral)?;
    writeln!(
        spatial,
        "file\tfamily\tnote\tcorrelation\tside_to_mid\tmono_rms\tmaximum_jump"
    )?;

    for family in ResearchFamily::ALL {
        for note in NOTES {
            let mut render = render_and_measure(
                family,
                ResearchRenderSpec {
                    sample_rate: SAMPLE_RATE,
                    note,
                    seconds: 1.5,
                },
            )?;
            apply_fades(&mut render.samples, SAMPLE_RATE, 0.01);
            loudness_match(&mut render.samples, SAMPLE_RATE, 0.08);
            let metrics =
                measure_stereo(&render.samples, SAMPLE_RATE as f32, midi_frequency(note))?;
            let filename = format!("{}_note{note:03}.wav", family_slug(family));
            write_wav(
                &output_directory.join(&filename),
                &render.samples,
                SAMPLE_RATE,
            )?;
            writeln!(
                manifest,
                "{filename}\t{}\t{note}\t{:.6}\t{:.6}\t{:.6}\t{:.9}\t{:.3}\t{}\t{}\t{:016x}",
                family_slug(family),
                midi_frequency(note),
                metrics.peak,
                metrics.rms,
                metrics.dc,
                metrics.fundamental_db,
                metrics.pitch_retained,
                metrics.finite,
                metrics.sample_hash,
            )?;
            write!(spectral, "{filename}\t{}\t{note}", family_slug(family))?;
            for level in metrics.harmonics_db {
                write!(spectral, "\t{level:.3}")?;
            }
            writeln!(spectral)?;
            writeln!(
                spatial,
                "{filename}\t{}\t{note}\t{:.6}\t{:.6}\t{:.6}\t{:.6}",
                family_slug(family),
                metrics.correlation,
                metrics.side_to_mid,
                metrics.mono_rms,
                metrics.maximum_jump,
            )?;
        }
    }
    manifest.flush()?;
    spectral.flush()?;
    spatial.flush()?;
    write_alias_error(&output_directory.join("alias-error.tsv"))?;
    write_integer_cycles(&output_directory.join("integer-cycles.tsv"))?;
    write_cost(&output_directory.join("workstation-cost.txt"))?;
    write_readme(&output_directory.join("README.md"))?;
    Ok(())
}

fn write_alias_error(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "family\tnote\talias_error_db\treference")?;
    for family in [
        ResearchFamily::NonlinearPm,
        ResearchFamily::SpectralTraversal,
        ResearchFamily::IntegerSwarm,
    ] {
        for note in NOTES {
            let residual = measure_alias_error(family, note, 8_192)?;
            writeln!(
                output,
                "{}\t{note}\t{residual:.3}\t8x_box_decimated_conservative_residual",
                family_slug(family)
            )?;
        }
    }
    Ok(())
}

fn write_integer_cycles(path: &Path) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "note\twidth_bits\tincrement\tphase_period_samples")?;
    for note in NOTES {
        let frequency = midi_frequency(note);
        for width in [8_u8, 10, 12, 16] {
            let modulus = if width == 16 {
                65_536.0
            } else {
                (1_u32 << width) as f32
            };
            let increment = (frequency * modulus / SAMPLE_RATE as f32)
                .round()
                .clamp(1.0, modulus - 1.0) as u16;
            writeln!(
                output,
                "{note}\t{width}\t{increment}\t{}",
                phase_period(width, increment)
            )?;
        }
    }
    Ok(())
}

fn write_cost(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    const SAMPLE_COUNT: usize = 480_000;
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "scope=isolated_scalar_research_source")?;
    writeln!(output, "sample_count_per_family={SAMPLE_COUNT}")?;
    for family in ResearchFamily::ALL {
        let mut source = ResearchSource::new(family, SAMPLE_RATE as f32, 440.0)?;
        let start = Instant::now();
        let mut accumulator = 0.0_f32;
        for _ in 0..SAMPLE_COUNT {
            let frame = source.sample();
            accumulator += frame.left + frame.right;
        }
        std::hint::black_box(accumulator);
        let elapsed = start.elapsed();
        writeln!(
            output,
            "{}_nanoseconds_per_sample={:.3}",
            family_slug(family),
            elapsed.as_nanos() as f64 / SAMPLE_COUNT as f64
        )?;
    }
    writeln!(
        output,
        "limitation=scalar x86_64 workstation development evidence; not callback timing or Raspberry Pi evidence"
    )?;
    Ok(())
}

fn write_readme(path: &Path) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "# Moj Sint five-family listening gate\n")?;
    writeln!(
        output,
        "The human listening verdict is open. These are five structurally different disposable monophonic research sources, not presets or accepted macro mappings.\n"
    )?;
    writeln!(
        output,
        "For each MIDI note 36, 60, and 84, compare nonlinear PM, excited comb, spatial micro-delay, spectral traversal, and integer swarm. All files are stereo 32-bit float at 48 kHz and were matched toward RMS 0.08 subject to a 0.98 peak ceiling.\n"
    )?;
    writeln!(
        output,
        "Use ordinary playback level. Do not add normalization between files. Judge whether the five mechanisms are genuinely distinct and whether any one is worth developing. The spatial candidate has not been accepted on headphones, speakers, or mono.\n"
    )?;
    writeln!(
        output,
        "Automated reports describe level, DC, pitch/partials, deterministic hashes, spatial behavior, conservative 8x alias/error residuals for PM, spectral traversal, and integer swarm, and integer phase periods. Workstation timing is not Raspberry Pi evidence and makes no latency, polyphony, or sound-quality claim."
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

fn family_slug(family: ResearchFamily) -> &'static str {
    match family {
        ResearchFamily::NonlinearPm => "nonlinear-pm",
        ResearchFamily::ExcitedComb => "excited-comb",
        ResearchFamily::SpatialMicroDelay => "spatial-micro-delay",
        ResearchFamily::SpectralTraversal => "spectral-traversal",
        ResearchFamily::IntegerSwarm => "integer-swarm",
    }
}
