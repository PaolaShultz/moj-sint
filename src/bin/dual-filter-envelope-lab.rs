use moj_sint::dual_filter_concept::{
    ConceptControl, ConceptControls, ConceptVariant, DualFilterConceptVoice,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SAMPLE_RATE: u32 = 48_000;
const CONTROL_UPDATE_FRAMES: usize = 64;
const LEAD_SECONDS: f32 = 9.0;
const BASS_SECONDS: f32 = 10.0;

const CONTROL_FILES: [(ConceptControl, &str); 15] = [
    (
        ConceptControl::FilterACutoff,
        "01 LEAD ROTARY 01 - A Cutoff.wav",
    ),
    (
        ConceptControl::FilterAResonance,
        "02 LEAD ROTARY 02 - A Resonance.wav",
    ),
    (
        ConceptControl::FilterAEnvelopeDepth,
        "03 LEAD ROTARY 03 - A Envelope Depth.wav",
    ),
    (
        ConceptControl::FilterBCutoff,
        "04 LEAD ROTARY 04 - B Cutoff.wav",
    ),
    (
        ConceptControl::FilterBResonance,
        "05 LEAD ROTARY 05 - B Resonance.wav",
    ),
    (
        ConceptControl::FilterBEnvelopeDepth,
        "06 LEAD ROTARY 06 - B Envelope Depth.wav",
    ),
    (ConceptControl::Routing, "07 LEAD ROTARY 07 - Routing.wav"),
    (
        ConceptControl::FilterAttack,
        "08 LEAD ROTARY 08 - Filter Attack.wav",
    ),
    (
        ConceptControl::FilterDecay,
        "09 LEAD ROTARY 09 - Filter Decay.wav",
    ),
    (
        ConceptControl::FilterSustain,
        "10 LEAD ROTARY 10 - Filter Sustain.wav",
    ),
    (
        ConceptControl::FilterRelease,
        "11 LEAD ROTARY 11 - Filter Release.wav",
    ),
    (
        ConceptControl::AmpAttack,
        "12 LEAD ROTARY 12 - Amp Attack.wav",
    ),
    (
        ConceptControl::AmpDecay,
        "13 LEAD ROTARY 13 - Amp Decay.wav",
    ),
    (
        ConceptControl::AmpSustain,
        "14 LEAD ROTARY 14 - Amp Sustain.wav",
    ),
    (
        ConceptControl::AmpRelease,
        "15 LEAD ROTARY 15 - Amp Release.wav",
    ),
];

const BASS_FILES: [&str; 4] = [
    "17 BASS 01 - Serial Cutoff and Resonance.wav",
    "18 BASS 02 - Countermotion Growl and Routing.wav",
    "19 BASS 03 - Envelope Punch.wav",
    "20 BASS 04 - Topology Push.wav",
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dual-filter-envelope-lab: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let destination = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: dual-filter-envelope-lab OUTPUT_DIRECTORY")?;
    if destination.exists() {
        return Err(format!("refusing existing output path {}", destination.display()).into());
    }
    fs::create_dir(&destination)?;

    for (control, file_name) in CONTROL_FILES {
        write_wav(&destination.join(file_name), &render_lead_control(control)?)?;
    }
    write_wav(
        &destination.join("16 LEAD PUSH - Serial Parallel Topology.wav"),
        &render_lead_topology()?,
    )?;
    for (program, file_name) in BASS_FILES.into_iter().enumerate() {
        write_wav(
            &destination.join(file_name),
            &render_bass(program.try_into().expect("four bass programs"))?,
        )?;
    }

    println!(
        "wrote 20 moving-control WAV files to {}",
        destination.display()
    );
    Ok(())
}

fn render_lead_control(control: ConceptControl) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut controls = lead_controls();
    if control == ConceptControl::FilterRelease {
        controls.set(ConceptControl::AmpRelease, 0.58);
    }
    let baseline = controls.get(control);
    let mut voice =
        DualFilterConceptVoice::new(SAMPLE_RATE as f32, ConceptVariant::SweetSerial, controls)?;
    let frames = seconds_to_frames(LEAD_SECONDS);
    let repeated_notes = control.index() >= ConceptControl::FilterAttack.index();
    let period = seconds_to_frames(1.5);
    let gate = seconds_to_frames(0.9);
    let mut samples = Vec::with_capacity(frames);

    if !repeated_notes {
        voice.note_on(60, 0.9);
    }
    for frame in 0..frames {
        if repeated_notes {
            let cycle = frame % period;
            if cycle == 0 {
                voice.note_on(60, 0.9);
            } else if cycle == gate {
                voice.note_off();
            }
        } else if frame == seconds_to_frames(8.0) {
            voice.note_off();
        }

        if frame % CONTROL_UPDATE_FRAMES == 0 {
            voice.set_control(control, lead_motion(control, frame, baseline));
        }
        samples.push(voice.sample());
    }
    Ok(samples)
}

fn render_lead_topology() -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut voice = DualFilterConceptVoice::new(
        SAMPLE_RATE as f32,
        ConceptVariant::SweetSerial,
        lead_controls(),
    )?;
    let frames = seconds_to_frames(LEAD_SECONDS);
    let pushes = [
        seconds_to_frames(2.0),
        seconds_to_frames(4.0),
        seconds_to_frames(6.0),
    ];
    let mut samples = Vec::with_capacity(frames);
    voice.note_on(60, 0.9);
    for frame in 0..frames {
        if pushes.contains(&frame) {
            voice.toggle_topology();
        }
        if frame == seconds_to_frames(8.0) {
            voice.note_off();
        }
        samples.push(voice.sample());
    }
    Ok(samples)
}

fn render_bass(program: u8) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let (variant, mut controls, notes) = bass_setup(program);
    let mut voice = DualFilterConceptVoice::new(SAMPLE_RATE as f32, variant, controls)?;
    let frames = seconds_to_frames(BASS_SECONDS);
    let period = seconds_to_frames(0.75);
    let gate = seconds_to_frames(0.55);
    let stop = seconds_to_frames(9.0);
    let mut samples = Vec::with_capacity(frames);

    for frame in 0..frames {
        if frame < stop {
            let cycle = frame % period;
            if cycle == 0 {
                voice.note_on(notes[(frame / period) % notes.len()], 0.94);
            } else if cycle == gate {
                voice.note_off();
            }
        }
        if frame % CONTROL_UPDATE_FRAMES == 0 {
            automate_bass(
                program,
                frame as f32 / stop as f32,
                &mut controls,
                &mut voice,
            );
        }
        if program == 3
            && [
                seconds_to_frames(2.25),
                seconds_to_frames(4.5),
                seconds_to_frames(6.75),
            ]
            .contains(&frame)
        {
            voice.toggle_topology();
        }
        samples.push(voice.sample());
    }
    Ok(samples)
}

fn lead_controls() -> ConceptControls {
    controls_from([
        (ConceptControl::FilterACutoff, 0.48),
        (ConceptControl::FilterAResonance, 0.62),
        (ConceptControl::FilterAEnvelopeDepth, 0.67),
        (ConceptControl::FilterBCutoff, 0.68),
        (ConceptControl::FilterBResonance, 0.38),
        (ConceptControl::FilterBEnvelopeDepth, 0.56),
        (ConceptControl::Routing, 0.18),
        (ConceptControl::FilterAttack, 0.14),
        (ConceptControl::FilterDecay, 0.39),
        (ConceptControl::FilterSustain, 0.34),
        (ConceptControl::FilterRelease, 0.42),
        (ConceptControl::AmpAttack, 0.08),
        (ConceptControl::AmpDecay, 0.36),
        (ConceptControl::AmpSustain, 0.68),
        (ConceptControl::AmpRelease, 0.40),
    ])
}

fn bass_setup(program: u8) -> (ConceptVariant, ConceptControls, &'static [u8]) {
    const SERIAL_NOTES: &[u8] = &[29, 29, 36, 32, 29, 41, 36, 32];
    const GROWL_NOTES: &[u8] = &[31, 31, 38, 34, 31, 43, 38, 34];
    const PUNCH_NOTES: &[u8] = &[36, 36, 34, 29, 36, 41, 34, 32];
    match program {
        0 => {
            let mut controls = lead_controls();
            controls.set(ConceptControl::FilterACutoff, 0.32);
            controls.set(ConceptControl::FilterBCutoff, 0.46);
            controls.set(ConceptControl::AmpAttack, 0.02);
            controls.set(ConceptControl::AmpRelease, 0.24);
            (ConceptVariant::SweetSerial, controls, SERIAL_NOTES)
        }
        1 => {
            let controls = controls_from([
                (ConceptControl::FilterACutoff, 0.30),
                (ConceptControl::FilterAResonance, 0.70),
                (ConceptControl::FilterAEnvelopeDepth, 0.76),
                (ConceptControl::FilterBCutoff, 0.50),
                (ConceptControl::FilterBResonance, 0.55),
                (ConceptControl::FilterBEnvelopeDepth, 0.18),
                (ConceptControl::Routing, 0.46),
                (ConceptControl::FilterAttack, 0.06),
                (ConceptControl::FilterDecay, 0.34),
                (ConceptControl::FilterSustain, 0.30),
                (ConceptControl::FilterRelease, 0.34),
                (ConceptControl::AmpAttack, 0.02),
                (ConceptControl::AmpDecay, 0.30),
                (ConceptControl::AmpSustain, 0.62),
                (ConceptControl::AmpRelease, 0.26),
            ]);
            (ConceptVariant::CounterMotion, controls, GROWL_NOTES)
        }
        2 => {
            let mut controls = lead_controls();
            controls.set(ConceptControl::FilterACutoff, 0.30);
            controls.set(ConceptControl::FilterBCutoff, 0.48);
            controls.set(ConceptControl::FilterAttack, 0.01);
            controls.set(ConceptControl::AmpAttack, 0.01);
            controls.set(ConceptControl::AmpRelease, 0.18);
            (ConceptVariant::SweetSerial, controls, PUNCH_NOTES)
        }
        3 => {
            let mut controls = lead_controls();
            controls.set(ConceptControl::FilterACutoff, 0.34);
            controls.set(ConceptControl::FilterBCutoff, 0.50);
            controls.set(ConceptControl::AmpAttack, 0.02);
            controls.set(ConceptControl::AmpRelease, 0.24);
            (ConceptVariant::SweetSerial, controls, SERIAL_NOTES)
        }
        _ => unreachable!("four bass programs"),
    }
}

fn automate_bass(
    program: u8,
    phase: f32,
    controls: &mut ConceptControls,
    voice: &mut DualFilterConceptVoice,
) {
    let triangle = 1.0 - (2.0 * phase.clamp(0.0, 1.0) - 1.0).abs();
    let set =
        |control, value, controls: &mut ConceptControls, voice: &mut DualFilterConceptVoice| {
            let value = quantize(value);
            controls.set(control, value);
            voice.set_control(control, value);
        };
    match program {
        0 => {
            set(
                ConceptControl::FilterACutoff,
                0.22 + 0.36 * triangle,
                controls,
                voice,
            );
            set(
                ConceptControl::FilterAResonance,
                0.76 - 0.30 * triangle,
                controls,
                voice,
            );
        }
        1 => {
            set(
                ConceptControl::Routing,
                0.08 + 0.84 * triangle,
                controls,
                voice,
            );
            set(
                ConceptControl::FilterBEnvelopeDepth,
                0.82 - 0.70 * triangle,
                controls,
                voice,
            );
        }
        2 => {
            set(
                ConceptControl::FilterDecay,
                0.08 + 0.58 * triangle,
                controls,
                voice,
            );
            set(
                ConceptControl::FilterSustain,
                0.68 - 0.56 * triangle,
                controls,
                voice,
            );
            set(
                ConceptControl::AmpDecay,
                0.12 + 0.34 * triangle,
                controls,
                voice,
            );
        }
        3 => {
            set(
                ConceptControl::FilterACutoff,
                0.26 + 0.22 * triangle,
                controls,
                voice,
            );
            set(
                ConceptControl::FilterBCutoff,
                0.42 + 0.22 * triangle,
                controls,
                voice,
            );
        }
        _ => unreachable!("four bass programs"),
    }
}

fn lead_motion(control: ConceptControl, frame: usize, baseline: f32) -> f32 {
    let seconds = frame as f32 / SAMPLE_RATE as f32;
    let high = if matches!(
        control,
        ConceptControl::FilterAttack
            | ConceptControl::FilterDecay
            | ConceptControl::FilterRelease
            | ConceptControl::AmpAttack
            | ConceptControl::AmpDecay
            | ConceptControl::AmpRelease
    ) {
        0.76
    } else {
        0.95
    };
    let value = if seconds < 0.75 {
        baseline
    } else if seconds < 2.75 {
        lerp(baseline, 0.03, (seconds - 0.75) / 2.0)
    } else if seconds < 5.25 {
        lerp(0.03, high, (seconds - 2.75) / 2.5)
    } else if seconds < 7.25 {
        lerp(high, baseline, (seconds - 5.25) / 2.0)
    } else {
        baseline
    };
    quantize(value)
}

fn controls_from(values: [(ConceptControl, f32); 15]) -> ConceptControls {
    let mut controls = ConceptControls::MIDPOINT;
    for (control, value) in values {
        controls.set(control, value);
    }
    controls
}

fn quantize(value: f32) -> f32 {
    (value.clamp(0.0, 1.0) * 127.0).round() / 127.0
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount.clamp(0.0, 1.0)
}

fn seconds_to_frames(seconds: f32) -> usize {
    (seconds * SAMPLE_RATE as f32).round() as usize
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
        writer.write_sample(*sample)?;
    }
    writer.finalize()
}
