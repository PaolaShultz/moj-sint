use assert_no_alloc::assert_no_alloc;
use moj_sint::bass_matrix::{BassMatrixControls, BassMatrixVoice};

const FRAMES: usize = 24_000;

fn render(controls: BassMatrixControls) -> Vec<[f32; 2]> {
    let mut voice = BassMatrixVoice::new(48_000.0, 0x51a7_2026).unwrap();
    voice.set_controls(controls);
    voice.note_on(36, 0.9);
    (0..FRAMES).map(|_| voice.sample()).collect()
}

fn residual(left: &[[f32; 2]], right: &[[f32; 2]]) -> f64 {
    (left
        .iter()
        .zip(right)
        .flat_map(|(a, b)| [f64::from(a[0] - b[0]), f64::from(a[1] - b[1])])
        .map(|value| value * value)
        .sum::<f64>()
        / (2 * left.len()) as f64)
        .sqrt()
}

fn measurements(samples: &[[f32; 2]]) -> (f64, f32, f64, u64) {
    let mut energy = 0.0;
    let mut difference_energy = 0.0;
    let mut peak = 0.0_f32;
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut previous = 0.0_f32;
    for frame in samples {
        let mono = 0.5 * (frame[0] + frame[1]);
        energy += f64::from(mono).powi(2);
        difference_energy += f64::from(mono - previous).powi(2);
        previous = mono;
        peak = peak.max(frame[0].abs()).max(frame[1].abs());
        for sample in frame {
            hash ^= u64::from(sample.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    (
        (energy / samples.len() as f64).sqrt(),
        peak,
        (difference_energy / samples.len() as f64).sqrt(),
        hash,
    )
}

#[test]
fn seven_timbre_controls_have_distinct_minimum_middle_and_maximum_outputs() {
    for index in 0..7 {
        let with = |value| {
            let mut c = BassMatrixControls::START;
            match index {
                0 => c.body = value,
                1 => c.growl = value,
                2 => c.metal = value,
                3 => c.punch = value,
                4 => c.drive = value,
                5 => c.filter = value,
                6 => c.unstable = value,
                _ => unreachable!(),
            }
            render(c)
        };
        let low = with(0.0);
        let middle = with(0.5);
        let high = with(1.0);
        assert!(residual(&low, &middle) > 0.002, "control {index} low/mid");
        assert!(residual(&middle, &high) > 0.002, "control {index} mid/high");
    }
}

#[test]
fn rendering_is_deterministic_finite_bounded_stereo_and_allocation_free() {
    let first = render(BassMatrixControls {
        body: 1.0,
        growl: 1.0,
        metal: 1.0,
        punch: 1.0,
        drive: 1.0,
        filter: 1.0,
        unstable: 1.0,
    });
    let second = render(BassMatrixControls {
        body: 1.0,
        growl: 1.0,
        metal: 1.0,
        punch: 1.0,
        drive: 1.0,
        filter: 1.0,
        unstable: 1.0,
    });
    assert_eq!(first, second);
    assert!(
        first
            .iter()
            .flatten()
            .all(|sample| sample.is_finite() && sample.abs() <= 0.94)
    );
    assert!(first.iter().any(|frame| frame[0] != frame[1]));

    let mut voice = BassMatrixVoice::new(48_000.0, 7).unwrap();
    voice.note_on(24, 1.0);
    assert_no_alloc(|| {
        for step in 0..4_096 {
            let value = if step & 1 == 0 { 0.0 } else { 1.0 };
            voice.set_controls(BassMatrixControls {
                body: value,
                growl: 1.0 - value,
                metal: value,
                punch: 1.0 - value,
                drive: value,
                filter: 1.0 - value,
                unstable: value,
            });
            std::hint::black_box(voice.sample());
        }
    });
    voice.reset();
    assert_eq!(voice.sample(), [0.0; 2]);
}

#[test]
fn five_named_transformations_have_deterministic_distinct_measurements() {
    let profiles = [
        (
            "clean",
            BassMatrixControls {
                body: 0.45,
                growl: 0.0,
                metal: 0.0,
                punch: 0.15,
                drive: 0.0,
                filter: 0.35,
                unstable: 0.0,
            },
        ),
        (
            "sub-heavy",
            BassMatrixControls {
                body: 1.0,
                growl: 0.05,
                metal: 0.0,
                punch: 0.25,
                drive: 0.05,
                filter: 0.18,
                unstable: 0.0,
            },
        ),
        (
            "driven",
            BassMatrixControls {
                body: 0.7,
                growl: 0.34,
                metal: 0.08,
                punch: 0.42,
                drive: 0.95,
                filter: 0.72,
                unstable: 0.1,
            },
        ),
        (
            "growling",
            BassMatrixControls {
                body: 0.72,
                growl: 1.0,
                metal: 0.18,
                punch: 0.55,
                drive: 0.64,
                filter: 0.74,
                unstable: 0.28,
            },
        ),
        (
            "extreme",
            BassMatrixControls {
                body: 1.0,
                growl: 1.0,
                metal: 1.0,
                punch: 1.0,
                drive: 1.0,
                filter: 1.0,
                unstable: 1.0,
            },
        ),
    ];
    let rendered = profiles.map(|(name, controls)| (name, render(controls)));
    let mut hashes = std::collections::BTreeSet::new();
    for (name, samples) in &rendered {
        let (rms, peak, difference_rms, hash) = measurements(samples);
        println!(
            "{name}\trms={rms:.8}\tpeak={peak:.8}\tdifference_rms={difference_rms:.8}\thash={hash:016x}"
        );
        assert!(rms > 0.005);
        assert!(peak <= 0.94);
        assert!(difference_rms.is_finite());
        assert!(hashes.insert(hash));
    }
    for pair in rendered.windows(2) {
        assert!(
            residual(&pair[0].1, &pair[1].1) > 0.01,
            "{} and {} were not materially different",
            pair[0].0,
            pair[1].0
        );
    }
}
