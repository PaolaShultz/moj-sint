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
    assert_eq!(wavs.len(), 54, "{wavs:?}");
    for file in [
        "01_hot-delayed-launch-reference_stereo.wav",
        "02_hot-synchronized-counterfactual_stereo.wav",
        "03_hot-full-score_reduced-heterogeneous-stack_stereo.wav",
        "04_octave_role-separated-machine_d1_stereo.wav",
        "04_octave_role-separated-machine_d2_stereo.wav",
        "04_octave_role-separated-machine_d3_stereo.wav",
        "05_d1-pedal_harmonic-lattice_stereo.wav",
        "06_punch_cross-topology-follower_stereo.wav",
        "07_mono_d1-held_risky-nonlinear-braid.wav",
        "07_mono_d1-pedal_risky-nonlinear-braid.wav",
        "07_mono_punch_risky-nonlinear-braid.wav",
        "08_raw_synchronized-reference_stereo.wav",
        "08_raw_delayed-launch-estimate_stereo.wav",
    ] {
        assert!(files.contains_key(file), "missing {file}");
        let reader = hound::WavReader::open(first.path().join(file)).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 4_000);
        assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
    }
    for candidate in [
        "reduced-heterogeneous-stack",
        "role-separated-machine",
        "harmonic-lattice",
        "cross-topology-follower",
        "risky-nonlinear-braid",
    ] {
        for register in ["d1", "d2", "d3"] {
            let name = format!("04_octave_{candidate}_{register}_stereo.wav");
            assert!(files.contains_key(&name), "missing {name}");
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
        "gain-policy.tsv",
        "octave.tsv",
        "punch-adsr.tsv",
        "allocation-evidence.tsv",
        "audition-similarity.tsv",
        "audition-residual.tsv",
        "reconstruction-regression.tsv",
        "processors.tsv",
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
    assert!(readme.contains("hot delayed-launch reference"));
    assert!(readme.contains("D1/D2/D3"));
    assert!(readme.contains("punch ADSR"));
    assert!(readme.contains("raw engineering"));
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
