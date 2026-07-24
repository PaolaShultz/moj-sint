use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn lab_writes_only_deterministic_passing_struck_objects() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());

    let files = deterministic_files(first.path());
    assert_eq!(files, deterministic_files(second.path()));
    for report in [
        "README.md",
        "manifest.tsv",
        "modes.tsv",
        "metrics.tsv",
        "decay.tsv",
        "rejections.tsv",
        "hashes.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    let wav_names = files
        .keys()
        .filter(|name| name.ends_with(".wav"))
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        wav_names,
        [
            "01_coupled_wire.wav",
            "02_spectral_plate.wav",
            "03_dual_bridge.wav",
        ]
    );
    for name in wav_names {
        let mut reader = hound::WavReader::open(first.path().join(name)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 48_000);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
        assert_eq!(reader.samples::<f32>().count(), 2 * 48_000 * 1_600 / 1_000);
    }

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("Moj Sint struck objects"));
    assert!(readme.contains("exciter is not mixed dry"));
    assert!(readme.contains("natural modal decay"));
    assert!(readme.contains("human listening decides"));
    assert!(!readme.to_lowercase().contains("piano"));
    assert!(!readme.to_lowercase().contains("chord"));

    let manifest = fs::read_to_string(first.path().join("manifest.tsv")).unwrap();
    assert_eq!(manifest.lines().count(), 4);
    for line in manifest.lines().skip(1) {
        assert!(line.ends_with("\tpass"), "{line}");
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
    let status = Command::new(env!("CARGO_BIN_EXE_struck-object-lab"))
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
