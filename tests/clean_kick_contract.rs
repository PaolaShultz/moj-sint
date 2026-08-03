use assert_no_alloc::assert_no_alloc;
use moj_sint::clean_kick::{
    KickTopology, PreparedKick, SAMPLE_RATE, evaluate, render_solo, select,
};

#[test]
fn house_impact_is_deterministic_finite_and_allocation_free() {
    let mut first = PreparedKick::new(KickTopology::HouseImpact, SAMPLE_RATE).unwrap();
    let mut second = PreparedKick::new(KickTopology::HouseImpact, SAMPLE_RATE).unwrap();
    assert_no_alloc(|| {
        first.trigger();
        second.trigger();
        for _ in 0..SAMPLE_RATE {
            let a = first.sample();
            let b = second.sample();
            assert_eq!(a.to_bits(), b.to_bits());
            assert!(a.is_finite());
        }
    });
}

#[test]
fn long_pressure_is_distinct_stable_and_allocation_free() {
    let mut house = PreparedKick::new(KickTopology::HouseImpact, SAMPLE_RATE).unwrap();
    let mut pressure = PreparedKick::new(KickTopology::LongPressure, SAMPLE_RATE).unwrap();
    assert_no_alloc(|| {
        house.trigger();
        pressure.trigger();
        let mut difference = 0.0_f64;
        for _ in 0..SAMPLE_RATE {
            let a = house.sample();
            let b = pressure.sample();
            assert!(b.is_finite());
            difference += f64::from((a - b).abs());
        }
        assert!(difference > 100.0);
    });
}

#[test]
fn selected_voices_pass_every_engineering_gate() {
    for topology in [KickTopology::HouseImpact, KickTopology::LongPressure] {
        let config = select(topology, SAMPLE_RATE).unwrap();
        let render = render_solo(config, SAMPLE_RATE).unwrap();
        let evidence = evaluate(&render).unwrap();
        assert!(
            evidence.rejection_reasons().is_empty(),
            "{evidence:#?}: {:?}",
            evidence.rejection_reasons()
        );
        assert!((-6.0..=-1.0).contains(&evidence.metrics.sample_peak_dbfs));
        assert!(evidence.metrics.true_peak_dbfs <= -1.0);
        assert_eq!(evidence.metrics.ceiling_contacts, 0);
        assert!(evidence.metrics.absolute_dc < 1.0e-5);
        assert!(evidence.metrics.maximum_jump < 0.10);
        assert!(evidence.high_rate_residual_db <= -60.0);
    }
}
