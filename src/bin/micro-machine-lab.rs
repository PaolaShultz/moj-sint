use moj_sint::envelope::AdsrConfig;
use moj_sint::micro_machine::{MicroMachineGraph, MicroMachineVoice, SwarmControls};
use moj_sint::micro_machine_lab::{measure_high_rate_residual, midi_frequency};
use moj_sint::research::measure_stereo;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

const SAMPLE_RATE: u32 = 48_000;
const CONTROL_LABELS: [&str; 8] = [
    "MASS", "DETUNE", "SPREAD", "SHAPE", "BITE", "MOTION", "COLOR", "SPACE",
];

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, graph, output] if command == "render" => {
            match render_lab(Path::new(graph), Path::new(output)) {
                Ok(()) => {
                    println!("wrote typed swarm listening set to {output}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("Usage: micro-machine-lab render <graph.toml> <empty-output-directory>");
            ExitCode::FAILURE
        }
    }
}

fn render_lab(graph_path: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(graph_path)?;
    let graph = MicroMachineGraph::parse(&source)?;
    fs::create_dir_all(output)?;
    if fs::read_dir(output)?.next().is_some() {
        return Err("output directory must be empty".into());
    }

    let mut renders = Vec::new();
    renders.push((
        "01_neutral-reference.wav".to_owned(),
        render_phrase(&graph, SwarmControls::NEUTRAL, &[45, 52, 57, 64], 0.46)?,
    ));
    renders.push((
        "02_forceful-lead.wav".to_owned(),
        render_phrase(&graph, SwarmControls::LEAD, &[45, 57, 64, 69, 57], 0.34)?,
    ));
    renders.push(("03_warm-pad.wav".to_owned(), render_pad(&graph)?));
    for (index, label) in CONTROL_LABELS.into_iter().enumerate() {
        renders.push((
            format!("{:02}_{label}_low-high.wav", index + 10),
            render_control_reel(&graph, index)?,
        ));
    }

    for (name, samples) in &renders {
        write_wav(&output.join(name), samples)?;
    }
    write_graph_summary(&output.join("graph-summary.tsv"), &graph)?;
    write_measurements(&output.join("measurements.tsv"), &renders)?;
    write_alias_error(&output.join("alias-error.tsv"), &graph)?;
    write_hashes(&output.join("hashes.tsv"), &renders)?;
    write_readme(&output.join("README.md"), graph_path)?;
    Ok(())
}

fn render_phrase(
    graph: &MicroMachineGraph,
    controls: SwarmControls,
    notes: &[u8],
    note_seconds: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let gap_frames = (0.06 * SAMPLE_RATE as f32) as usize;
    let adsr = AdsrConfig::new(0.008, 0.070, 0.72, 0.090)?;
    let mut output = Vec::new();
    for (index, note) in notes.iter().copied().enumerate() {
        let segment = render_note(
            graph,
            controls,
            note,
            note_seconds,
            note_seconds - 0.11,
            adsr,
        )?;
        output.extend_from_slice(&segment);
        if index + 1 != notes.len() {
            output.resize(output.len() + 2 * gap_frames, 0.0);
        }
    }
    Ok(output)
}

fn render_pad(graph: &MicroMachineGraph) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let chords = [[48_u8, 55, 60], [50, 57, 62], [45, 52, 57]];
    let seconds = 1.55;
    let frames = (seconds * SAMPLE_RATE as f32) as usize;
    let note_off = (0.98 * SAMPLE_RATE as f32) as usize;
    let adsr = AdsrConfig::new(0.16, 0.32, 0.76, 0.48)?;
    let mut output = Vec::with_capacity(2 * frames * chords.len());
    for chord in chords {
        let mut voices = chord
            .into_iter()
            .map(|note| {
                MicroMachineVoice::compile(
                    graph,
                    SAMPLE_RATE as f32,
                    midi_frequency(note),
                    SwarmControls::PAD,
                    adsr,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        for voice in &mut voices {
            voice.note_on(0.76);
        }
        for frame in 0..frames {
            if frame == note_off {
                for voice in &mut voices {
                    voice.note_off();
                }
            }
            let mut mixed = [0.0_f32; 2];
            for voice in &mut voices {
                let sample = voice.sample();
                mixed[0] += sample[0] / 3.0;
                mixed[1] += sample[1] / 3.0;
            }
            output.push(mixed[0]);
            output.push(mixed[1]);
        }
    }
    Ok(output)
}

fn render_control_reel(
    graph: &MicroMachineGraph,
    control: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let gap_frames = (0.18 * SAMPLE_RATE as f32) as usize;
    let adsr = AdsrConfig::new(0.010, 0.090, 0.78, 0.12)?;
    let mut output = Vec::new();
    for (index, value) in [0.08_f32, 0.92].into_iter().enumerate() {
        let controls = SwarmControls::NEUTRAL.with(control, value)?;
        let samples = render_note(graph, controls, 57, 1.35, 1.18, adsr)?;
        output.extend_from_slice(&samples);
        if index == 0 {
            output.resize(output.len() + 2 * gap_frames, 0.0);
        }
    }
    Ok(output)
}

fn render_note(
    graph: &MicroMachineGraph,
    controls: SwarmControls,
    note: u8,
    seconds: f32,
    note_off_seconds: f32,
    adsr: AdsrConfig,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let frames = (seconds * SAMPLE_RATE as f32).round() as usize;
    let note_off = (note_off_seconds * SAMPLE_RATE as f32).round() as usize;
    let mut voice = MicroMachineVoice::compile(
        graph,
        SAMPLE_RATE as f32,
        midi_frequency(note),
        controls,
        adsr,
    )?;
    voice.note_on(0.82);
    let mut output = Vec::with_capacity(2 * frames);
    for frame in 0..frames {
        if frame == note_off {
            voice.note_off();
        }
        let sample = voice.sample();
        output.push(sample[0]);
        output.push(sample[1]);
    }
    Ok(output)
}

fn write_graph_summary(
    path: &Path,
    graph: &MicroMachineGraph,
) -> Result<(), Box<dyn std::error::Error>> {
    let machine = graph.compile(48_000.0, 220.0, SwarmControls::NEUTRAL)?;
    let usage = machine.resource_usage();
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(
        output,
        "nodes\tedges\toscillator_slots\tstate_bytes\twork_units_per_sample\tplan_fingerprint"
    )?;
    writeln!(
        output,
        "{}\t{}\t{}\t{}\t{}\t{:016x}",
        usage.nodes,
        usage.edges,
        usage.oscillator_slots,
        usage.state_bytes,
        usage.work_units_per_sample,
        machine.plan_fingerprint()
    )?;
    Ok(())
}

fn write_measurements(
    path: &Path,
    renders: &[(String, Vec<f32>)],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(
        output,
        "file\tpeak\trms\tdc\tmaximum_jump\tcorrelation\tside_to_mid\tmono_rms\tfinite"
    )?;
    for (name, samples) in renders {
        let metrics = measure_stereo(samples, SAMPLE_RATE as f32, midi_frequency(57))?;
        writeln!(
            output,
            "{}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{}",
            name,
            metrics.peak,
            metrics.rms,
            metrics.dc,
            metrics.maximum_jump,
            metrics.correlation,
            metrics.side_to_mid,
            metrics.mono_rms,
            metrics.finite
        )?;
    }
    Ok(())
}

fn write_alias_error(
    path: &Path,
    graph: &MicroMachineGraph,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "note\t48k_vs_384k_fitted_residual_db\tstatus")?;
    for note in [36, 60, 84] {
        let residual = measure_high_rate_residual(graph, SwarmControls::NEUTRAL, note, 16_384)?;
        writeln!(output, "{note}\t{residual:.6}\tdiagnostic")?;
    }
    Ok(())
}

fn write_hashes(path: &Path, renders: &[(String, Vec<f32>)]) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "file\tinterleaved_f32_fnv1a")?;
    for (name, samples) in renders {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for sample in samples {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        writeln!(output, "{name}\t{hash:016x}")?;
    }
    Ok(())
}

fn write_readme(path: &Path, graph_path: &Path) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "# Typed swarm micro-machine listening set\n")?;
    writeln!(
        output,
        "This disposable offline experiment was compiled from `{}`. It is not a fourth live model, factory preset, or production integration.\n",
        graph_path.display()
    )?;
    writeln!(output, "## Controls\n")?;
    writeln!(
        output,
        "The exact timbral surface is MASS, DETUNE, SPREAD, SHAPE, BITE, MOTION, COLOR, SPACE. ADSR is the existing outer envelope and is not a graph node.\n"
    )?;
    writeln!(output, "## Order\n")?;
    writeln!(
        output,
        "01 is the neutral four-note reference. 02 is a tighter forceful lead phrase. 03 is a three-chord warm pad. Files 10-17 compare low, silence, high for one named timbral control at a time.\n"
    )?;
    writeln!(output, "## Signal boundary\n")?;
    writeln!(
        output,
        "The graph contains separate phase-bank, waveform-bank, normalized mixer, spectral, nonlinearity, stereo-width, and output-guard nodes. No reverb, delay, chorus, compressor, limiter, or normalization is present. No Engine, preset schema, JACK, ALSA, MIDI, SHR-DAW, or hardware path is used.\n"
    )?;
    writeln!(output, "## Evidence\n")?;
    writeln!(
        output,
        "`graph-summary.tsv` records the compiled resource plan. `measurements.tsv` records finite level and stereo/mono evidence. `alias-error.tsv` is a conservative 48 kHz versus 384 kHz fitted residual diagnostic; it is not a perceptual or alias-free claim. Human listening decides whether the sound is useful."
    )?;
    Ok(())
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<(), hound::Error> {
    let specification = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, specification)?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()
}
