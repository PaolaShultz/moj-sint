use assert_no_alloc::assert_no_alloc;
use moj_sint::dual_filter::{DualFilterCore, DualFilterInstrument};
use moj_sint::dual_filter_concept::{ConceptControl, ConceptControls};

fn controls() -> ConceptControls {
    ConceptControls::new([
        0.32, 0.62, 0.67, 0.46, 0.38, 0.56, 0.18, 0.14, 0.39, 0.34, 0.42, 0.08, 0.36, 0.68, 0.40,
    ])
    .unwrap()
}

fn render_structure(structure: f32) -> Vec<f32> {
    let mut values = controls();
    values.set(ConceptControl::Routing, structure);
    let mut voice = DualFilterInstrument::new(48_000.0, values, DualFilterCore::Industrial)
        .expect("valid production voice");
    voice.note_on(36, 0.9);
    (0..8_192).map(|_| voice.sample()).collect()
}

#[test]
fn exposes_the_approved_fifteen_control_order_and_structure_label() {
    assert_eq!(ConceptControl::ALL.len(), 15);
    assert_eq!(ConceptControl::Routing.index(), 6);
    assert_eq!(ConceptControl::Routing.label(), "STRUCTURE");
    assert_eq!(ConceptControl::FilterAttack.index(), 7);
    assert_eq!(ConceptControl::AmpRelease.index(), 14);
}

#[test]
fn industrial_structure_continuously_changes_the_live_graph() {
    let serial = render_structure(0.0);
    let middle = render_structure(0.5);
    let parallel = render_structure(1.0);
    let residual = |left: &[f32], right: &[f32]| {
        (left
            .iter()
            .zip(right)
            .skip(1_024)
            .map(|(a, b)| f64::from(a - b).powi(2))
            .sum::<f64>()
            / (left.len() - 1_024) as f64)
            .sqrt()
    };
    assert!(residual(&serial, &middle) > 1.0e-4);
    assert!(residual(&middle, &parallel) > 1.0e-4);
    assert!(residual(&serial, &parallel) > 1.0e-4);
}

#[test]
fn core_click_crossfades_a_held_note_without_retrigger_or_silence() {
    let mut voice = DualFilterInstrument::new(48_000.0, controls(), DualFilterCore::Industrial)
        .expect("valid production voice");
    voice.note_on(36, 0.9);
    for _ in 0..4_096 {
        let _ = voice.sample();
    }
    voice.toggle_core();
    let transition: Vec<f32> = (0..2_048).map(|_| voice.sample()).collect();
    assert_eq!(voice.core(), DualFilterCore::Counter);
    assert!(transition.iter().all(|sample| sample.is_finite()));
    assert!(transition.iter().all(|sample| sample.abs() < 2.0));
    assert!(transition.iter().any(|sample| sample.abs() > 1.0e-5));
    assert!(
        transition
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 1.25)
    );
}

#[test]
fn live_controls_core_switch_note_lifecycle_and_render_are_allocation_free() {
    let mut voice = DualFilterInstrument::new(48_000.0, controls(), DualFilterCore::Industrial)
        .expect("valid production voice");
    assert_no_alloc(|| {
        voice.note_on(36, 0.9);
        voice.set_controls(ConceptControls::MIDPOINT);
        voice.toggle_core();
        for _ in 0..2_048 {
            let sample = voice.sample();
            assert!(sample.is_finite());
        }
        voice.note_off();
        voice.reset();
    });
    assert!(voice.is_idle());
    assert_eq!(voice.sample(), 0.0);
}
