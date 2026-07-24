use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn hybrid_subset_lab_writes_only_deterministic_passing_original_combinations() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());
    assert_eq!(
        deterministic_files(first.path()),
        deterministic_files(second.path())
    );

    let files = deterministic_files(first.path());
    assert!(files.contains_key("01_reference_fixed-microdelay-master-envelope.wav"));
    for report in [
        "README.md",
        "manifest.tsv",
        "metrics.tsv",
        "tonal.tsv",
        "residual.tsv",
        "rejections.tsv",
        "hashes.tsv",
        "composite-regression.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    let allowed = [
        "01_reference_fixed-microdelay-master-envelope.wav",
        "02_three-singles.wav",
        "03_three-chords.wav",
        "04_three-stereo-progressions.wav",
        "05_three-mono-progressions.wav",
        "06_cross-anchor.wav",
        "07_spectral-anchor.wav",
        "08_dual-anchor.wav",
    ];
    let wav_names = files
        .keys()
        .filter(|name| name.ends_with(".wav"))
        .collect::<Vec<_>>();
    assert_eq!(wav_names.len(), 8);
    for name in wav_names {
        assert!(allowed.contains(&name.as_str()), "unexpected WAV {name}");
        let reader = hound::WavReader::open(first.path().join(name)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 4_000);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
    }

    let rejections = fs::read_to_string(first.path().join("rejections.tsv")).unwrap();
    for line in rejections.lines().skip(1) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 4, "{line}");
        if fields[2] == "reject" {
            assert!(!first.path().join(fields[1]).exists(), "{line}");
        }
    }

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("exact original hybrid layers"));
    assert!(readme.contains("fixed micro-delays"));
    assert!(readme.contains("one shared master ADSR"));
    assert!(!readme.contains("delayed-launch"));
    assert!(readme.contains("start with playback volume low"));
    assert!(readme.contains("human listening decides"));
    assert!(readme.contains("not acoustic SPL"));

    let manifest = fs::read_to_string(first.path().join("manifest.tsv")).unwrap();
    assert!(manifest.contains("0,2,5"));
    assert!(manifest.contains("0,4,9,15"));
    for line in manifest.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 8, "{line}");
        for offset in fields[5].split(',') {
            assert!(offset.parse::<u32>().unwrap() <= 15, "{line}");
        }
    }
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_hybrid-subset-lab"))
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
