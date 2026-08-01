use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus, Output};

const WAV_NAMES: [&str; 7] = [
    "01_full_bass_phrase.wav",
    "02_full_lead_phrase.wav",
    "03_full_filter_articulation.wav",
    "04_matched_idealized_path.wav",
    "05_matched_linear_mixer.wav",
    "06_matched_linear_ladder.wav",
    "07_matched_no_drift_or_feedback.wav",
];

const REPORT_NAMES: [&str; 10] = [
    "README.md",
    "manifest.tsv",
    "metrics.tsv",
    "ablations.tsv",
    "oscillators.tsv",
    "filter.tsv",
    "alias.tsv",
    "hashes.tsv",
    "generation-summary.tsv",
    "workstation-cost.txt",
];

#[test]
#[ignore = "development-only historical Model D audition renderer"]
fn lab_writes_exact_deterministic_passing_model_d_audition() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let external_cost = external.path().join("misdirected-cost.txt");
    fs::write(first.path().join("stale.txt"), "remove me").unwrap();
    let first_status = run_lab_status(
        first.path(),
        &[(
            "MOJ_SINT_MODEL_D_LAB_TEST_WORKSTATION_COST_PATH",
            external_cost.as_os_str(),
        )],
    );
    assert!(first_status.success());
    run_lab(second.path());
    assert!(
        !external_cost.exists(),
        "legacy path injection escaped the staged batch"
    );

    let actual = file_names(first.path());
    let expected = WAV_NAMES
        .into_iter()
        .chain(REPORT_NAMES)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    assert_eq!(
        deterministic_files(first.path()),
        deterministic_files(second.path())
    );

    let mut frame_counts = BTreeMap::new();
    let mut decoded_hashes = BTreeMap::new();
    for name in WAV_NAMES {
        let mut wav = hound::WavReader::open(first.path().join(name)).unwrap();
        assert_eq!(wav.spec().channels, 2, "{name}");
        assert_eq!(wav.spec().sample_rate, 48_000, "{name}");
        assert_eq!(wav.spec().bits_per_sample, 32, "{name}");
        assert_eq!(
            wav.spec().sample_format,
            hound::SampleFormat::Float,
            "{name}"
        );
        let samples = wav.samples::<f32>().collect::<Result<Vec<_>, _>>().unwrap();
        frame_counts.insert(name, samples.len() / 2);
        decoded_hashes.insert(name, hash_sample_stream(&samples));
    }
    let bass_frames = frame_counts[WAV_NAMES[0]];
    for name in &WAV_NAMES[3..] {
        assert_eq!(frame_counts[*name], bass_frames, "{name}");
    }

    let manifest = fs::read_to_string(first.path().join("manifest.tsv")).unwrap();
    assert_eq!(manifest.matches("\tpass\n").count(), 7);
    for row in [
        "01_full_bass_phrase.wav\tbass-phrase\t264000\t1.000000\tpass",
        "02_full_lead_phrase.wav\tlead-phrase\t216000\t0.750000\tpass",
        "03_full_filter_articulation.wav\tfilter-articulation\t158400\t1.600000\tpass",
        "04_matched_idealized_path.wav\tbass-phrase\t264000\t1.000000\tpass",
        "05_matched_linear_mixer.wav\tbass-phrase\t264000\t1.000000\tpass",
        "06_matched_linear_ladder.wav\tbass-phrase\t264000\t1.000000\tpass",
        "07_matched_no_drift_or_feedback.wav\tbass-phrase\t264000\t1.000000\tpass",
    ] {
        assert!(manifest.contains(row), "missing manifest row: {row}");
    }

    for report in [
        "metrics.tsv",
        "ablations.tsv",
        "oscillators.tsv",
        "filter.tsv",
        "alias.tsv",
        "generation-summary.tsv",
    ] {
        let text = fs::read_to_string(first.path().join(report)).unwrap();
        assert!(text.contains("pass"), "{report} has no passing evidence");
        assert!(
            !text.contains("\tfail"),
            "{report} contains failed evidence"
        );
    }

    let alias = fs::read_to_string(first.path().join("alias.tsv")).unwrap();
    for phrase in [
        "configuration\tproxy_method",
        "controlled_native_nonharmonic_out_of_mask_proxy",
        "full_path_48_vs_192k_spectral_magnitude_alias_error_with_192_vs_768k_floor",
        "reference_floor_6db_classification",
        "harmonic_mask_coverage",
        "harmonic_mask_blind_spot",
        "raw_transfer_residual_diagnostic_only",
        "full_authored_bass_static_drift_frozen",
        "0.880000,0.720000,0.140000\t2.400000\t2.200000\ttrue\ttrue\tfrozen_for_stationary_analysis",
        "diagnostic_fail",
    ] {
        assert!(alias.contains(phrase), "alias report missing {phrase}");
    }
    assert_eq!(alias.lines().count(), 7);
    assert_eq!(alias.matches("controlled_diagnostic\t").count(), 3);
    assert_eq!(
        alias
            .matches("full_authored_bass_static_drift_frozen\t")
            .count(),
        3
    );
    assert_eq!(alias.matches("\tdiagnostic_fail\n").count(), 3);

    let oscillators = fs::read_to_string(first.path().join("oscillators.tsv")).unwrap();
    assert_eq!(oscillators.lines().count(), 28);
    assert_eq!(oscillators.matches("vco_pitch_drift_matrix\t").count(), 9);
    assert_eq!(oscillators.matches("vco_waveform_alias\t").count(), 15);
    for sample_rate in ["44100", "48000", "96000"] {
        for note in ["36", "60", "84"] {
            assert!(
                oscillators.contains(&format!(
                    "vco_pitch_drift_matrix\tNA\ttriangle\t{note}\t{sample_rate}\t-2.000000\t1.500000"
                )),
                "missing pitch/drift row note={note}, sample_rate={sample_rate}"
            );
        }
        for waveform in ["triangle", "saw", "rectangle", "wide_pulse", "narrow_pulse"] {
            assert!(
                oscillators.contains(&format!(
                    "vco_waveform_alias\tNA\t{waveform}\t96\t{sample_rate}\t"
                )),
                "missing waveform row waveform={waveform}, sample_rate={sample_rate}"
            );
        }
    }

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    for statement in [
        "No per-file normalization",
        "No full-band limiter",
        "No copied factory preset",
        "No hardware-equivalence claim",
        "No production integration",
        "controlled_diagnostic",
        "full_authored_bass_static_drift_frozen",
        "only drift is frozen",
        "diagnostic_fail",
        "harmonic-mask coverage and blind spot",
        "raw transfer residual remains diagnostic only",
    ] {
        assert!(readme.contains(statement), "README missing: {statement}");
    }

    let hashes = fs::read_to_string(first.path().join("hashes.tsv")).unwrap();
    assert_eq!(
        hashes.lines().next(),
        Some("file\tfnv1a_interleaved_stereo_sample_hash\tfnv1a_score_hash")
    );
    for (name, decoded_hash) in decoded_hashes {
        assert!(
            hashes.contains(&format!("{name}\t{decoded_hash:016x}\t")),
            "{name} decoded stereo hash is absent from hashes.tsv"
        );
    }
}

#[test]
#[ignore = "development-only historical Model D publication recovery"]
fn preexisting_fixed_stage_is_refused_without_touching_destination() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("audition");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel.txt"), "old destination").unwrap();
    let stage = root.path().join(".audition.model-d-lab-stage");
    fs::create_dir(&stage).unwrap();

    let status = run_lab_status(&output, &[]);
    assert!(!status.success());
    assert_eq!(
        fs::read_to_string(output.join("sentinel.txt")).unwrap(),
        "old destination"
    );
    assert!(stage.is_dir());
    assert!(!root.path().join(".audition.model-d-lab-backup").exists());
}

#[test]
#[ignore = "development-only historical Model D publication recovery"]
fn controlled_late_failure_preserves_prior_destination() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("audition");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel.txt"), "old destination").unwrap();

    let status = run_lab_status(
        &output,
        &[(
            "MOJ_SINT_MODEL_D_LAB_TEST_FAIL_AFTER_COST_REPORT",
            std::ffi::OsStr::new("1"),
        )],
    );
    assert!(!status.success());
    assert_eq!(
        file_names(&output),
        BTreeSet::from(["sentinel.txt".to_owned()])
    );
    assert_eq!(
        fs::read_to_string(output.join("sentinel.txt")).unwrap(),
        "old destination"
    );
    assert!(!root.path().join(".audition.model-d-lab-stage").exists());
    assert!(!root.path().join(".audition.model-d-lab-backup").exists());
}

#[test]
#[ignore = "development-only historical Model D publication recovery"]
fn destination_backup_rename_failure_cleans_populated_stage() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("audition");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel.txt"), "old destination").unwrap();

    let status = run_lab_status(
        &output,
        &[(
            "MOJ_SINT_MODEL_D_LAB_TEST_FAIL_BACKUP_RENAME",
            std::ffi::OsStr::new("1"),
        )],
    );
    assert!(!status.success());
    assert_eq!(
        file_names(&output),
        BTreeSet::from(["sentinel.txt".to_owned()])
    );
    assert_eq!(
        fs::read_to_string(output.join("sentinel.txt")).unwrap(),
        "old destination"
    );
    assert!(!root.path().join(".audition.model-d-lab-stage").exists());
    assert!(!root.path().join(".audition.model-d-lab-backup").exists());
}

#[test]
#[ignore = "development-only historical Model D publication recovery"]
fn partial_retired_cleanup_keeps_new_destination_authoritative_and_does_not_block_retry() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("audition");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel.txt"), "old destination").unwrap();

    let first = run_lab_output(
        &output,
        &[(
            "MOJ_SINT_MODEL_D_LAB_TEST_FAIL_RETIRED_PARTIAL_CLEANUP",
            std::ffi::OsStr::new("1"),
        )],
    );
    assert!(first.status.success(), "{first:?}");
    let stderr = String::from_utf8(first.stderr).unwrap();
    assert!(
        stderr.contains("warning: publication succeeded"),
        "{stderr}"
    );
    assert!(
        stderr.contains("retired previous destination cleanup incomplete"),
        "{stderr}"
    );
    let expected = WAV_NAMES
        .into_iter()
        .chain(REPORT_NAMES)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    assert_eq!(file_names(&output), expected);
    assert!(!output.join("sentinel.txt").exists());
    assert!(!root.path().join(".audition.model-d-lab-stage").exists());
    assert!(!root.path().join(".audition.model-d-lab-backup").exists());
    let retired = retired_paths(root.path(), "audition");
    assert_eq!(retired.len(), 1, "{retired:?}");
    assert!(retired[0].is_dir());
    assert!(
        stderr.contains(&retired[0].display().to_string()),
        "{stderr}"
    );

    let retry = run_lab_status(&output, &[]);
    assert!(retry.success());
    assert_eq!(
        file_names(&output),
        WAV_NAMES
            .into_iter()
            .chain(REPORT_NAMES)
            .map(str::to_owned)
            .collect()
    );
    assert!(!root.path().join(".audition.model-d-lab-stage").exists());
    assert!(!root.path().join(".audition.model-d-lab-backup").exists());
    assert_eq!(retired_paths(root.path(), "audition"), retired);

    fs::remove_dir_all(&retired[0]).unwrap();
    assert!(retired_paths(root.path(), "audition").is_empty());
}

fn run_lab(output: &Path) {
    let status = run_lab_status(output, &[]);
    assert!(status.success());
}

fn run_lab_status(output: &Path, environment: &[(&str, &std::ffi::OsStr)]) -> ExitStatus {
    let mut command = Command::new(env!("CARGO_BIN_EXE_model-d-lab"));
    command.arg("render-test").arg(output);
    for (name, value) in environment {
        command.env(name, value);
    }
    command.status().unwrap()
}

fn run_lab_output(output: &Path, environment: &[(&str, &std::ffi::OsStr)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_model-d-lab"));
    command.arg("render-test").arg(output);
    for (name, value) in environment {
        command.env(name, value);
    }
    command.output().unwrap()
}

fn retired_paths(parent: &Path, destination_name: &str) -> Vec<std::path::PathBuf> {
    let prefix = format!(".{destination_name}.model-d-lab-retired-");
    let mut paths = fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(&prefix))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn file_names(directory: &Path) -> BTreeSet<String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn deterministic_files(directory: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_name() != "workstation-cost.txt")
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (name, fs::read(entry.path()).unwrap())
        })
        .collect()
}

fn hash_sample_stream(samples: &[f32]) -> u64 {
    samples.iter().fold(0xcbf2_9ce4_8422_2325, |hash, sample| {
        (hash ^ u64::from(sample.to_bits())).wrapping_mul(0x0000_0100_0000_01b3)
    })
}
