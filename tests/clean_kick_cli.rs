use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const WAVS: [&str; 4] = [
    "01_house-impact_solo.wav",
    "02_long-pressure_solo.wav",
    "03_house-impact_124bpm.wav",
    "04_long-pressure_124bpm.wav",
];

#[test]
fn lab_writes_four_deterministic_clean_kick_wavs() {
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
            .map(String::as_str)
            .collect::<Vec<_>>(),
        WAVS
    );
    for report in [
        "README.md",
        "settings.tsv",
        "metrics.tsv",
        "hashes.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    for name in WAVS {
        let mut wav = hound::WavReader::open(first.path().join(name)).unwrap();
        assert_eq!(wav.spec().channels, 2);
        assert_eq!(wav.spec().sample_rate, 48_000);
        assert_eq!(wav.spec().bits_per_sample, 32);
        assert_eq!(wav.spec().sample_format, hound::SampleFormat::Float);
        let samples = wav.samples::<f32>().collect::<Result<Vec<_>, _>>().unwrap();
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.chunks_exact(2).all(|pair| pair[0] == pair[1]));
    }

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    for forbidden in [
        "clipper",
        "limiter",
        "compressor",
        "saturation",
        "normalizer",
        "noise layer",
    ] {
        assert!(readme.contains(forbidden), "README omits {forbidden}");
    }
    assert!(readme.contains("listening decision remains open"));
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_clean-kick-lab"))
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
