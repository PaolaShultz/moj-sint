use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "development-only historical audition renderer"]
fn composite_machine_lab_writes_deterministic_compact_power_batch() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());
    assert_eq!(
        deterministic_files(first.path()),
        deterministic_files(second.path())
    );

    let files = deterministic_files(first.path());
    let primary = [
        "02_sustained-low_deep-rotor_d1.wav",
        "03_playable-mid_phase-forge_d3.wav",
        "04_evolving_evolving-lattice_d2.wav",
        "05_pedal_pedal-monolith_d1.wav",
        "06_bass_d2-braid_d2.wav",
        "07_thump_clipped-thump_d1.wav",
        "08_thump_resonant-thump_d1.wav",
        "09_kick_synthetic-kick_d1.wav",
        "10_struck_struck-comb_d3.wav",
        "11_context_context-relay.wav",
    ];
    for name in [
        "01_reference_hot-delayed-launch.wav",
        "12_mono_pedal-monolith_d1.wav",
        "12_mono_synthetic-kick_d1.wav",
    ]
    .into_iter()
    .chain(primary)
    {
        assert!(files.contains_key(name), "missing {name}");
        let reader = hound::WavReader::open(first.path().join(name)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 4_000);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
    }
    let primary_wavs = files
        .keys()
        .filter(|name| {
            name.ends_with(".wav")
                && name
                    .split('_')
                    .next()
                    .is_some_and(|number| (2..=11).contains(&number.parse::<u8>().unwrap_or(0)))
        })
        .count();
    assert_eq!(primary_wavs, 10);

    for report in [
        "README.md",
        "primary-manifest.tsv",
        "settings.tsv",
        "loudness.tsv",
        "tone-retention.tsv",
        "pitch-retention.tsv",
        "event-metrics.tsv",
        "ablation.tsv",
        "stereo-mono.tsv",
        "residual.tsv",
        "hashes.tsv",
        "rejections.tsv",
        "reconstruction-regression.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("intentionally hot"));
    assert!(readme.contains("start with playback volume low"));
    assert!(readme.contains("one mechanism"));
    assert!(readme.contains("three mechanisms"));
    assert!(readme.contains("human listening"));
    assert!(readme.contains("not acoustic SPL"));
    assert!(readme.contains("mono diagnostics last"));

    let settings = fs::read_to_string(first.path().join("settings.tsv")).unwrap();
    assert!(settings.contains("hard-clip"));
    assert!(settings.contains("cubic-saturation"));
    assert!(settings.contains("rational-saturation"));
    assert!(settings.contains("pitch-drop-table-oscillator"));
    let rejections = fs::read_to_string(first.path().join("rejections.tsv")).unwrap();
    assert!(!rejections.lines().any(|line| line.contains("\treject\t")));
    let regression =
        fs::read_to_string(first.path().join("reconstruction-regression.tsv")).unwrap();
    assert!(regression.contains("quick_mode_determinism_only"));
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
