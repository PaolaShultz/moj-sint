use moj_sint::envelope::AdsrConfig;
use moj_sint::pressure_chain::{
    PressureArticulation, PressureChainControl, PressureChainControls, PressureChainTopology,
    PressureChainVoice, measure_high_rate_residual,
};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SAMPLE_RATE: u32 = 48_000;
const TOTAL_FRAMES: usize = 12 * SAMPLE_RATE as usize;
const STEP_FRAMES: usize = 9_000;
const PLAY_STEPS: usize = 56;
const UPDATE_FRAMES: usize = 64;
const NOTES: [u8; 16] = [
    40, 40, 47, 43, 40, 52, 47, 43, 38, 38, 45, 41, 38, 50, 45, 41,
];
const VELOCITIES: [f32; 16] = [
    0.72, 1.0, 0.76, 0.90, 1.0, 0.74, 0.92, 0.70, 0.74, 1.0, 0.72, 0.88, 1.0, 0.76, 0.94, 0.70,
];
const SLIDE_INTO_STEP: [bool; 16] = [
    false, false, true, false, false, true, true, false, false, false, true, false, false, true,
    false, true,
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pressure-chain-lab: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: pressure-chain-lab OUTPUT_DIRECTORY")?;
    if destination.exists() {
        return Err(format!("refusing existing output path {}", destination.display()).into());
    }
    fs::create_dir(&destination)?;

    let files = [
        (PressureChainTopology::DeepCascade, "01 Deep Cascade.wav"),
        (PressureChainTopology::BodyTap, "02 Body Tap.wav"),
        (PressureChainTopology::CrossFeed, "03 Cross Feed.wav"),
    ];
    let mut report = String::from(
        "Pressure Chain listening gate\nsource: docs/ACID_CHAIN_RESEARCH.md\nformat: 48000 Hz stereo float32 dual-mono\nperformance: identical 56-step note/velocity/slide score and cutoff motion\n\n",
    );
    for (topology, file) in files {
        let samples = render_score(topology)?;
        write_wav(&destination.join(file), &samples)?;
        let metrics = measure(&samples);
        writeln!(&mut report, "topology: {}", topology.title())?;
        writeln!(&mut report, "file: {file}")?;
        writeln!(&mut report, "peak: {:.9}", metrics.peak)?;
        writeln!(&mut report, "rms: {:.9}", metrics.rms)?;
        writeln!(&mut report, "dc: {:.9}", metrics.dc)?;
        writeln!(&mut report, "maximum_jump: {:.9}", metrics.maximum_jump)?;
        writeln!(&mut report, "sample_hash: {:016x}", metrics.hash)?;
        for note in [36, 60, 84] {
            let residual =
                measure_high_rate_residual(topology, PressureChainControls::START, note, 8_192)?;
            writeln!(&mut report, "residual_db_midi_{note}: {residual:.6}")?;
        }
        report.push('\n');
    }
    fs::write(destination.join("measurements.txt"), report)?;
    println!(
        "wrote three topology listening files and measurements to {}",
        destination.display()
    );
    Ok(())
}

fn render_score(
    topology: PressureChainTopology,
) -> Result<Vec<[f32; 2]>, Box<dyn std::error::Error>> {
    let adsr = AdsrConfig::new(0.003, 0.055, 0.66, 0.120)?;
    let mut controls = PressureChainControls::START;
    let mut voice = PressureChainVoice::new(SAMPLE_RATE as f32, topology, controls, adsr)?;
    let mut samples = Vec::with_capacity(TOTAL_FRAMES);
    let play_frames = PLAY_STEPS * STEP_FRAMES;

    for frame in 0..TOTAL_FRAMES {
        if frame < play_frames {
            let step_number = frame / STEP_FRAMES;
            let step = step_number % NOTES.len();
            let within = frame % STEP_FRAMES;
            if within == 0 {
                let articulation = if SLIDE_INTO_STEP[step] {
                    PressureArticulation::Slide
                } else {
                    PressureArticulation::Trigger
                };
                voice.note_on(NOTES[step], VELOCITIES[step], articulation);
            }
            let next_step = (step + 1) % NOTES.len();
            if within == STEP_FRAMES * 3 / 4 && !SLIDE_INTO_STEP[next_step] {
                voice.note_off();
            }
            if frame % UPDATE_FRAMES == 0 {
                let progress = frame as f32 / play_frames as f32;
                let triangle = 1.0 - (2.0 * progress - 1.0).abs();
                controls.set(PressureChainControl::Cutoff, 0.24 + 0.34 * triangle);
                controls.set(PressureChainControl::Resonance, 0.58 + 0.18 * progress);
                voice.set_controls(controls);
            }
        } else if frame == play_frames {
            voice.note_off();
        }
        samples.push(voice.sample());
    }
    Ok(samples)
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    peak: f64,
    rms: f64,
    dc: f64,
    maximum_jump: f64,
    hash: u64,
}

fn measure(samples: &[[f32; 2]]) -> Metrics {
    let mut peak = 0.0_f64;
    let mut energy = 0.0_f64;
    let mut sum = 0.0_f64;
    let mut maximum_jump = 0.0_f64;
    let mut previous = 0.0_f64;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for frame in samples {
        let value = f64::from(frame[0]);
        peak = peak.max(value.abs());
        energy += value * value;
        sum += value;
        maximum_jump = maximum_jump.max((value - previous).abs());
        previous = value;
        for sample in frame {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    Metrics {
        peak,
        rms: (energy / samples.len() as f64).sqrt(),
        dc: sum / samples.len() as f64,
        maximum_jump,
        hash,
    }
}

fn write_wav(path: &Path, samples: &[[f32; 2]]) -> Result<(), hound::Error> {
    let specification = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, specification)?;
    for frame in samples {
        writer.write_sample(frame[0])?;
        writer.write_sample(frame[1])?;
    }
    writer.finalize()
}
