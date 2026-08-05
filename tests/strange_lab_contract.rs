use moj_sint::strange::{StrangeControls, StrangeType};
use moj_sint::strange_lab::{
    evaluate_type, measure_cyclic_envelope_depth_db, measure_high_rate_residual, render_and_measure,
};

#[test]
fn every_type_has_bounded_low_mid_high_evidence() {
    for kind in StrangeType::ALL {
        for note in [36, 60, 84] {
            let render = render_and_measure(kind, StrangeControls::MIDPOINT, note, 16_384)
                .expect("valid render");
            assert!(render.metrics.finite, "type={kind:?} note={note}");
            assert!(render.metrics.peak <= 1.0, "type={kind:?} note={note}");
            assert!(render.metrics.rms > 0.001, "type={kind:?} note={note}");
            assert!(render.metrics.dc.abs() < 0.02, "type={kind:?} note={note}");
            assert!(
                render.metrics.maximum_jump < 1.25,
                "type={kind:?} note={note}"
            );
            assert!(render.metrics.mono_rms > 0.001, "type={kind:?} note={note}");
        }
    }
}

#[test]
fn high_rate_policy_is_explicit_and_deterministic() {
    for kind in StrangeType::ALL {
        let first = measure_high_rate_residual(kind, StrangeControls::MIDPOINT, 60, 8_192)
            .expect("valid residual request");
        let second = measure_high_rate_residual(kind, StrangeControls::MIDPOINT, 60, 8_192)
            .expect("valid residual request");
        assert_eq!(first, second, "type={kind:?}");
        let residual = first.expect("every revised type needs a tonal residual");
        assert!(
            residual.is_finite() && residual <= 0.0,
            "type={kind:?} residual={residual}"
        );
    }
}

#[test]
fn modulated_resonator_has_a_measured_cyclic_envelope() {
    for kind in StrangeType::ALL {
        let depth = measure_cyclic_envelope_depth_db(kind, StrangeControls::MIDPOINT, 60, 96_000)
            .expect("valid envelope-depth request");
        if kind == StrangeType::ModulatedResonator {
            assert!(
                depth.is_some_and(|db| db.is_finite() && (3.0..=24.0).contains(&db)),
                "type={kind:?} depth={depth:?}"
            );
        } else {
            assert!(depth.is_none(), "type={kind:?}");
        }
    }
}

#[test]
fn gate_evaluates_without_hiding_rejections() {
    for kind in StrangeType::ALL {
        let evidence = evaluate_type(kind).expect("gate should run");
        assert_eq!(evidence.kind, kind);
        assert_eq!(evidence.notes.len(), 3);
        assert_eq!(evidence.control_rows.len(), 7);
        assert!(!evidence.status.is_empty());
        assert!(!evidence.reason.is_empty());
    }
}
