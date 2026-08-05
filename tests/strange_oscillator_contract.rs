use moj_sint::strange::{StrangeControls, StrangeType, StrangeVoice};

const SAMPLE_RATE: f32 = 48_000.0;
const FREQUENCY: f32 = 220.0;

fn render(kind: StrangeType, controls: StrangeControls) -> Vec<[f32; 2]> {
    let mut voice = StrangeVoice::new(kind, SAMPLE_RATE, FREQUENCY, 0x51a7_2026, controls)
        .expect("valid strange voice");
    (0..16_384)
        .map(|_| {
            let frame = voice.sample();
            [frame.left, frame.right]
        })
        .collect()
}

#[test]
fn type_macro_has_eight_stable_detents() {
    assert_eq!(StrangeType::ALL.len(), 8);
    for (index, kind) in StrangeType::ALL.into_iter().enumerate() {
        let center = index as f32 / 7.0;
        assert_eq!(StrangeType::from_normalized(center), kind);
    }
    assert_eq!(StrangeType::from_normalized(-1.0), StrangeType::Triangle);
    assert_eq!(
        StrangeType::from_normalized(2.0),
        StrangeType::RegisterMachine
    );
}

#[test]
fn first_four_motion_rate_spans_slow_motion_to_fifty_hertz() {
    let low = StrangeControls::MIDPOINT.with(3, 0.0).lfo_rate_hz();
    let middle = StrangeControls::MIDPOINT.lfo_rate_hz();
    let high = StrangeControls::MIDPOINT.with(3, 1.0).lfo_rate_hz();
    assert!((low - 0.05).abs() < 1.0e-6, "low={low}");
    assert!((middle - 50.0_f32.sqrt() * 0.05_f32.sqrt()).abs() < 1.0e-5);
    assert!((high - 50.0).abs() < 1.0e-4, "high={high}");
}

#[test]
fn every_type_is_deterministic_finite_bounded_non_silent_and_distinct() {
    let mut hashes = Vec::new();
    for kind in StrangeType::ALL {
        let first = render(kind, StrangeControls::MIDPOINT);
        let second = render(kind, StrangeControls::MIDPOINT);
        assert_eq!(first, second, "type={kind:?}");
        assert!(
            first
                .iter()
                .flatten()
                .all(|sample| sample.is_finite() && sample.abs() <= 1.0),
            "type={kind:?}"
        );
        let energy = first
            .iter()
            .flatten()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / (2 * first.len()) as f64;
        assert!(energy.sqrt() > 0.005, "type={kind:?} rms={}", energy.sqrt());
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for sample in first.iter().flatten() {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hashes.push(hash);
    }
    hashes.sort_unstable();
    hashes.dedup();
    assert_eq!(hashes.len(), StrangeType::ALL.len());
}

#[test]
fn prepared_sample_paths_allocate_nothing() {
    for kind in StrangeType::ALL {
        let mut voice = StrangeVoice::new(
            kind,
            SAMPLE_RATE,
            FREQUENCY,
            0x51a7_2026,
            StrangeControls::MIDPOINT,
        )
        .unwrap();
        assert_no_alloc::assert_no_alloc(|| {
            let mut sum = 0.0_f32;
            for _ in 0..4_096 {
                let frame = voice.sample();
                sum += frame.left + frame.right;
            }
            std::hint::black_box(sum);
        });
    }
}

#[test]
fn every_shared_macro_changes_most_types_materially() {
    for slot in 0..7 {
        let mut changed = 0;
        for kind in StrangeType::ALL {
            let low = render(kind, StrangeControls::MIDPOINT.with(slot, 0.1));
            let high = render(kind, StrangeControls::MIDPOINT.with(slot, 0.9));
            let residual = low
                .iter()
                .zip(&high)
                .flat_map(|(left, right)| {
                    [f64::from(left[0] - right[0]), f64::from(left[1] - right[1])]
                })
                .map(|difference| difference * difference)
                .sum::<f64>()
                / (2 * low.len()) as f64;
            if residual.sqrt() > 0.02 {
                changed += 1;
            }
        }
        assert!(changed >= 6, "macro_slot={slot} changed_types={changed}");
    }
}

#[test]
fn invalid_construction_is_rejected() {
    for rate in [0.0, -1.0, f32::NAN] {
        assert!(
            StrangeVoice::new(
                StrangeType::DeformedLoop,
                rate,
                FREQUENCY,
                1,
                StrangeControls::MIDPOINT,
            )
            .is_err()
        );
    }
    for frequency in [0.0, -1.0, 24_000.0, f32::INFINITY] {
        assert!(
            StrangeVoice::new(
                StrangeType::DeformedLoop,
                SAMPLE_RATE,
                frequency,
                1,
                StrangeControls::MIDPOINT,
            )
            .is_err()
        );
    }
}
