use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "development-only historical audition renderer"]
fn lab_writes_one_deterministic_passing_controlled_thump() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());

    let files = deterministic_files(first.path());
    assert_eq!(files, deterministic_files(second.path()));
    assert_eq!(
        files
            .keys()
            .filter(|name| name.ends_with(".wav"))
            .collect::<Vec<_>>(),
        ["01_coupled_wire_controlled_thump.wav"]
    );
    for report in [
        "README.md",
        "manifest.tsv",
        "metrics.tsv",
        "bands.tsv",
        "alias.tsv",
        "selection.tsv",
        "rejections.tsv",
        "hashes.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    let mut wav =
        hound::WavReader::open(first.path().join("01_coupled_wire_controlled_thump.wav")).unwrap();
    assert_eq!(wav.spec().channels, 2);
    assert_eq!(wav.spec().sample_rate, 48_000);
    assert_eq!(wav.spec().sample_format, hound::SampleFormat::Float);
    assert_eq!(wav.samples::<f32>().count(), 2 * 48_000 * 1_600 / 1_000);

    let rejection = fs::read_to_string(first.path().join("rejections.tsv")).unwrap();
    assert!(rejection.contains("\tpass\tnone"));
    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("one listening file"));
    assert!(readme.contains("sub and fundamental remain linear"));
    assert!(readme.contains("external playback chain"));
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_coupled-wire-thump-lab"))
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
