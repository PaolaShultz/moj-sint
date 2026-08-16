use assert_no_alloc::assert_no_alloc;
use moj_sint::envelope::AdsrConfig;
use moj_sint::micro_machine::{
    MAX_NODES, MAX_OSCILLATORS, MicroMachineGraph, MicroMachineVoice, SwarmControls,
};
use moj_sint::micro_machine_lab::{measure_high_rate_residual, midi_frequency};

const GRAPH: &str = include_str!("../experiments/swarm-micro-machine-v1.toml");
const SAMPLE_RATE: f32 = 48_000.0;

fn render(note: u8, controls: SwarmControls, frames: usize) -> Vec<[f32; 2]> {
    let graph = MicroMachineGraph::parse(GRAPH).expect("valid experimental graph");
    let mut machine = graph
        .compile(SAMPLE_RATE, midi_frequency(note), controls)
        .expect("compiled experimental graph");
    let mut output = vec![[0.0; 2]; frames];
    machine.render_block(&mut output);
    output
}

fn rms(samples: &[[f32; 2]]) -> f64 {
    (samples
        .iter()
        .flat_map(|frame| frame.iter())
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / (2 * samples.len()) as f64)
        .sqrt()
}

#[test]
fn strict_graph_parsing_and_typed_validation_reject_bad_descriptions() {
    let graph = MicroMachineGraph::parse(GRAPH).unwrap();
    assert_eq!(
        graph.control_names(),
        [
            "MASS", "DETUNE", "SPREAD", "SHAPE", "BITE", "MOTION", "COLOR", "SPACE"
        ]
    );

    let unknown_field = GRAPH.replacen("schema_version = 1", "schema_version = 1\nmystery = 2", 1);
    assert!(MicroMachineGraph::parse(&unknown_field).is_err());

    let unknown_node = GRAPH.replacen("phase_swarm", "entire_supersaw", 1);
    assert!(
        MicroMachineGraph::parse(&unknown_node)
            .unwrap()
            .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
            .unwrap_err()
            .to_string()
            .contains("unknown node type")
    );

    let incompatible = GRAPH.replacen("waves.waves", "phases.phases", 1);
    assert!(
        MicroMachineGraph::parse(&incompatible)
            .unwrap()
            .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
            .unwrap_err()
            .to_string()
            .contains("incompatible port")
    );

    let duplicate = GRAPH.replacen("id = \"waves\"", "id = \"phases\"", 1);
    assert!(
        MicroMachineGraph::parse(&duplicate)
            .unwrap()
            .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
            .unwrap_err()
            .to_string()
            .contains("duplicate node")
    );

    let invalid_cycle = r#"
schema_version = 1
name = "cycle"
seed = 1
controls = ["MASS", "DETUNE", "SPREAD", "SHAPE", "BITE", "MOTION", "COLOR", "SPACE"]
output = "guard.output"

[[nodes]]
id = "tone"
type = "spectral_tilt"
input = "drive.stereo"
min_cutoff_hz = 900.0
max_cutoff_hz = 18000.0

[[nodes]]
id = "drive"
type = "bounded_drive"
input = "tone.stereo"
drive = 2.0

[[nodes]]
id = "guard"
type = "output_guard"
input = "drive.stereo"
ceiling = 0.9
"#;
    assert!(
        MicroMachineGraph::parse(invalid_cycle)
            .unwrap()
            .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );

    let non_finite = GRAPH.replacen("drive = 2.8", "drive = nan", 1);
    assert!(
        MicroMachineGraph::parse(&non_finite)
            .unwrap()
            .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
            .is_err()
    );

    let too_many_oscillators = GRAPH.replacen(
        "max_oscillators = 9",
        &format!("max_oscillators = {}", MAX_OSCILLATORS + 1),
        1,
    );
    assert!(
        MicroMachineGraph::parse(&too_many_oscillators)
            .unwrap()
            .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
            .unwrap_err()
            .to_string()
            .contains("resource limit")
    );
}

#[test]
fn compilation_is_deterministic_and_resource_bounded() {
    let graph = MicroMachineGraph::parse(GRAPH).unwrap();
    let first = graph
        .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
        .unwrap();
    let second = graph
        .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
        .unwrap();
    assert_eq!(first.plan_fingerprint(), second.plan_fingerprint());
    let changed =
        MicroMachineGraph::parse(&GRAPH.replacen("coupling = 0.38", "coupling = 0.39", 1))
            .unwrap()
            .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
            .unwrap();
    assert_ne!(first.plan_fingerprint(), changed.plan_fingerprint());
    assert_eq!(
        first.scheduled_node_ids().collect::<Vec<_>>(),
        [
            "phases", "waves", "mass_mix", "color", "bite", "space", "guard"
        ]
    );
    let usage = first.resource_usage();
    assert_eq!(usage.nodes, 7);
    assert_eq!(usage.edges, 6);
    assert_eq!(usage.oscillator_slots, 9);
    assert!(usage.nodes <= MAX_NODES);
    assert!(usage.state_bytes > 0);
    assert!(usage.work_units_per_sample > 0);
    assert!(usage.within_limits());
}

#[test]
fn rendering_and_rapid_live_controls_are_fixed_finite_bounded_and_resettable() {
    let graph = MicroMachineGraph::parse(GRAPH).unwrap();
    let mut first = graph
        .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
        .unwrap();
    let mut second = graph
        .compile(SAMPLE_RATE, 220.0, SwarmControls::NEUTRAL)
        .unwrap();
    let first_address = first.state_address();
    let mut left = [[0.0_f32; 2]; 256];
    let mut right = [[0.0_f32; 2]; 256];

    assert_no_alloc(|| {
        for step in 0..128 {
            let value = if step % 2 == 0 { 0.0 } else { 1.0 };
            first.set_controls(SwarmControls::uniform(value)).unwrap();
            second.set_controls(SwarmControls::uniform(value)).unwrap();
            first.render_block(&mut left);
            second.render_block(&mut right);
            assert_eq!(left, right);
        }
    });
    assert_eq!(first.state_address(), first_address);
    assert!(first.maximum_internal_magnitude().is_finite());
    assert!(first.maximum_internal_magnitude() <= 4.0);
    assert!(
        left.iter()
            .flatten()
            .all(|sample| sample.is_finite() && sample.abs() <= 0.92)
    );

    first.reset();
    second.reset();
    first.render_block(&mut left);
    second.render_block(&mut right);
    assert_eq!(left, right);
}

#[test]
fn low_mid_high_notes_chords_stereo_and_mono_fold_are_usable() {
    for note in [36, 60, 84] {
        let samples = render(note, SwarmControls::NEUTRAL, 16_384);
        assert!(
            samples
                .iter()
                .flatten()
                .all(|sample| sample.is_finite() && sample.abs() <= 0.92)
        );
        assert!(rms(&samples[2_048..]) > 0.01, "note={note}");
    }

    let graph = MicroMachineGraph::parse(GRAPH).unwrap();
    let mut voices = [48_u8, 55, 62].map(|note| {
        graph
            .compile(SAMPLE_RATE, midi_frequency(note), SwarmControls::PAD)
            .unwrap()
    });
    let mut stereo_difference = 0.0_f64;
    let mut mono_energy = 0.0_f64;
    let mut peak = 0.0_f32;
    for _ in 0..24_000 {
        let mut frame = [0.0_f32; 2];
        for voice in &mut voices {
            let sample = voice.sample();
            frame[0] += sample[0] / 3.0_f32.sqrt();
            frame[1] += sample[1] / 3.0_f32.sqrt();
        }
        frame[0] = frame[0].clamp(-1.0, 1.0);
        frame[1] = frame[1].clamp(-1.0, 1.0);
        stereo_difference += f64::from(frame[0] - frame[1]).powi(2);
        mono_energy += f64::from(0.5 * (frame[0] + frame[1])).powi(2);
        peak = peak.max(frame[0].abs()).max(frame[1].abs());
    }
    assert!(stereo_difference / 24_000.0 > 1.0e-5);
    assert!(mono_energy / 24_000.0 > 1.0e-4);
    assert!(peak <= 1.0);
}

#[test]
fn mass_normalization_keeps_population_changes_in_one_useful_gain_band() {
    let levels = [0.0, 0.5, 1.0].map(|mass| {
        let controls = SwarmControls {
            mass,
            ..SwarmControls::NEUTRAL
        };
        rms(&render(57, controls, 48_000)[4_096..])
    });
    let minimum = levels.into_iter().fold(f64::INFINITY, f64::min);
    let maximum = levels.into_iter().fold(0.0_f64, f64::max);
    assert!(minimum > 0.01, "levels={levels:?}");
    assert!(maximum / minimum < 1.8, "levels={levels:?}");
}

#[test]
fn every_named_timbral_control_changes_the_dry_machine_materially() {
    for control in 0..8 {
        let low = render(
            57,
            SwarmControls::NEUTRAL.with(control, 0.08).unwrap(),
            24_000,
        );
        let high = render(
            57,
            SwarmControls::NEUTRAL.with(control, 0.92).unwrap(),
            24_000,
        );
        let residual = low
            .iter()
            .zip(high)
            .skip(2_048)
            .flat_map(|(left, right)| {
                [f64::from(left[0] - right[0]), f64::from(left[1] - right[1])]
            })
            .map(|difference| difference * difference)
            .sum::<f64>()
            / (2 * (24_000 - 2_048)) as f64;
        assert!(
            residual.sqrt() > 0.005,
            "control={control} residual={}",
            residual.sqrt()
        );
    }
}

#[test]
fn outer_adsr_cleans_up_notes_and_reset_replays_the_seeded_machine() {
    let graph = MicroMachineGraph::parse(GRAPH).unwrap();
    let adsr = AdsrConfig::new(0.002, 0.010, 0.65, 0.010).unwrap();
    let mut allocation_voice =
        MicroMachineVoice::compile(&graph, SAMPLE_RATE, 220.0, SwarmControls::LEAD, adsr).unwrap();
    allocation_voice.note_on(0.8);
    assert_no_alloc(|| {
        let mut sum = 0.0_f32;
        for _ in 0..4_096 {
            let frame = allocation_voice.sample();
            sum += frame[0] + frame[1];
        }
        std::hint::black_box(sum);
    });

    let mut voice =
        MicroMachineVoice::compile(&graph, SAMPLE_RATE, 220.0, SwarmControls::LEAD, adsr).unwrap();
    voice.note_on(0.8);
    let first: Vec<_> = (0..2_048).map(|_| voice.sample()).collect();
    voice.note_off();
    for _ in 0..1_024 {
        voice.sample();
    }
    assert!(voice.is_idle());
    assert_eq!(voice.sample(), [0.0, 0.0]);

    voice.reset();
    voice.note_on(0.8);
    let replay: Vec<_> = (0..2_048).map(|_| voice.sample()).collect();
    assert_eq!(first, replay);
    voice.panic();
    assert_eq!(voice.sample(), [0.0, 0.0]);
}

#[test]
fn high_rate_comparison_records_bounded_alias_error_evidence() {
    let graph = MicroMachineGraph::parse(GRAPH).unwrap();
    for note in [36, 60, 84] {
        let residual =
            measure_high_rate_residual(&graph, SwarmControls::NEUTRAL, note, 8_192).unwrap();
        assert!(
            residual.is_finite() && residual <= 0.0,
            "note={note} residual={residual}"
        );
    }
}
