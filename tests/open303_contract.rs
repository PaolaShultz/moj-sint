#![cfg(feature = "open303")]
use assert_no_alloc::assert_no_alloc;
use moj_sint::open303::{Controls, FilterMode, Open303};

#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

#[test]
fn rejects_invalid_configuration_without_changing_the_voice() {
    for rate in [0.0, 8000.0, f64::NAN, f64::INFINITY] {
        assert!(Open303::new(rate, FilterMode::Tb303, Controls::default()).is_err());
    }
    let mut voice = Open303::new(48000.0, FilterMode::Tb303, Controls::default()).unwrap();
    let before = voice.controls();
    for controls in [
        Controls {
            cutoff_hz: f64::NAN,
            ..before
        },
        Controls {
            resonance: 101.0,
            ..before
        },
        Controls {
            volume_db: 1.0,
            ..before
        },
    ] {
        assert!(voice.set_controls(controls).is_err());
        assert_eq!(voice.controls(), before);
    }
    assert!(voice.note_on(128, 80).is_err());
    assert!(voice.note_on(36, 128).is_err());
    assert!(voice.is_idle());
}

#[test]
fn note_render_control_and_reset_paths_do_not_allocate_rust_storage() {
    let mut voice = Open303::new(48000.0, FilterMode::Tb303, Controls::default()).unwrap();
    let mut block = [0.0; 512];
    assert_no_alloc(|| {
        voice.note_off(36); // stale release before first note
        voice.render(&mut block);
        assert!(block.iter().all(|x| *x == 0.0));
        for key in 0..128 {
            voice.note_on(key, 110).unwrap();
            voice.render(&mut block);
            assert!(block.iter().all(|x| x.is_finite() && x.abs() <= 0.999));
        }
        for i in 0..128 {
            voice
                .set_controls(Controls {
                    waveform: f64::from(i) / 127.0,
                    ..Controls::default()
                })
                .unwrap();
            voice.note_off(i);
            voice.render(&mut block);
        }
        voice.reset();
        voice.render(&mut block);
        assert!(voice.is_idle());
        assert!(block.iter().all(|x| *x == 0.0));
    });
}

fn phrase(mode: FilterMode, chunks: usize) -> Vec<f32> {
    let mut voice = Open303::new(48000.0, mode, Controls::default()).unwrap();
    let mut result = vec![0.0; 24000];
    for (i, (key, velocity)) in [(36, 110), (43, 70), (43, 0), (36, 0)]
        .into_iter()
        .enumerate()
    {
        voice.note_on(key, velocity).unwrap();
        for block in result[i * 6000..(i + 1) * 6000].chunks_mut(chunks) {
            voice.render(block);
        }
    }
    assert!(!voice.status().faulted);
    result
}

#[test]
fn deterministic_chunk_independent_distinct_filter_modes() {
    let a = phrase(FilterMode::Tb303, 6000);
    assert_eq!(a, phrase(FilterMode::Tb303, 17));
    let b = phrase(FilterMode::Lowpass18, 6000);
    assert_ne!(a, b);
    assert!(a.iter().any(|x| x.abs() > 0.001));
}

#[test]
fn independent_concurrent_preparation_and_recovery() {
    let reference = phrase(FilterMode::Tb303, 64);
    let handles: Vec<_> = (0..4)
        .map(|_| std::thread::spawn(|| phrase(FilterMode::Tb303, 64)))
        .collect();
    for handle in handles {
        assert_eq!(reference, handle.join().unwrap());
    }
    let mut voice = Open303::new(48000.0, FilterMode::Tb303, Controls::default()).unwrap();
    let mut a = [0.0; 2048];
    let mut b = [0.0; 2048];
    voice.note_on(36, 110).unwrap();
    voice.render(&mut a);
    voice.reset();
    voice.note_on(36, 110).unwrap();
    voice.render(&mut b);
    assert_eq!(a, b);
}

#[test]
fn native_allocation_and_source_regressions() {
    let exe = env!("OPEN303_NATIVE_TEST");
    for arg in ["bounds", "contracts"] {
        let output = std::process::Command::new(exe).arg(arg).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
#[ignore = "development-only full oscillator spectral evidence; focused bounds/state tests run normally"]
fn oscillator_spectral_evidence() {
    let output = std::process::Command::new(env!("OPEN303_NATIVE_TEST"))
        .arg("spectrum")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn release_reaches_silence_and_ceiling_is_reported() {
    let mut voice = Open303::new(
        96000.0,
        FilterMode::Lowpass18,
        Controls {
            volume_db: 0.0,
            cutoff_hz: 2394.0,
            resonance: 100.0,
            env_mod: 100.0,
            accent: 100.0,
            waveform: 0.0,
            ..Controls::default()
        },
    )
    .unwrap();
    let mut block = [0.0; 4096];
    voice.note_on(127, 110).unwrap();
    for _ in 0..4 {
        voice.render(&mut block);
    }
    assert!(voice.status().clipped_samples > 0);
    voice.release_all();
    for _ in 0..100 {
        voice.render(&mut block);
    }
    assert!(voice.is_idle());
    assert!(block.iter().all(|x| *x == 0.0));
    assert!(!voice.status().faulted);
    voice.reset();
    assert_eq!(voice.status().clipped_samples, 0);
}
