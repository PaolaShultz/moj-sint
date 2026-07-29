use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

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
        frame_counts.insert(name, wav.samples::<f32>().count() / 2);
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
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_model-d-lab"))
        .arg("render-test")
        .arg(output)
        .status()
        .unwrap();
    assert!(status.success());
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
