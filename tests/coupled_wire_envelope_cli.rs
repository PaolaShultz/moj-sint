use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn lab_writes_only_deterministic_passing_envelope_comparisons() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());

    let files = deterministic_files(first.path());
    assert_eq!(files, deterministic_files(second.path()));
    for report in [
        "README.md",
        "manifest.tsv",
        "envelopes.tsv",
        "motion.tsv",
        "metrics.tsv",
        "rejections.tsv",
        "hashes.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    let expected = [
        ("00_coupled_wire_reference.wav", 1_600),
        ("01_coupled_wire_warm_hold.wav", 3_000),
        ("02_coupled_wire_slow_orbit.wav", 3_300),
        ("03_coupled_wire_fast_orbit.wav", 3_300),
    ];
    let wav_names = files
        .keys()
        .filter(|name| name.ends_with(".wav"))
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(wav_names, expected.map(|(name, _)| name).as_slice());
    for (name, duration_ms) in expected {
        let mut reader = hound::WavReader::open(first.path().join(name)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().bits_per_sample, 32);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        assert_eq!(
            reader.samples::<f32>().count(),
            2 * 48_000 * duration_ms / 1_000
        );
    }

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("Coupled Wire envelope and motion"));
    assert!(readme.contains("low body remains fixed"));
    assert!(readme.contains("mono sum is unchanged"));
    assert!(readme.contains("human listening decides"));
    assert!(!readme.to_lowercase().contains("chord"));
    assert!(!readme.to_lowercase().contains("piano"));

    let manifest = fs::read_to_string(first.path().join("manifest.tsv")).unwrap();
    assert_eq!(manifest.lines().count(), 5);
    for line in manifest.lines().skip(1) {
        assert!(line.ends_with("\tpass"), "{line}");
    }

    let hashes = fs::read_to_string(first.path().join("hashes.tsv")).unwrap();
    assert!(hashes.contains("00_coupled_wire_reference.wav\t443e5faae3184791"));

    let motion = fs::read_to_string(first.path().join("motion.tsv")).unwrap();
    let mut lines = motion.lines();
    let header = lines.next().unwrap().split('\t').collect::<Vec<_>>();
    let ratio_index = header
        .iter()
        .position(|field| *field == "side_difference_ratio")
        .unwrap();
    for line in
        lines.filter(|line| line.starts_with("slow-orbit\t") || line.starts_with("fast-orbit\t"))
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert!(fields[ratio_index].parse::<f64>().unwrap() >= 0.003);
    }

    let rejections = fs::read_to_string(first.path().join("rejections.tsv")).unwrap();
    for line in rejections.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 4, "{line}");
        if fields[2] == "reject" {
            assert!(!first.path().join(fields[1]).exists(), "{line}");
        }
    }
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_coupled-wire-envelope-lab"))
        .arg("render-test")
        .arg(output)
        .status()
        .unwrap();
    assert!(status.success());
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
