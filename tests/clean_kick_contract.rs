use assert_no_alloc::assert_no_alloc;
use moj_sint::clean_kick::{KickTopology, PreparedKick, SAMPLE_RATE};

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
