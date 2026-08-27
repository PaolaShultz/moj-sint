use assert_no_alloc::assert_no_alloc;
use moj_sint::dual_filter_concept::{
    ConceptControl, ConceptControls, ConceptTopology, ConceptVariant, DualFilterConceptVoice,
    measure_high_rate_residual, render_concept,
};

#[test]
fn concept_exposes_exactly_fifteen_direct_continuous_controls() {
    assert_eq!(ConceptControl::ALL.len(), 15);
    let mut labels = ConceptControl::ALL.map(ConceptControl::label).to_vec();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), 15);

    assert_eq!(ConceptControl::ALL[0].label(), "A CUTOFF");
    assert_eq!(ConceptControl::ALL[6].label(), "STRUCTURE");
    assert_eq!(ConceptControl::ALL[7].label(), "F ATTACK");
    assert_eq!(ConceptControl::ALL[11].label(), "A ATTACK");
    assert_eq!(ConceptControl::ALL[14].label(), "A RELEASE");
}

#[test]
fn one_push_action_toggles_only_the_filter_topology() {
    let mut voice = DualFilterConceptVoice::new(
        48_000.0,
        ConceptVariant::SweetSerial,
        ConceptControls::MIDPOINT,
    )
    .unwrap();
    assert_eq!(voice.topology(), ConceptTopology::Serial);
    voice.toggle_topology();
    assert_eq!(voice.topology(), ConceptTopology::Parallel);
    voice.toggle_topology();
    assert_eq!(voice.topology(), ConceptTopology::Serial);
    assert_eq!(voice.controls(), ConceptControls::MIDPOINT);
}

#[test]
fn render_path_is_deterministic_finite_bounded_and_allocation_free() {
    for variant in ConceptVariant::ALL {
        let first =
            render_concept(variant, ConceptControls::MIDPOINT, 48_000, 48, 0.35, 0.8).unwrap();
        let second =
            render_concept(variant, ConceptControls::MIDPOINT, 48_000, 48, 0.35, 0.8).unwrap();
        assert_eq!(first, second, "variant={variant:?}");
        assert!(
            first
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0),
            "variant={variant:?}"
        );
        assert!(first.iter().any(|sample| sample.abs() > 1.0e-4));

        let mut voice =
            DualFilterConceptVoice::new(48_000.0, variant, ConceptControls::MIDPOINT).unwrap();
        assert_no_alloc(|| {
            voice.note_on(48, 0.9);
            for frame in 0..2_048 {
                if frame == 512 {
                    voice.set_control(ConceptControl::FilterACutoff, 0.9);
                    voice.set_control(ConceptControl::Routing, 0.1);
                }
                if frame == 1_024 {
                    voice.toggle_topology();
                }
                let sample = voice.sample();
                assert!(sample.is_finite() && sample.abs() <= 1.0);
            }
            voice.note_off();
            for _ in 0..2_048 {
                let sample = voice.sample();
                assert!(sample.is_finite() && sample.abs() <= 1.0);
            }
        });
    }
}

#[test]
fn topology_push_fades_through_zero_while_the_note_keeps_playing() {
    let mut voice = DualFilterConceptVoice::new(
        48_000.0,
        ConceptVariant::SweetSerial,
        ConceptControls::MIDPOINT,
    )
    .unwrap();
    voice.note_on(60, 0.9);
    for _ in 0..4_800 {
        voice.sample();
    }
    voice.toggle_topology();
    let transition: Vec<_> = (0..600).map(|_| voice.sample()).collect();
    assert!(
        transition[..260]
            .iter()
            .any(|sample| sample.abs() <= 1.0e-6)
    );
    assert!(transition[500..].iter().any(|sample| sample.abs() > 1.0e-4));
    assert!(
        transition
            .iter()
            .all(|sample| sample.is_finite() && sample.abs() <= 1.0)
    );
}

#[test]
fn every_physical_control_and_the_push_action_change_the_render() {
    let baseline = render_concept(
        ConceptVariant::CounterMotion,
        ConceptControls::MIDPOINT,
        12_000,
        52,
        0.45,
        1.2,
    )
    .unwrap();

    for control in ConceptControl::ALL {
        let mut controls = ConceptControls::MIDPOINT;
        controls.set(control, if control.index() % 2 == 0 { 0.1 } else { 0.9 });
        let changed = render_concept(
            ConceptVariant::CounterMotion,
            controls,
            12_000,
            52,
            0.45,
            1.2,
        )
        .unwrap();
        let residual = rms_residual(&baseline, &changed);
        assert!(
            residual > 1.0e-5,
            "control={} residual={residual}",
            control.label()
        );
    }

    let mut pushed = DualFilterConceptVoice::new(
        12_000.0,
        ConceptVariant::CounterMotion,
        ConceptControls::MIDPOINT,
    )
    .unwrap();
    pushed.toggle_topology();
    pushed.note_on(52, 0.9);
    let changed: Vec<_> = (0..baseline.len())
        .map(|frame| {
            if frame == (0.45 * 12_000.0) as usize {
                pushed.note_off();
            }
            pushed.sample()
        })
        .collect();
    assert!(rms_residual(&baseline, &changed) > 1.0e-4);
}

#[test]
fn amp_release_returns_to_exact_silence() {
    let mut controls = ConceptControls::MIDPOINT;
    controls.set(ConceptControl::AmpRelease, 0.0);
    let rendered = render_concept(
        ConceptVariant::ParallelSplit,
        controls,
        48_000,
        48,
        0.08,
        0.5,
    )
    .unwrap();
    assert!(
        rendered
            .last_chunk::<256>()
            .unwrap()
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn high_rate_comparison_is_explicit_deterministic_and_finite() {
    for variant in ConceptVariant::ALL {
        for note in [36, 60, 84] {
            let first = measure_high_rate_residual(variant, ConceptControls::MIDPOINT, note, 4_096)
                .unwrap();
            let second =
                measure_high_rate_residual(variant, ConceptControls::MIDPOINT, note, 4_096)
                    .unwrap();
            assert_eq!(first, second);
            assert!(
                first.is_finite() && first <= 0.0,
                "variant={variant:?} note={note} residual={first}"
            );
        }
    }
}

fn rms_residual(left: &[f32], right: &[f32]) -> f64 {
    assert_eq!(left.len(), right.len());
    (left
        .iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = f64::from(*left) - f64::from(*right);
            delta * delta
        })
        .sum::<f64>()
        / left.len() as f64)
        .sqrt()
}
