use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "development-only historical audition renderer"]
fn hybrid_sound_lab_writes_deterministic_complete_listening_gate() {
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
    assert_eq!(wav_names.len(), 12, "{wav_names:?}");
    for family in [
        "cross-coupled-machine",
        "spectral-shadow",
        "dual-resonant-body",
    ] {
        for condition in ["single", "chord", "progression", "progression-mono"] {
            let name = format!("{family}_{condition}.wav");
            assert!(wav_names.contains(&name), "missing {name}");
            let reader = hound::WavReader::open(first.path().join(name)).unwrap();
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
        "impact.tsv",
        "stereo.tsv",
        "harmony.tsv",
        "products.tsv",
        "alias-error.tsv",
        "topology.tsv",
    ] {
        assert!(first_files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());
    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("human listening decides"));
    assert!(readme.contains("mono-fold diagnostics"));
    assert!(readme.contains("does not report acoustic SPL"));
    assert!(readme.contains("not Raspberry Pi evidence"));
    let manifest = fs::read_to_string(first.path().join("manifest.tsv")).unwrap();
    assert_eq!(manifest.lines().count(), 13);
    let products = fs::read_to_string(first.path().join("products.tsv")).unwrap();
    assert!(products.starts_with("file\tapplicable\t"));
    assert!(
        products
            .lines()
            .find(|line| line.starts_with("cross-coupled-machine_single.wav\t"))
            .unwrap()
            .contains("\tfalse\t")
    );
    assert!(
        products
            .lines()
            .find(|line| line.starts_with("cross-coupled-machine_chord.wav\t"))
            .unwrap()
            .contains("\ttrue\t")
    );
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_hybrid-sound-lab"))
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
