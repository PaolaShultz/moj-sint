use std::process::Command;

#[test]
fn strange_lab_writes_one_type_dial_and_seven_clear_macro_reels() {
    let output = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_strange-oscillator-lab"))
        .arg("render")
        .arg(output.path())
        .status()
        .unwrap();
    assert!(status.success());

    let mut names = std::fs::read_dir(output.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    let wav_count = names.iter().filter(|name| name.ends_with(".wav")).count();
    assert_eq!(wav_count, 16, "{names:#?}");
    for required in [
        "09_TYPE_all-8-positions_live.wav",
        "10_FORM_saw_low-high.wav",
        "11_WARP_saw_low-high.wav",
        "12_COUPLE_saw_low-high.wav",
        "13_MOTION_saw_low-high.wav",
        "14_CHAOS_saw_low-high.wav",
        "15_COLOR_saw_low-high.wav",
        "16_SPACE_saw_low-high.wav",
    ] {
        assert!(
            names.iter().any(|name| name == required),
            "missing {required}"
        );
    }
    for required in [
        "README.md",
        "gates.tsv",
        "structural-macros.tsv",
        "hashes.tsv",
    ] {
        assert!(
            names.iter().any(|name| name == required),
            "missing {required}"
        );
    }

    let gates = std::fs::read_to_string(output.path().join("gates.tsv")).unwrap();
    assert!(gates.contains("type\tstatus\treason"));
    for slug in [
        "triangle",
        "saw",
        "pulse",
        "modulated-resonator",
        "deformed-loop",
        "stochastic-breakpoints",
        "scanned-string",
        "register-machine",
    ] {
        assert!(gates.contains(slug), "missing gate row for {slug}");
    }

    for name in names.iter().filter(|name| name.ends_with(".wav")) {
        let reader = hound::WavReader::open(output.path().join(name)).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.bits_per_sample, 32);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);
    }
    for name in names.iter().filter(|name| name.ends_with(".wav")) {
        let reader = hound::WavReader::open(output.path().join(name)).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.bits_per_sample, 32);
        assert_eq!(spec.sample_format, hound::SampleFormat::Float);
        if name == "09_TYPE_all-8-positions_live.wav" {
            assert!(reader.duration() >= 9 * 48_000, "name={name}");
        }
        if name == "13_MOTION_saw_low-high.wav" {
            assert!(reader.duration() >= 13 * 48_000, "name={name}");
        }
        if name == "14_CHAOS_saw_low-high.wav" {
            assert!(reader.duration() >= 8 * 48_000, "name={name}");
        }
    }

    let readme = std::fs::read_to_string(output.path().join("README.md")).unwrap();
    assert!(readme.contains("one uninterrupted held note"));
    assert!(readme.contains("low, silence, high"));
    assert!(readme.contains("Saw is the proof topology"));
}
