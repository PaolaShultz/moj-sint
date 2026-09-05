use assert_no_alloc::assert_no_alloc;
use moj_sint::envelope::AdsrConfig;
use moj_sint::pressure_chain::{
    PressureArticulation, PressureChainControl, PressureChainControls, PressureChainTopology,
    PressureChainVoice, PressureRenderSpec, measure_high_rate_residual, render_note,
};

const SAMPLE_RATE: f32 = 48_000.0;

fn adsr() -> AdsrConfig {
    AdsrConfig::new(0.004, 0.090, 0.72, 0.080).unwrap()
}

fn render(topology: PressureChainTopology, controls: PressureChainControls) -> Vec<[f32; 2]> {
    render_note(
        topology,
        controls,
        PressureRenderSpec {
            adsr: adsr(),
            sample_rate: 48_000,
            note: 40,
            velocity: 0.9,
            gate_seconds: 0.38,
            total_seconds: 0.62,
        },
    )
    .unwrap()
}

fn residual(left: &[[f32; 2]], right: &[[f32; 2]]) -> f64 {
    assert_eq!(left.len(), right.len());
    (left
        .iter()
        .zip(right)
        .flat_map(|(a, b)| [f64::from(a[0] - b[0]), f64::from(a[1] - b[1])])
        .map(|sample| sample * sample)
        .sum::<f64>()
        / (left.len() * 2) as f64)
        .sqrt()
}

#[test]
fn experiment_has_exactly_eight_distinct_timbral_roles_plus_external_adsr() {
    assert_eq!(PressureChainControl::ALL.len(), 8);
    assert_eq!(
        PressureChainControl::ALL.map(PressureChainControl::label),
        [
            "SOURCE",
            "SHAPE",
            "CUTOFF",
            "RESONANCE",
            "SWEEP",
            "DECAY",
            "PRESSURE",
            "BITE",
        ]
    );
}

#[test]
fn every_topology_is_deterministic_finite_bounded_dual_mono_and_allocation_free() {
    for topology in PressureChainTopology::ALL {
        let first = render(topology, PressureChainControls::START);
        let second = render(topology, PressureChainControls::START);
        assert_eq!(first, second, "topology={topology:?}");
        assert!(first.iter().any(|frame| frame[0].abs() > 1.0e-4));
        assert!(first.iter().all(|frame| {
            frame[0].is_finite()
                && frame[1].is_finite()
                && frame[0].abs() <= 0.95
                && frame[1].abs() <= 0.95
                && frame[0] == frame[1]
        }));

        let mut voice =
            PressureChainVoice::new(SAMPLE_RATE, topology, PressureChainControls::START, adsr())
                .unwrap();
        assert_no_alloc(|| {
            voice.note_on(40, 1.0, PressureArticulation::Trigger);
            for frame in 0..4_096 {
                if frame == 1_024 {
                    voice.set_control(PressureChainControl::Cutoff, 0.9);
                    voice.set_control(PressureChainControl::Bite, 0.8);
                }
                if frame == 2_048 {
                    voice.note_on(47, 0.8, PressureArticulation::Slide);
                }
                std::hint::black_box(voice.sample());
            }
            voice.note_off();
            for _ in 0..4_096 {
                std::hint::black_box(voice.sample());
            }
            voice.reset();
        });
        assert_eq!(voice.sample(), [0.0; 2]);
    }
}

#[test]
fn three_candidates_are_topologically_and_audibly_distinct() {
    let rendered =
        PressureChainTopology::ALL.map(|topology| render(topology, PressureChainControls::START));
    assert!(residual(&rendered[0], &rendered[1]) > 0.002);
    assert!(residual(&rendered[1], &rendered[2]) > 0.002);
    assert!(residual(&rendered[0], &rendered[2]) > 0.002);
}

#[test]
fn every_timbral_control_materially_changes_the_output() {
    let baseline = render(
        PressureChainTopology::CrossFeed,
        PressureChainControls::START,
    );
    for control in PressureChainControl::ALL {
        let mut controls = PressureChainControls::START;
        controls.set(control, if control.index() % 2 == 0 { 0.08 } else { 0.92 });
        let changed = render(PressureChainTopology::CrossFeed, controls);
        assert!(
            residual(&baseline, &changed) > 0.000_2,
            "control={} residual={}",
            control.label(),
            residual(&baseline, &changed)
        );
    }
}

#[test]
fn pressure_is_stateful_and_repeated_strikes_accumulate_then_relax() {
    let mut voice = PressureChainVoice::new(
        SAMPLE_RATE,
        PressureChainTopology::DeepCascade,
        PressureChainControls::START,
        adsr(),
    )
    .unwrap();
    voice.note_on(40, 1.0, PressureArticulation::Trigger);
    for _ in 0..2_400 {
        voice.sample();
    }
    let first = voice.pressure_level();
    voice.note_on(40, 1.0, PressureArticulation::Trigger);
    let second = voice.pressure_level();
    assert!(second > first + 0.05, "first={first} second={second}");
    for _ in 0..48_000 {
        voice.sample();
    }
    assert!(voice.pressure_level() < second * 0.5);
}

#[test]
fn slide_slews_pitch_without_restarting_the_amp_contour() {
    let mut voice = PressureChainVoice::new(
        SAMPLE_RATE,
        PressureChainTopology::BodyTap,
        PressureChainControls::START,
        AdsrConfig::new(0.100, 0.200, 0.8, 0.100).unwrap(),
    )
    .unwrap();
    voice.note_on(40, 0.8, PressureArticulation::Trigger);
    for _ in 0..2_400 {
        voice.sample();
    }
    let before_level = voice.amp_level();
    let before_frequency = voice.current_frequency_hz();
    voice.note_on(52, 0.8, PressureArticulation::Slide);
    assert_eq!(voice.amp_level(), before_level);
    assert_eq!(voice.current_frequency_hz(), before_frequency);
    for _ in 0..2_400 {
        voice.sample();
    }
    assert!(voice.current_frequency_hz() > before_frequency * 1.5);
    assert!(voice.current_frequency_hz() < voice.target_frequency_hz());
}

#[test]
fn release_reaches_exact_silence_and_alias_measurement_is_explicit() {
    let samples = render(
        PressureChainTopology::DeepCascade,
        PressureChainControls::START,
    );
    assert!(
        samples
            .last_chunk::<256>()
            .unwrap()
            .iter()
            .all(|frame| *frame == [0.0; 2])
    );

    for topology in PressureChainTopology::ALL {
        for note in [36, 60, 84] {
            let first =
                measure_high_rate_residual(topology, PressureChainControls::START, note, 4_096)
                    .unwrap();
            let second =
                measure_high_rate_residual(topology, PressureChainControls::START, note, 4_096)
                    .unwrap();
            assert_eq!(first, second);
            assert!(
                first.is_finite() && first <= 0.0,
                "{topology:?} {note} {first}"
            );
        }
    }
}
