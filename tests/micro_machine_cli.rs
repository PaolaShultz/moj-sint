use std::process::Command;

#[test]
fn micro_machine_lab_writes_the_named_dry_listening_set() {
    let output = tempfile::tempdir().unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_micro-machine-lab"))
        .args([
            "render",
            "experiments/swarm-micro-machine-v1.toml",
            output.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let mut names = std::fs::read_dir(output.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    names.sort();
    let expected_wavs = [
        "01_neutral-reference.wav",
        "02_forceful-lead.wav",
        "03_warm-pad.wav",
        "10_MASS_low-high.wav",
        "11_DETUNE_low-high.wav",
        "12_SPREAD_low-high.wav",
        "13_SHAPE_low-high.wav",
        "14_BITE_low-high.wav",
        "15_MOTION_low-high.wav",
        "16_COLOR_low-high.wav",
        "17_SPACE_low-high.wav",
    ];
    for expected in expected_wavs {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
        let mut reader = hound::WavReader::open(output.path().join(expected)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().bits_per_sample, 32);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        let samples = reader
            .samples::<f32>()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().any(|sample| sample.abs() > 1.0e-4));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    for report in [
        "README.md",
        "graph-summary.tsv",
        "measurements.tsv",
        "alias-error.tsv",
        "hashes.tsv",
    ] {
        assert!(names.iter().any(|name| name == report), "missing {report}");
    }
    let readme = std::fs::read_to_string(output.path().join("README.md")).unwrap();
    assert!(readme.contains("MASS, DETUNE, SPREAD, SHAPE, BITE, MOTION, COLOR, SPACE"));
    assert!(readme.contains("No reverb, delay, chorus, compressor, limiter, or normalization"));
    assert!(readme.contains("not a fourth live model"));
    let summary = std::fs::read_to_string(output.path().join("graph-summary.tsv")).unwrap();
    assert!(summary.contains("nodes\tedges\toscillator_slots\tstate_bytes\twork_units_per_sample"));
    let alias = std::fs::read_to_string(output.path().join("alias-error.tsv")).unwrap();
    assert_eq!(alias.lines().count(), 4);
}
