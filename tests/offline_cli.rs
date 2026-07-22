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

#[test]
fn oscillator_lab_writes_small_deterministic_character_gate() {
    let directory = tempfile::tempdir().unwrap();
    let first = directory.path().join("first");
    let second = directory.path().join("second");
    for output_directory in [&first, &second] {
        let output = Command::new(env!("CARGO_BIN_EXE_oscillator-lab"))
            .args(["character"])
            .arg(output_directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let wav_count = std::fs::read_dir(output_directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wav"))
            .count();
        assert_eq!(wav_count, 9);
    }

    for filename in [
        "note036_dry.wav",
        "note036_moderate.wav",
        "note036_strong.wav",
        "note060_dry.wav",
        "note060_moderate.wav",
        "note060_strong.wav",
        "note084_dry.wav",
        "note084_moderate.wav",
        "note084_strong.wav",
        "character-manifest.tsv",
        "harmonic-distribution.tsv",
        "alias-comparison.tsv",
        "intermodulation.tsv",
    ] {
        assert_eq!(
            std::fs::read(first.join(filename)).unwrap(),
            std::fs::read(second.join(filename)).unwrap(),
            "{filename}"
        );
    }

    let manifest = std::fs::read_to_string(first.join("character-manifest.tsv")).unwrap();
    assert!(manifest.starts_with(
        "file\tnote\tcondition\tedge\tcouple\tpeak\trms\tdc\tfundamental_db\tthird_db\tpitch_retained\tfinite\tsample_hash\n"
    ));
    assert_eq!(manifest.lines().count(), 10);
    for line in manifest.lines().skip(1) {
        let columns: Vec<_> = line.split('\t').collect();
        let rms: f64 = columns[6].parse().unwrap();
        assert!((rms - 0.08).abs() < 0.000_25, "{line}");
        assert_eq!(columns[10], "true");
        assert_eq!(columns[11], "true");
    }

    let alias = std::fs::read_to_string(first.join("alias-comparison.tsv")).unwrap();
    assert!(alias.starts_with(
        "method\tnote\tcondition\talias_error_db\tpeak\trms\tdc\tfundamental_db\tthird_db\tpitch_retained\tfinite\tselected\n"
    ));
    assert_eq!(alias.lines().count(), 25);
    assert_eq!(
        alias
            .lines()
            .filter(|line| line.ends_with("\ttrue"))
            .count(),
        8
    );

    let harmonics = std::fs::read_to_string(first.join("harmonic-distribution.tsv")).unwrap();
    assert!(harmonics.starts_with(
        "file\tnote\tcondition\th01_db\th02_db\th03_db\th04_db\th05_db\th06_db\th07_db\th08_db\th09_db\th10_db\th11_db\th12_db\n"
    ));
    assert_eq!(harmonics.lines().count(), 10);

    let intermodulation = std::fs::read_to_string(first.join("intermodulation.tsv")).unwrap();
    assert!(intermodulation.starts_with("method\tper_voice_imd_db\tpost_mix_imd_db\tfinite\n"));
}
