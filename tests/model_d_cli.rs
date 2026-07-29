use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};

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
fn lab_writes_exact_deterministic_passing_model_d_audition() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    fs::write(first.path().join("stale.txt"), "remove me").unwrap();
    run_lab(first.path());
    run_lab(second.path());

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
        "native_192k_nonharmonic_out_of_mask_foldback_proxy",
        "reference_floor_6db_classification",
        "harmonic_mask_coverage",
        "harmonic_mask_blind_spot",
        "raw_transfer_residual_diagnostic_only",
    ] {
        assert!(alias.contains(phrase), "alias report missing {phrase}");
    }

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    for statement in [
        "No per-file normalization",
        "No full-band limiter",
        "No copied factory preset",
        "No hardware-equivalence claim",
        "No production integration",
        "native 192 kHz nonharmonic out-of-mask foldback proxy",
        "6 dB floor classification",
        "harmonic-mask coverage and blind spot",
        "raw transfer residual is diagnostic only",
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

#[cfg(unix)]
#[test]
fn late_dev_full_report_failure_preserves_prior_destination() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("audition");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("sentinel.txt"), "old destination").unwrap();
    let full_link = root.path().join("dev-full");
    symlink("/dev/full", &full_link).unwrap();

    let status = run_lab_status(
        &output,
        &[(
            "MOJ_SINT_MODEL_D_LAB_TEST_WORKSTATION_COST_PATH",
            full_link.as_os_str(),
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
