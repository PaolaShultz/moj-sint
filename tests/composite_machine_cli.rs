use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn composite_machine_lab_writes_deterministic_complete_quick_batch() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());
    assert_eq!(
        deterministic_files(first.path()),
        deterministic_files(second.path())
    );

    let files = deterministic_files(first.path());
    let wavs: Vec<_> = files
        .keys()
        .filter(|name| name.ends_with(".wav"))
        .cloned()
        .collect();
    assert_eq!(wavs.len(), 14, "{wavs:?}");
    for candidate in [
        "synchronized-reference",
        "delayed-launch-estimate",
        "reduced-heterogeneous-stack",
        "role-separated-machine",
        "harmonic-lattice",
        "cross-topology-follower",
        "risky-nonlinear-braid",
    ] {
        for suffix in ["stereo", "mono"] {
            let name = format!("{candidate}_{suffix}.wav");
            assert!(files.contains_key(&name), "missing {name}");
            let reader = hound::WavReader::open(first.path().join(name)).unwrap();
            assert_eq!(reader.spec().channels, 2);
            assert_eq!(reader.spec().sample_rate, 4_000);
            assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        }
    }

    for report in [
        "README.md",
        "manifest.tsv",
        "layer-schedule.tsv",
        "frequency-timeline.tsv",
        "ablation.tsv",
        "harmony.tsv",
        "spectral.tsv",
        "stereo.tsv",
        "residual.tsv",
        "topology.tsv",
        "hashes.tsv",
        "similarity.tsv",
        "products.tsv",
        "rejections.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());
    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("synchronized counterfactual"));
    assert!(readme.contains("delayed-launch estimate"));
    assert!(readme.contains("unknown"));
    assert!(readme.contains("human listening"));
    assert!(readme.contains("acoustic SPL"));
    let timeline = fs::read_to_string(first.path().join("frequency-timeline.tsv")).unwrap();
    assert!(timeline.contains("30\t38:3"));
    assert!(timeline.contains("delayed-launch-estimate"));
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_composite-machine-lab"))
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
