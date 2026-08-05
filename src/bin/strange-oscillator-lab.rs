use moj_sint::strange::{StrangeControls, StrangeInstrument, StrangeType, StrangeVoice};
use moj_sint::strange_lab::{
    StructuralMacroEvidence, evaluate_structural_macros, evaluate_type, midi_frequency,
};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

const SAMPLE_RATE: u32 = 48_000;
const PHRASE_NOTES: [u8; 4] = [45, 52, 57, 64];
const CONTROL_LABELS: [&str; 7] = [
    "FORM", "WARP", "COUPLE", "MOTION", "CHAOS", "COLOR", "SPACE",
];

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, output] if command == "render" => match render_gate(Path::new(output)) {
            Ok(()) => {
                println!("wrote Strange Oscillator gate to {output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("Usage: strange-oscillator-lab render <output-directory>");
            ExitCode::FAILURE
        }
    }
}

fn render_gate(output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(output)?;
    if fs::read_dir(output)?.next().is_some() {
        return Err("output directory must be empty".into());
    }

    let mut gate_rows = Vec::new();
    for kind in StrangeType::ALL {
        gate_rows.push(evaluate_type(kind)?);
    }
    write_gates(&output.join("gates.tsv"), &gate_rows)?;
    let mut structural_rows = Vec::with_capacity(StrangeType::ALL.len());
    for kind in StrangeType::ALL {
        structural_rows.push((kind, evaluate_structural_macros(kind)?));
    }
    write_structural_macros(&output.join("structural-macros.tsv"), &structural_rows)?;

    let mut rendered = Vec::new();
    for (index, kind) in StrangeType::ALL.into_iter().enumerate() {
        let name = format!("{:02}_{}_neutral.wav", index + 1, kind.slug());
        let samples = render_phrase(kind, StrangeControls::MIDPOINT)?;
        write_wav(&output.join(&name), &samples)?;
        rendered.push((name, samples));
    }

    let type_name = "09_TYPE_all-8-positions_live.wav".to_string();
    let type_reel = render_type_reel()?;
    write_wav(&output.join(&type_name), &type_reel)?;
    rendered.push((type_name, type_reel));

    for (slot, label) in CONTROL_LABELS.into_iter().enumerate() {
        let name = format!("{:02}_{label}_saw_low-high.wav", slot + 10);
        let samples = render_macro_reel(slot)?;
        write_wav(&output.join(&name), &samples)?;
        rendered.push((name, samples));
    }

    write_hashes(&output.join("hashes.tsv"), &rendered)?;
    write_readme(&output.join("README.md"), &gate_rows)?;
    Ok(())
}

fn render_phrase(
    kind: StrangeType,
    controls: StrangeControls,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let note_frames = (0.48 * SAMPLE_RATE as f32) as usize;
    let gap_frames = (0.07 * SAMPLE_RATE as f32) as usize;
    let mut output = Vec::with_capacity(PHRASE_NOTES.len() * 2 * (note_frames + gap_frames));
    for (note_index, note) in PHRASE_NOTES.into_iter().enumerate() {
        let mut voice = StrangeVoice::new(
            kind,
            SAMPLE_RATE as f32,
            midi_frequency(note),
            0x51a7_2026 ^ (u32::from(note) << 8) ^ kind as u32,
            controls,
        )?;
        for frame in 0..note_frames {
            let boundary = boundary_gain(frame, note_frames, 240);
            let sample = voice.sample();
            output.push(boundary * sample.left);
            output.push(boundary * sample.right);
        }
        if note_index + 1 != PHRASE_NOTES.len() {
            output.resize(output.len() + 2 * gap_frames, 0.0);
        }
    }
    Ok(output)
}

fn render_type_reel() -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let segment_frames = (1.25 * SAMPLE_RATE as f32) as usize;
    let total_frames = segment_frames * StrangeType::ALL.len();
    let mut instrument = StrangeInstrument::new(
        SAMPLE_RATE as f32,
        midi_frequency(57),
        0x7400_0000,
        0.0,
        StrangeControls::MIDPOINT,
    )?;
    let mut output = Vec::with_capacity(2 * total_frames);
    for (type_index, kind) in StrangeType::ALL.into_iter().enumerate() {
        if type_index != 0 {
            instrument.set_type(kind)?;
        }
        for frame in 0..segment_frames {
            let absolute_frame = type_index * segment_frames + frame;
            let boundary = boundary_gain(absolute_frame, total_frames, 240);
            let sample = instrument.sample();
            output.push(boundary * sample.left);
            output.push(boundary * sample.right);
        }
    }
    Ok(output)
}

fn render_macro_reel(slot: usize) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let gap_frames = (0.2 * SAMPLE_RATE as f32) as usize;
    let mut output = Vec::new();
    for (state_index, value) in [0.1_f32, 0.9].into_iter().enumerate() {
        let seconds = match slot {
            3 => [11.0_f32, 2.0][state_index],
            4 => 4.0,
            _ => 2.0,
        };
        let segment_frames = (seconds * SAMPLE_RATE as f32) as usize;
        let controls = StrangeControls::MIDPOINT.with(slot, value);
        let mut voice = StrangeVoice::new(
            StrangeType::Saw,
            SAMPLE_RATE as f32,
            midi_frequency(57),
            0x4a00_0000 ^ ((slot as u32) << 8),
            controls,
        )?;
        for frame in 0..segment_frames {
            let boundary = boundary_gain(frame, segment_frames, 240);
            let sample = voice.sample();
            output.push(boundary * sample.left);
            output.push(boundary * sample.right);
        }
        if state_index == 0 {
            output.resize(output.len() + 2 * gap_frames, 0.0);
        }
    }
    Ok(output)
}

fn boundary_gain(frame: usize, total: usize, fade: usize) -> f32 {
    let fade_in = (frame as f32 / fade as f32).min(1.0);
    let fade_out = ((total - 1 - frame) as f32 / fade as f32).min(1.0);
    fade_in.min(fade_out)
}

fn write_gates(
    path: &Path,
    rows: &[moj_sint::strange_lab::StrangeGateEvidence],
) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(
        output,
        "type\tstatus\treason\tnote\tpeak\trms\tdc\tmaximum_jump\tcorrelation\tside_to_mid\tmono_rms\thigh_rate_residual_db\tcyclic_envelope_depth_db"
    )?;
    for row in rows {
        for note in &row.notes {
            let residual = note
                .high_rate_residual_db
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "not_measured".to_string());
            let envelope_depth = note
                .cyclic_envelope_depth_db
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "not_applicable".to_string());
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{:.9}\t{}\t{}",
                row.kind.slug(),
                row.status,
                row.reason,
                note.note,
                note.metrics.peak,
                note.metrics.rms,
                note.metrics.dc,
                note.metrics.maximum_jump,
                note.metrics.correlation,
                note.metrics.side_to_mid,
                note.metrics.mono_rms,
                residual,
                envelope_depth,
            )?;
        }
    }
    Ok(())
}

fn write_structural_macros(
    path: &Path,
    rows: &[(StrangeType, Vec<StructuralMacroEvidence>)],
) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "type\tmacro\tmetric\tvalue\tfloor\tstatus")?;
    for (kind, evidence) in rows {
        for row in evidence {
            writeln!(
                output,
                "{}\t{}\t{}\t{:.9}\t{:.9}\t{}",
                kind.slug(),
                CONTROL_LABELS[row.slot].to_lowercase(),
                row.metric.slug(),
                row.value,
                row.floor,
                if row.value >= row.floor {
                    "pass"
                } else {
                    "reject"
                },
            )?;
        }
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

fn write_readme(
    path: &Path,
    rows: &[moj_sint::strange_lab::StrangeGateEvidence],
) -> std::io::Result<()> {
    let mut output = BufWriter::new(File::create(path)?);
    writeln!(output, "# Strange Oscillator dry gate\n")?;
    writeln!(
        output,
        "This is a disposable monophonic research batch for a possible third Moj Sint model. Nothing here is a production preset or accepted sound.\n"
    )?;
    writeln!(output, "## What is on\n")?;
    writeln!(
        output,
        "One Strange Oscillator voice is on: the selected source topology, seven structural macro stages, two simple COLOR-owned tone poles, prepared 7 Hz DC blocking, fixed internal gain, and short file-boundary fades. TYPE changes use the instrument's prepared 10 ms crossfade. MOTION spans 0.05-50 Hz; CHAOS changes whole-cycle grammar; COLOR can move from a dark fundamental anchor to a bright fifth-harmonic anchor; SPACE changes mid/side width while retaining mono. The source references play MIDI 45, 52, 57, and 64. The TYPE and macro reels hold MIDI 57.\n"
    )?;
    writeln!(output, "## What is off\n")?;
    writeln!(
        output,
        "No production ADSR, filter model, reverb, delay, chorus, compressor, limiter, normalization, master strip, direct-input mix, sampler, drums, Model D, Six-Op PM, synthv1, Yoshimi, FluidSynth, JACK, ALSA, MIDI, or SHR-DAW is running or rendered into these files.\n"
    )?;
    writeln!(output, "## Order\n")?;
    writeln!(
        output,
        "Files 01-08 are separate neutral references for the eight TYPE positions. File 09 is one uninterrupted held note while the single TYPE parameter moves through triangle, saw, pulse, modulated resonator, deformed loop, stochastic breakpoints, scanned string, and register machine. There are no gaps at the seven switches: the audible boundaries are the actual 10 ms live crossfades. Files 10-16 each compare one macro as low, silence, high. Saw is the proof topology because it was the owner's preferred earlier source and makes weak mappings hard to excuse. Fixed gain is shared; there is no loudness matching, so level changes remain visible.\n"
    )?;
    writeln!(output, "## Seven structural macro reels\n")?;
    writeln!(
        output,
        "FORM changes oscillator geometry. WARP applies strong contour/fifth-harmonic deformation. COUPLE cross-modulates the source with an inharmonic partner. MOTION changes a deep cyclic envelope from approximately 0.1 Hz at the demonstrated low point to approximately 25 Hz at the high point. CHAOS turns repeatable cycles into deterministic held-cycle drop/admit decisions. COLOR travels between fundamental and fifth-harmonic spectral anchors as well as changing the two-pole cutoff. SPACE changes stereo side energy. Each file uses low, silence, high at one fixed gain. Ordinary endpoints last two seconds; MOTION gives low eleven seconds so one slow cycle is heard, and CHAOS gives each endpoint four seconds so repeatability versus irregularity is clear.\n"
    )?;
    writeln!(output, "## Automated macro gate\n")?;
    writeln!(
        output,
        "`structural-macros.tsv` measures the domain each control claims to change instead of accepting raw sample residual: harmonic structure for FORM/WARP, periodic or spectral topology for COUPLE, rate plus envelope depth for MOTION, nonrepeating cycle envelopes for CHAOS, harmonic center for COLOR, and side/mid energy for SPACE. All eight TYPE positions must pass all seven rows.\n"
    )?;
    writeln!(output, "## Automated verdicts\n")?;
    for row in rows {
        writeln!(
            output,
            "- `{}`: **{}** — {}",
            row.kind.slug(),
            row.status,
            row.reason
        )?;
    }
    writeln!(
        output,
        "\nA pass only means the source survived the declared engineering gate. Human listening decides whether the mechanism and its macro travel are useful."
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
