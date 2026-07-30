use moj_sint::analysis::{
    AnalysisSpec, CharacterAnalysisSpec, CharacterMetrics, WaveformMetrics, measure_candidate,
    measure_character_intermodulation, measure_character_method, measure_harmonic_distribution,
    measure_harmonic_system,
};
use moj_sint::control::Normalized;
use moj_sint::dsp::character::{CharacterLayer, CharacterMethod};
use moj_sint::dsp::harmonic_selector::ThreePhaseBank;
use moj_sint::dsp::oscillator::{BandlimitedOscillator, OscillatorMethod};
use moj_sint::engine::{ENGINE_CHARACTER_METHOD, ENGINE_OSCILLATOR_METHOD};
use moj_sint::offline::{RenderSpec, render_note};
use moj_sint::preset::Preset;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

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
        [command, output] if command == "system" => {
            match render_harmonic_selector_evidence(Path::new(output)) {
                Ok(()) => {
                    println!("wrote harmonic-selector evidence to {output}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        [command, output] if command == "character" => {
            match render_character_evidence(Path::new(output)) {
                Ok(()) => {
                    println!("wrote parallel-character evidence to {output}");
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
                "Usage:\n  oscillator-lab compare <output-directory>\n  oscillator-lab render <output-directory>\n  oscillator-lab system <output-directory>\n  oscillator-lab character <output-directory>"
            );
            ExitCode::FAILURE
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CharacterComparisonRow {
    method: CharacterMethod,
    note: u8,
    condition: &'static str,
    metrics: CharacterMetrics,
}

fn render_character_evidence(output_directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_directory)?;
    let rows = character_comparison_rows()?;
    let selection = select_character_method(&rows).ok_or_else(|| {
        let summaries: Vec<_> = CharacterMethod::ALL
            .into_iter()
            .map(|method| {
                let worst = |condition: &str| {
                    rows.iter()
                        .filter(|row| row.method == method && row.condition == condition)
                        .map(|row| row.metrics.alias_error_db)
                        .fold(f64::NEG_INFINITY, f64::max)
                };
                format!(
                    "{} moderate={:.3} dB strong={:.3} dB",
                    character_method_name(method),
                    worst("moderate"),
                    worst("strong")
                )
            })
            .collect();
        format!(
            "no character method met the declared alias-error thresholds: {}",
            summaries.join(", ")
        )
    })?;
    if selection != ENGINE_CHARACTER_METHOD {
        return Err(format!(
            "evidence selects {}, but engine declares {}",
            character_method_name(selection),
            character_method_name(ENGINE_CHARACTER_METHOD)
        )
        .into());
    }
    write_character_alias_comparison(
        &output_directory.join("alias-comparison.tsv"),
        &rows,
        selection,
    )?;
    write_character_intermodulation(&output_directory.join("intermodulation.tsv"))?;
    render_character_listening_gate(output_directory)?;
    write_character_cost(&output_directory.join("workstation-cost.txt"), selection)?;
    let mut readme = BufWriter::new(File::create(output_directory.join("README.md"))?);
    writeln!(readme, "# Parallel character listening gate\n")?;
    writeln!(
        readme,
        "Nine loudness-matched dry/moderate/strong files cover MIDI notes 36, 60, and 84. The user listening verdict is open.\n"
    )?;
    writeln!(
        readme,
        "Listen to each note in dry, moderate, strong order without additional normalization. Reject the branch if it reads as minor EQ or generic distortion; do not expand the matrix before that decision.\n"
    )?;
    writeln!(
        readme,
        "`character-manifest.tsv` records level, DC, pitch, and hashes; `harmonic-distribution.tsv` records the first 12 partials; `alias-comparison.tsv` records method selection; `intermodulation.tsv` compares per-voice and post-mix placement.\n"
    )?;
    writeln!(
        readme,
        "The selected production method is `{}`. Workstation timing is scalar development evidence only, not Raspberry Pi evidence.",
        character_method_name(selection)
    )?;
    Ok(())
}

fn character_comparison_rows() -> Result<Vec<CharacterComparisonRow>, Box<dyn std::error::Error>> {
    let mut rows = Vec::with_capacity(24);
    for method in CharacterMethod::ALL {
        for note in [36_u8, 60, 84, 96] {
            for (condition, edge, couple) in [("moderate", 0.6_f32, 0.6_f32), ("strong", 1.0, 1.0)]
            {
                let metrics = measure_character_method(
                    method,
                    CharacterAnalysisSpec {
                        sample_rate: SAMPLE_RATE,
                        frequency_hz: midi_frequency(note),
                        edge,
                        couple,
                        sample_count: 16_384,
                    },
                )?;
                rows.push(CharacterComparisonRow {
                    method,
                    note,
                    condition,
                    metrics,
                });
            }
        }
    }
    Ok(rows)
}

fn select_character_method(rows: &[CharacterComparisonRow]) -> Option<CharacterMethod> {
    let worst = |method: CharacterMethod, condition: &str| {
        rows.iter()
            .filter(|row| row.method == method && row.condition == condition)
            .map(|row| row.metrics.alias_error_db)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let overall = |method| worst(method, "moderate").max(worst(method, "strong"));
    let best_worst = CharacterMethod::ALL
        .into_iter()
        .map(overall)
        .fold(f64::INFINITY, f64::min);
    CharacterMethod::ALL.into_iter().find(|method| {
        worst(*method, "moderate") <= -60.0
            && worst(*method, "strong") <= -50.0
            && overall(*method) <= best_worst + 3.0
    })
}

fn write_character_alias_comparison(
    path: &Path,
    rows: &[CharacterComparisonRow],
    selection: CharacterMethod,
) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(
        output,
        "method\tnote\tcondition\talias_error_db\tpeak\trms\tdc\tfundamental_db\tthird_db\tpitch_retained\tfinite\tselected"
    )?;
    for row in rows {
        writeln!(
            output,
            "{}\t{}\t{}\t{:.3}\t{:.6}\t{:.6}\t{:.9}\t{:.3}\t{:.3}\t{}\t{}\t{}",
            character_method_name(row.method),
            row.note,
            row.condition,
            row.metrics.alias_error_db,
            row.metrics.peak,
            row.metrics.rms,
            row.metrics.dc,
            row.metrics.fundamental_db,
            row.metrics.harmonics_db[2],
            row.metrics.pitch_retained,
            row.metrics.finite,
            row.method == selection,
        )?;
    }
    Ok(())
}

fn write_character_intermodulation(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "method\tper_voice_imd_db\tpost_mix_imd_db\tfinite")?;
    for method in CharacterMethod::ALL {
        let metrics = measure_character_intermodulation(method, SAMPLE_RATE, 600.0, 900.0, 16_000)?;
        writeln!(
            output,
            "{}\t{:.3}\t{:.3}\t{}",
            character_method_name(method),
            metrics.per_voice_imd_db,
            metrics.post_mix_imd_db,
            metrics.finite
        )?;
    }
    Ok(())
}

fn render_character_listening_gate(
    output_directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_preset = Preset::parse(include_str!("../../presets/reference.mojsint"))?;
    let mut manifest = BufWriter::new(File::create(
        output_directory.join("character-manifest.tsv"),
    )?);
    let mut harmonic_distribution = BufWriter::new(File::create(
        output_directory.join("harmonic-distribution.tsv"),
    )?);
    writeln!(
        manifest,
        "file\tnote\tcondition\tedge\tcouple\tpeak\trms\tdc\tfundamental_db\tthird_db\tpitch_retained\tfinite\tsample_hash"
    )?;
    writeln!(
        harmonic_distribution,
        "file\tnote\tcondition\th01_db\th02_db\th03_db\th04_db\th05_db\th06_db\th07_db\th08_db\th09_db\th10_db\th11_db\th12_db"
    )?;
    for note in [36_u8, 60, 84] {
        for (condition, edge, couple) in [
            ("dry", 0.0_f32, 0.0_f32),
            ("moderate", 0.6, 0.6),
            ("strong", 1.0, 1.0),
        ] {
            let mut preset = base_preset.clone();
            preset.voices = 1;
            preset.macros.shape = Normalized::new(0.5)?;
            preset.macros.color = Normalized::new(0.5)?;
            preset.macros.edge = Normalized::new(edge)?;
            preset.macros.couple = Normalized::new(couple)?;
            let spec = RenderSpec {
                sample_rate: 48_000,
                note,
                velocity: 0.8,
                seconds: 1.25,
            };
            let mut samples = render_note(&preset, spec)?;
            loudness_match(&mut samples, spec.sample_rate, midi_frequency(note), 0.08);
            let filename = format!("note{note:03}_{condition}.wav");
            write_listening_wav(
                &output_directory.join(&filename),
                &samples,
                spec.sample_rate,
            )?;
            let mono = analysis_window(&samples, spec.sample_rate, Some(midi_frequency(note)));
            let metrics =
                measure_harmonic_system(&mono, spec.sample_rate as f32, midi_frequency(note))?;
            let harmonics = measure_harmonic_distribution(
                &mono,
                spec.sample_rate as f32,
                midi_frequency(note),
            )?;
            writeln!(
                manifest,
                "{filename}\t{note}\t{condition}\t{edge:.2}\t{couple:.2}\t{:.6}\t{:.6}\t{:.9}\t{:.3}\t{:.3}\t{}\t{}\t{:016x}",
                metrics.peak,
                metrics.rms,
                metrics.dc,
                metrics.fundamental_db,
                metrics.third_db,
                metrics.pitch_retained,
                metrics.finite,
                metrics.sample_hash
            )?;
            write!(harmonic_distribution, "{filename}\t{note}\t{condition}")?;
            for level in harmonics {
                write!(harmonic_distribution, "\t{level:.3}")?;
            }
            writeln!(harmonic_distribution)?;
        }
    }
    Ok(())
}

fn write_character_cost(path: &Path, method: CharacterMethod) -> std::io::Result<()> {
    const SAMPLE_COUNT: usize = 4_800_000;
    let mut layer = CharacterLayer::new(SAMPLE_RATE, method).expect("fixed valid sample rate");
    let start = Instant::now();
    let mut accumulator = 0.0_f32;
    for index in 0..SAMPLE_COUNT {
        let dry = (index as f32 * 0.017).sin();
        accumulator += layer.sample(dry, 1.0, 1.0);
    }
    std::hint::black_box(accumulator);
    let elapsed = start.elapsed();
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "scope=isolated_parallel_character_layer")?;
    writeln!(output, "method={}", character_method_name(method))?;
    writeln!(output, "sample_count={SAMPLE_COUNT}")?;
    writeln!(output, "elapsed_nanoseconds={}", elapsed.as_nanos())?;
    writeln!(
        output,
        "nanoseconds_per_sample={:.3}",
        elapsed.as_nanos() as f64 / SAMPLE_COUNT as f64
    )?;
    writeln!(
        output,
        "limitation=scalar x86_64 workstation development evidence; not callback timing or Raspberry Pi evidence"
    )
}

fn character_method_name(method: CharacterMethod) -> &'static str {
    match method {
        CharacterMethod::Direct => "direct",
        CharacterMethod::Adaa1 => "adaa1",
        CharacterMethod::Oversampled2x => "oversampled_2x",
    }
}

fn render_harmonic_selector_evidence(
    output_directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output_directory)?;
    let base_preset = Preset::parse(include_str!("../../presets/reference.mojsint"))?;
    let mut manifest = BufWriter::new(File::create(output_directory.join("system-manifest.tsv"))?);
    writeln!(
        manifest,
        "file\tnote\tfrequency_hz\tedge\tcouple\tpeak\trms\tdc\tfundamental_db\tthird_db\tnonharmonic_error_db\tpitch_retained\tfinite\tsample_hash"
    )?;
    for note in [36_u8, 60, 84] {
        for edge in [0.0_f32, 0.5, 1.0] {
            for couple in [0.0_f32, 0.5, 1.0] {
                let mut preset = base_preset.clone();
                preset.voices = 1;
                preset.macros.edge = Normalized::new(edge)?;
                preset.macros.couple = Normalized::new(couple)?;
                let spec = RenderSpec {
                    sample_rate: 48_000,
                    note,
                    velocity: 0.8,
                    seconds: 1.25,
                };
                let mut samples = render_legacy_harmonic_selector(
                    spec.sample_rate,
                    note,
                    spec.velocity,
                    spec.seconds,
                    preset.macros.shape.get(),
                    preset.macros.color.get(),
                    edge,
                    couple,
                )?;
                loudness_match(&mut samples, spec.sample_rate, midi_frequency(note), 0.08);
                let filename = format!(
                    "note{note:03}_edge{:03}_couple{:03}.wav",
                    (edge * 100.0).round() as u8,
                    (couple * 100.0).round() as u8
                );
                write_listening_wav(
                    &output_directory.join(&filename),
                    &samples,
                    spec.sample_rate,
                )?;
                let mono = analysis_window(&samples, spec.sample_rate, Some(midi_frequency(note)));
                let metrics =
                    measure_harmonic_system(&mono, spec.sample_rate as f32, midi_frequency(note))?;
                writeln!(
                    manifest,
                    "{filename}\t{note}\t{:.6}\t{edge:.2}\t{couple:.2}\t{:.6}\t{:.6}\t{:.9}\t{:.3}\t{:.3}\t{:.3}\t{}\t{}\t{:016x}",
                    midi_frequency(note),
                    metrics.peak,
                    metrics.rms,
                    metrics.dc,
                    metrics.fundamental_db,
                    metrics.third_db,
                    metrics.nonharmonic_error_db,
                    metrics.pitch_retained,
                    metrics.finite,
                    metrics.sample_hash,
                )?;
            }
        }
    }
    manifest.flush()?;
    write_alias_matrix(&output_directory.join("alias-matrix.tsv"))?;
    write_workstation_cost(&output_directory.join("workstation-cost.txt"))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render_legacy_harmonic_selector(
    sample_rate: u32,
    note: u8,
    velocity: f32,
    seconds: f32,
    shape: f32,
    color: f32,
    edge: f32,
    couple: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let sample_rate_f32 = sample_rate as f32;
    let frame_count = (seconds * sample_rate_f32).round() as usize;
    let frequency = midi_frequency(note);
    let mut oscillator = BandlimitedOscillator::new(sample_rate_f32, ENGINE_OSCILLATOR_METHOD)?;
    oscillator.set_frequency(frequency);
    let mut selector = ThreePhaseBank::new(sample_rate_f32)?;
    selector.set_frequency(frequency);
    let mut color_lowpass = 0.0_f32;
    let mut samples = Vec::with_capacity(2 * frame_count);
    let fade_frames = (sample_rate / 100).max(1) as usize;
    for frame in 0..frame_count {
        let oscillator_sample = oscillator.sample(shape);
        let color_coefficient =
            (oscillator.phase_increment() * (8.0 + 120.0 * color * color)).clamp(0.001, 0.75);
        color_lowpass += color_coefficient * (oscillator_sample - color_lowpass);
        let colored = color_lowpass + color * (oscillator_sample - color_lowpass);
        let harmonic = selector.sample();
        let selected = harmonic.fundamental + edge * (harmonic.third - harmonic.fundamental);
        let coupled = colored + couple * (selected - colored);
        let compensation = (1.0 + 4.0 * shape * (1.0 - shape))
            * (1.15 - 0.15 * color)
            * (1.0 + 0.4 * couple * (1.0 - couple))
            * (1.0 + 0.2 * couple * edge * (1.0 - edge));
        let fade_in = (frame as f32 / fade_frames as f32).min(1.0);
        let fade_out = ((frame_count - frame) as f32 / fade_frames as f32).min(1.0);
        let sample = coupled * compensation * velocity * 0.2 * fade_in * fade_out;
        samples.push(sample);
        samples.push(sample);
    }
    Ok(samples)
}

fn write_alias_matrix(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(
        output,
        "note\tfrequency_hz\ttarget_third_hz\tthird_weight\tnonharmonic_error_db\tpitch_retained\tfinite"
    )?;
    for note in [84_u8, 96, 108, 114, 117, 120, 127] {
        let frequency_hz = midi_frequency(note);
        let sample_count = coherent_sample_count(32_768, SAMPLE_RATE, frequency_hz);
        let mut selector = ThreePhaseBank::new(SAMPLE_RATE)?;
        selector.set_frequency(frequency_hz);
        let third_weight = selector.third_harmonic_weight();
        let samples: Vec<_> = (0..sample_count).map(|_| selector.sample().third).collect();
        let metrics = measure_harmonic_system(&samples, SAMPLE_RATE, frequency_hz)?;
        writeln!(
            output,
            "{note}\t{frequency_hz:.6}\t{:.6}\t{third_weight:.3}\t{:.3}\t{}\t{}",
            3.0 * frequency_hz,
            metrics.nonharmonic_error_db,
            metrics.pitch_retained,
            metrics.finite,
        )?;
    }
    Ok(())
}

fn analysis_window(samples: &[f32], sample_rate: u32, frequency_hz: Option<f32>) -> Vec<f32> {
    let start_frame = (sample_rate as usize) / 5;
    let maximum = 32_768.min(samples.len() / 2 - start_frame);
    let frame_count = frequency_hz.map_or(maximum, |frequency_hz| {
        coherent_sample_count(maximum, sample_rate as f32, frequency_hz)
    });
    (start_frame..start_frame + frame_count)
        .map(|frame| samples[2 * frame])
        .collect()
}

fn coherent_sample_count(maximum: usize, sample_rate: f32, frequency_hz: f32) -> usize {
    (8_192.min(maximum)..=maximum)
        .min_by(|left, right| {
            let left_cycles = frequency_hz * *left as f32 / sample_rate;
            let right_cycles = frequency_hz * *right as f32 / sample_rate;
            let left_error = left_cycles.fract().min(1.0 - left_cycles.fract());
            let right_error = right_cycles.fract().min(1.0 - right_cycles.fract());
            left_error.total_cmp(&right_error)
        })
        .unwrap_or(maximum)
}

fn loudness_match(samples: &mut [f32], sample_rate: u32, frequency_hz: f32, target_rms: f64) {
    let active = analysis_window(samples, sample_rate, Some(frequency_hz));
    let rms = (active
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / active.len() as f64)
        .sqrt();
    let peak = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    let gain = if rms > 0.0 {
        (target_rms / rms).min(0.95 / f64::from(peak.max(1.0e-12))) as f32
    } else {
        1.0
    };
    for sample in samples {
        *sample *= gain;
    }
}

fn write_workstation_cost(path: &Path) -> std::io::Result<()> {
    const SAMPLE_COUNT: usize = 4_800_000;
    let mut selector = ThreePhaseBank::new(48_000.0).expect("fixed valid sample rate");
    selector.set_frequency(440.0);
    let start = Instant::now();
    let mut accumulator = 0.0_f32;
    for _ in 0..SAMPLE_COUNT {
        let sample = selector.sample();
        accumulator += sample.fundamental + sample.third;
    }
    std::hint::black_box(accumulator);
    let elapsed = start.elapsed();
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "scope=shared_three_phase_bank_only")?;
    writeln!(output, "sample_count={SAMPLE_COUNT}")?;
    writeln!(output, "elapsed_nanoseconds={}", elapsed.as_nanos())?;
    writeln!(
        output,
        "nanoseconds_per_sample={:.3}",
        elapsed.as_nanos() as f64 / SAMPLE_COUNT as f64
    )?;
    writeln!(
        output,
        "limitation=workstation development evidence; not callback timing or Raspberry Pi evidence"
    )
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
