use moj_sint::dsp::hybrid::{HybridFamily, HybridVoice};
use moj_sint::hybrid::{
    HybridCondition, HybridRenderSpec, PROGRESSION, measure_frequency_levels,
    measure_hybrid, measure_hybrid_alias_error, measure_target_levels, midi_frequency,
    render_hybrid,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const SAMPLE_RATE: u32 = 48_000;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, output] if command == "render" => match render_lab(Path::new(output)) {
            Ok(()) => {
                println!("wrote final hybrid sound lab to {output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("Usage: hybrid-sound-lab render <output-directory>");
            ExitCode::FAILURE
        }
    }
}

fn render_lab(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    let mut manifest = BufWriter::new(File::create(output.join("manifest.tsv"))?);
    let mut impact = BufWriter::new(File::create(output.join("impact.tsv"))?);
    let mut stereo = BufWriter::new(File::create(output.join("stereo.tsv"))?);
    let mut harmony = BufWriter::new(File::create(output.join("harmony.tsv"))?);
    let mut products = BufWriter::new(File::create(output.join("products.tsv"))?);
    writeln!(
        manifest,
        "file\tfamily\tcondition\tpeak\trms\tdc\tcrest\tfinite\thash"
    )?;
    writeln!(
        impact,
        "file\ttime_to_peak_ms\tearly_0_5ms_rms\tbody_20_40ms_rms\tmaximum_jump"
    )?;
    writeln!(
        stereo,
        "file\tcorrelation\tside_to_mid\tmono_rms\tdifference_rms"
    )?;
    writeln!(
        harmony,
        "file\tsegment\tnote1\tnote2\tnote3\tlevel1_db\tlevel2_db\tlevel3_db"
    )?;
    writeln!(
        products,
        "file\tdifference1_hz\tdifference2_hz\tdifference3_hz\tlevel1_db\tlevel2_db\tlevel3_db"
    )?;

    for family in HybridFamily::ALL {
        for condition in [
            HybridCondition::Single,
            HybridCondition::HeldChord,
            HybridCondition::Progression,
            HybridCondition::ProgressionMono,
        ] {
            let mut render = render_hybrid(HybridRenderSpec {
                sample_rate: SAMPLE_RATE,
                family,
                condition,
            })?;
            loudness_match(
                &mut render.samples,
                if condition == HybridCondition::Single {
                    0.075
                } else {
                    0.065
                },
            );
            let notes = condition_notes(condition);
            let metrics = measure_hybrid(&render.samples, SAMPLE_RATE as f32, notes)?;
            let filename = format!("{}_{}.wav", family_slug(family), condition_slug(condition));
            write_wav(&output.join(&filename), &render.samples)?;
            writeln!(
                manifest,
                "{filename}\t{}\t{}\t{:.6}\t{:.6}\t{:.9}\t{:.4}\t{}\t{:016x}",
                family_slug(family),
                condition_slug(condition),
                metrics.peak,
                metrics.rms,
                metrics.dc,
                metrics.crest_factor,
                metrics.finite,
                metrics.sample_hash
            )?;
            let (time_to_peak_ms, early_rms, body_rms) = impact_metrics(&render.samples);
            writeln!(
                impact,
                "{filename}\t{time_to_peak_ms:.4}\t{early_rms:.6}\t{body_rms:.6}\t{:.6}",
                metrics.maximum_jump
            )?;
            writeln!(
                stereo,
                "{filename}\t{:.6}\t{:.6}\t{:.6}\t{:.6}",
                metrics.correlation,
                metrics.side_to_mid,
                metrics.mono_rms,
                metrics.difference_rms
            )?;
            write_harmony_rows(
                &mut harmony,
                &filename,
                condition,
                &render.samples,
            )?;
            write_product_row(&mut products, &filename, notes, &render.samples)?;
        }
    }
    manifest.flush()?;
    impact.flush()?;
    stereo.flush()?;
    harmony.flush()?;
    products.flush()?;
    write_alias(&output.join("alias-error.tsv"))?;
    write_topology(&output.join("topology.tsv"))?;
    write_cost(&output.join("workstation-cost.txt"))?;
    write_readme(&output.join("README.md"))?;
    Ok(())
}

fn condition_notes(condition: HybridCondition) -> [u8; 3] {
    match condition {
        HybridCondition::Single => [38; 3],
        HybridCondition::HeldChord => [50, 53, 57],
        HybridCondition::Progression | HybridCondition::ProgressionMono => PROGRESSION[0],
    }
}

fn write_harmony_rows(
    output: &mut impl Write,
    filename: &str,
    condition: HybridCondition,
    samples: &[f32],
) -> std::io::Result<()> {
    match condition {
        HybridCondition::Progression | HybridCondition::ProgressionMono => {
            let segment_frames = 4 * SAMPLE_RATE as usize;
            for (index, notes) in PROGRESSION.iter().copied().enumerate() {
                let start = 2 * index * segment_frames;
                let end = start + 2 * segment_frames;
                let levels =
                    measure_target_levels(&samples[start..end], SAMPLE_RATE as f32, notes);
                writeln!(
                    output,
                    "{filename}\t{index}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}",
                    notes[0], notes[1], notes[2], levels[0], levels[1], levels[2]
                )?;
            }
        }
        _ => {
            let notes = condition_notes(condition);
            let levels = measure_target_levels(samples, SAMPLE_RATE as f32, notes);
            writeln!(
                output,
                "{filename}\t0\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}",
                notes[0], notes[1], notes[2], levels[0], levels[1], levels[2]
            )?;
        }
    }
    Ok(())
}

fn write_product_row(
    output: &mut impl Write,
    filename: &str,
    notes: [u8; 3],
    samples: &[f32],
) -> std::io::Result<()> {
    let frequencies = notes.map(midi_frequency);
    let differences = [
        (frequencies[1] - frequencies[0]).abs(),
        (frequencies[2] - frequencies[1]).abs(),
        (frequencies[2] - frequencies[0]).abs(),
    ];
    let levels = measure_frequency_levels(samples, SAMPLE_RATE as f32, differences);
    writeln!(
        output,
        "{filename}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}",
        differences[0], differences[1], differences[2], levels[0], levels[1], levels[2]
    )
}

fn impact_metrics(samples: &[f32]) -> (f64, f64, f64) {
    let frames = samples.len() / 2;
    let window = frames.min((0.1 * SAMPLE_RATE as f32) as usize);
    let mut peak = 0.0;
    let mut peak_index = 0;
    for (index, frame) in samples.chunks_exact(2).take(window).enumerate() {
        let value = f64::from(0.5 * (frame[0] + frame[1])).abs();
        if value > peak {
            peak = value;
            peak_index = index;
        }
    }
    let rms_between = |start_ms: f32, end_ms: f32| {
        let start = (start_ms * 0.001 * SAMPLE_RATE as f32) as usize;
        let end = ((end_ms * 0.001 * SAMPLE_RATE as f32) as usize).min(frames);
        let mut energy = 0.0;
        let mut count = 0;
        for frame in samples[2 * start..2 * end].chunks_exact(2) {
            let mid = f64::from(0.5 * (frame[0] + frame[1]));
            energy += mid * mid;
            count += 1;
        }
        (energy / count.max(1) as f64).sqrt()
    };
    (
        1_000.0 * peak_index as f64 / SAMPLE_RATE as f64,
        rms_between(0.0, 5.0),
        rms_between(20.0, 40.0),
    )
}

fn loudness_match(samples: &mut [f32], target_rms: f64) {
    let energy = samples
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    let rms = (energy / samples.len() as f64).sqrt();
    let peak = samples
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0, f64::max);
    if rms == 0.0 || peak == 0.0 {
        return;
    }
    let gain = (target_rms / rms).min(0.979 / peak);
    for sample in samples {
        *sample *= gain as f32;
    }
}

fn write_alias(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "family\tnote\tresidual_db\treference")?;
    for family in HybridFamily::ALL {
        for note in [36, 60, 84] {
            writeln!(
                output,
                "{}\t{note}\t{:.3}\t8x_box_decimated_conservative_residual",
                family_slug(family),
                measure_hybrid_alias_error(family, note, 8_192)?
            )?;
        }
    }
    Ok(())
}

fn write_topology(path: &Path) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "family\tmechanisms\tmovement_rates_hz")?;
    for family in HybridFamily::ALL {
        let voice = HybridVoice::new(family, SAMPLE_RATE as f32, 110.0, 7).unwrap();
        let rates = voice.movement_rates_hz();
        let mechanisms = match family {
            HybridFamily::CrossCoupledMachine => {
                "impact+pm+register+split-drive+cross-resonator+micro-delay"
            }
            HybridFamily::SpectralShadow => {
                "spectral-frames+address-machine+sub-shadow+asymmetric-filter+micro-delay"
            }
            HybridFamily::DualResonantBody => {
                "finite-excitation+register-events+four-resonators+cross-feedback+body-delay"
            }
        };
        writeln!(
            output,
            "{}\t{mechanisms}\t{:.3},{:.3},{:.3}",
            family_slug(family),
            rates[0],
            rates[1],
            rates[2]
        )?;
    }
    Ok(())
}

fn write_cost(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    const FRAMES: usize = 480_000;
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "scope=isolated_scalar_hybrid_voice")?;
    for family in HybridFamily::ALL {
        let mut voice = HybridVoice::new(family, SAMPLE_RATE as f32, 110.0, 0xc057)?;
        let start = Instant::now();
        let mut accumulator = 0.0;
        for _ in 0..FRAMES {
            let frame = voice.sample();
            accumulator += frame.left + frame.right;
        }
        std::hint::black_box(accumulator);
        writeln!(
            output,
            "{}_nanoseconds_per_frame={:.3}",
            family_slug(family),
            start.elapsed().as_nanos() as f64 / FRAMES as f64
        )?;
    }
    writeln!(
        output,
        "limitation=scalar x86_64 workstation evidence; not Raspberry Pi or callback evidence"
    )?;
    Ok(())
}

fn write_readme(path: &Path) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "# Moj Sint final hybrid sound lab\n")?;
    writeln!(
        output,
        "These are three complete, structurally different, genuinely stereo research voices. The human listening decides whether any sound is worth developing; none is accepted as a final sound or macro mapping.\n"
    )?;
    writeln!(
        output,
        "Listen in this order: the three single files, the three chord files, the three stereo progressions on headphones, those progressions on speakers, then the three progression-mono mono-fold diagnostics. Start at a comfortable low level and do not raise gain to seek physical vibration.\n"
    )?;
    writeln!(
        output,
        "The files and reports describe normalized waveform evidence. It does not report acoustic SPL or guarantee tactile sensation. Reported peak is sample peak, not standardized ITU-R true peak.\n"
    )?;
    writeln!(
        output,
        "All cost data is scalar x86_64 workstation development evidence, not Raspberry Pi evidence. No JACK/ALSA host, SHR-DAW integration, safe polyphony, headphone/speaker translation, or sound-quality conclusion follows automatically."
    )
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

fn family_slug(family: HybridFamily) -> &'static str {
    match family {
        HybridFamily::CrossCoupledMachine => "cross-coupled-machine",
        HybridFamily::SpectralShadow => "spectral-shadow",
        HybridFamily::DualResonantBody => "dual-resonant-body",
    }
}

fn condition_slug(condition: HybridCondition) -> &'static str {
    match condition {
        HybridCondition::Single => "single",
        HybridCondition::HeldChord => "chord",
        HybridCondition::Progression => "progression",
        HybridCondition::ProgressionMono => "progression-mono",
    }
}
