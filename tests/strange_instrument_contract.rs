use moj_sint::strange::{StrangeControls, StrangeInstrument, StrangeType};
use moj_sint::strange_lab::{StructuralMacroMetric, evaluate_structural_macros};

const SAMPLE_RATE: f32 = 48_000.0;
const FREQUENCY: f32 = 220.0;

#[test]
fn one_type_parameter_reaches_all_eight_topologies_with_click_safe_transitions() {
    let mut instrument = StrangeInstrument::new(
        SAMPLE_RATE,
        FREQUENCY,
        0x51a7_2026,
        0.0,
        StrangeControls::MIDPOINT,
    )
    .expect("valid unified voice");

    for (index, expected) in StrangeType::ALL.into_iter().enumerate() {
        let normalized = index as f32 / 7.0;
        assert_eq!(
            instrument.set_type_normalized(normalized).unwrap(),
            expected
        );
        let mut maximum_jump = 0.0_f32;
        let mut previous = instrument.sample();
        for _ in 0..2_048 {
            let next = instrument.sample();
            maximum_jump = maximum_jump
                .max((next.left - previous.left).abs())
                .max((next.right - previous.right).abs());
            previous = next;
        }
        assert_eq!(instrument.current_type(), expected);
        assert!(!instrument.is_transitioning());
        assert!(maximum_jump < 0.65, "type={expected:?} jump={maximum_jump}");
    }
}

#[test]
fn type_switch_and_sample_paths_allocate_nothing() {
    let mut instrument = StrangeInstrument::new(
        SAMPLE_RATE,
        FREQUENCY,
        0x51a7_2026,
        0.0,
        StrangeControls::MIDPOINT,
    )
    .unwrap();
    assert_no_alloc::assert_no_alloc(|| {
        for kind in StrangeType::ALL {
            instrument.set_type(kind).unwrap();
            for _ in 0..1_024 {
                std::hint::black_box(instrument.sample());
            }
        }
    });
}

#[test]
fn every_type_has_seven_domain_specific_structural_macro_landmarks() {
    for kind in StrangeType::ALL {
        let evidence = evaluate_structural_macros(kind).expect("structural gate should run");
        assert_eq!(evidence.len(), 7, "type={kind:?}");
        for (slot, row) in evidence.into_iter().enumerate() {
            assert_eq!(row.slot, slot);
            assert!(row.value.is_finite(), "type={kind:?} slot={slot}");
            assert!(
                row.value >= row.floor,
                "type={kind:?} slot={slot} metric={:?} value={} floor={}",
                row.metric,
                row.value,
                row.floor
            );
        }
    }
}

#[test]
fn macro_gate_uses_the_intended_perceptual_domain_not_raw_sample_residual() {
    let rows = evaluate_structural_macros(StrangeType::Saw).unwrap();
    assert_eq!(rows[0].metric, StructuralMacroMetric::SpectralShapeDb);
    assert_eq!(rows[1].metric, StructuralMacroMetric::SpectralShapeDb);
    assert_eq!(
        rows[2].metric,
        StructuralMacroMetric::CouplingTopologyDistance
    );
    assert_eq!(rows[3].metric, StructuralMacroMetric::MotionRateRatio);
    assert_eq!(rows[4].metric, StructuralMacroMetric::CycleIrregularity);
    assert_eq!(rows[5].metric, StructuralMacroMetric::BrightnessRatio);
    assert_eq!(rows[6].metric, StructuralMacroMetric::StereoWidthDb);
}
