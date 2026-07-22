use std::process::Command;

#[test]
fn validate_accepts_reference_preset() {
    let output = Command::new(env!("CARGO_BIN_EXE_moj-sint"))
        .args(["validate", "presets/reference.mojsint"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid"));
}

#[test]
fn render_writes_a_valid_wav() {
    let directory = tempfile::tempdir().unwrap();
    let output_path = directory.path().join("note.wav");
    let output = Command::new(env!("CARGO_BIN_EXE_moj-sint"))
        .args(["render", "presets/reference.mojsint"])
        .arg(&output_path)
        .args(["--note", "60", "--seconds", "0.05"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(hound::WavReader::open(output_path).is_ok());
}

#[test]
fn invalid_arguments_fail_clearly() {
    let output = Command::new(env!("CARGO_BIN_EXE_moj-sint"))
        .args([
            "render",
            "presets/reference.mojsint",
            "/tmp/unused.wav",
            "--note",
            "300",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("MIDI note"));
}

#[test]
fn oscillator_lab_writes_a_deterministic_listening_matrix() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first");
    let second = directory.path().join("second");
    for output_directory in [&first, &second] {
        let output = Command::new(env!("CARGO_BIN_EXE_oscillator-lab"))
            .args(["render"])
            .arg(output_directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let wav_paths = |directory: &std::path::Path| {
        let mut paths: Vec<_> = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wav"))
            .collect();
        paths.sort();
        paths
    };
    let first_wavs = wav_paths(&first);
    let second_wavs = wav_paths(&second);
    assert_eq!(first_wavs.len(), 27);
    assert_eq!(second_wavs.len(), 27);
    for (first_path, second_path) in first_wavs.iter().zip(&second_wavs) {
        assert_eq!(first_path.file_name(), second_path.file_name());
        assert_eq!(
            std::fs::read(first_path).unwrap(),
            std::fs::read(second_path).unwrap()
        );
        let mut reader = hound::WavReader::open(first_path).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().bits_per_sample, 32);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        let samples: Vec<f32> = reader.samples().map(Result::unwrap).collect();
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().any(|sample| sample.abs() > 0.001));
    }
    assert_eq!(
        std::fs::read(first.join("listening-manifest.tsv")).unwrap(),
        std::fs::read(second.join("listening-manifest.tsv")).unwrap()
    );
}

#[test]
fn oscillator_lab_writes_harmonic_selector_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first");
    let second = directory.path().join("second");
    for output_directory in [&first, &second] {
        let output = Command::new(env!("CARGO_BIN_EXE_oscillator-lab"))
            .args(["system"])
            .arg(output_directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let evidence_paths = |directory: &std::path::Path| {
        let mut paths: Vec<_> = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension().is_some_and(|extension| extension == "wav")
                    || path.file_name().is_some_and(|name| {
                        name == "system-manifest.tsv" || name == "alias-matrix.tsv"
                    })
            })
            .collect();
        paths.sort();
        paths
    };
    let first_paths = evidence_paths(&first);
    let second_paths = evidence_paths(&second);
    assert_eq!(first_paths.len(), 29);
    assert_eq!(second_paths.len(), 29);
    for (first_path, second_path) in first_paths.iter().zip(&second_paths) {
        assert_eq!(first_path.file_name(), second_path.file_name());
        assert_eq!(
            std::fs::read(first_path).unwrap(),
            std::fs::read(second_path).unwrap()
        );
    }
    let manifest = std::fs::read_to_string(first.join("system-manifest.tsv")).unwrap();
    assert!(manifest.starts_with(
        "file\tnote\tfrequency_hz\tedge\tcouple\tpeak\trms\tdc\tfundamental_db\tthird_db\tnonharmonic_error_db\tpitch_retained\tfinite\tsample_hash\n"
    ));
    assert_eq!(
        manifest
            .lines()
            .skip(1)
            .filter(|line| line.contains("\ttrue\ttrue\t"))
            .count(),
        24
    );
    assert!(manifest.contains("note060_edge000_couple000.wav"));
    assert!(manifest.contains("note060_edge100_couple100.wav"));
    let alias_matrix = std::fs::read_to_string(first.join("alias-matrix.tsv")).unwrap();
    assert!(alias_matrix.starts_with(
        "note\tfrequency_hz\ttarget_third_hz\tthird_weight\tnonharmonic_error_db\tpitch_retained\tfinite\n"
    ));
    assert!(
        alias_matrix
            .lines()
            .any(|line| line.starts_with("120\t") && line.contains("\t0.000\t"))
    );
}
