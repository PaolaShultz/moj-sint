use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn five_family_lab_writes_deterministic_listening_gate() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());

    let first_files = deterministic_files(first.path());
    let second_files = deterministic_files(second.path());
    assert_eq!(first_files, second_files);

    let wav_names: Vec<_> = first_files
        .keys()
        .filter(|name| name.ends_with(".wav"))
        .cloned()
        .collect();
    assert_eq!(wav_names.len(), 15, "{wav_names:?}");
    for family in [
        "nonlinear-pm",
        "excited-comb",
        "spatial-micro-delay",
        "spectral-traversal",
        "integer-swarm",
    ] {
        for note in [36, 60, 84] {
            let name = format!("{family}_note{note:03}.wav");
            assert!(wav_names.contains(&name), "missing {name}");
            let reader = hound::WavReader::open(first.path().join(&name)).unwrap();
            let spec = reader.spec();
            assert_eq!(spec.channels, 2);
            assert_eq!(spec.sample_rate, 48_000);
            assert_eq!(spec.bits_per_sample, 32);
            assert_eq!(spec.sample_format, hound::SampleFormat::Float);
        }
    }
    for report in [
        "README.md",
        "manifest.tsv",
        "spectral.tsv",
        "spatial.tsv",
        "alias-error.tsv",
        "integer-cycles.tsv",
    ] {
        assert!(first_files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());
    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("human listening verdict is open"));
    assert!(readme.contains("not Raspberry Pi evidence"));
    let manifest = fs::read_to_string(first.path().join("manifest.tsv")).unwrap();
    assert_eq!(manifest.lines().count(), 16);
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_five-family-lab"))
        .arg("render")
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
            let bytes = fs::read(entry.path()).unwrap();
            (name, bytes)
        })
        .collect()
}
