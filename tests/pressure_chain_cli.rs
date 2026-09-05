use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

const EXPECTED_FILES: [&str; 4] = [
    "01 Deep Cascade.wav",
    "02 Body Tap.wav",
    "03 Cross Feed.wav",
    "measurements.txt",
];

#[test]
#[ignore = "development-only Pressure Chain audition renderer"]
fn lab_writes_one_deterministic_controlled_comparison_of_three_real_topologies() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for destination in [first.path().join("lab"), second.path().join("lab")] {
        let output = Command::new(env!("CARGO_BIN_EXE_pressure-chain-lab"))
            .arg(&destination)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let mut names: Vec<_> = fs::read_dir(&destination)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, EXPECTED_FILES);
    }

    let mut audio_hashes = BTreeSet::new();
    for file in EXPECTED_FILES {
        let first_bytes = fs::read(first.path().join("lab").join(file)).unwrap();
        let second_bytes = fs::read(second.path().join("lab").join(file)).unwrap();
        assert_eq!(first_bytes, second_bytes, "file={file}");
        if file.ends_with(".wav") {
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            for byte in &first_bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            assert!(audio_hashes.insert(hash), "duplicate file={file}");

            let mut reader = hound::WavReader::open(first.path().join("lab").join(file)).unwrap();
            assert_eq!(reader.spec().channels, 2, "file={file}");
            assert_eq!(reader.spec().sample_rate, 48_000, "file={file}");
            assert_eq!(reader.spec().bits_per_sample, 32, "file={file}");
            assert_eq!(reader.spec().sample_format, hound::SampleFormat::Float);
            let samples: Vec<f32> = reader.samples().collect::<Result<_, _>>().unwrap();
            assert_eq!(samples.len(), 12 * 48_000 * 2, "file={file}");
            assert!(
                samples
                    .iter()
                    .all(|sample| sample.is_finite() && sample.abs() <= 0.95)
            );
            assert!(samples.iter().any(|sample| sample.abs() > 1.0e-4));
            assert!(samples.chunks_exact(2).all(|frame| frame[0] == frame[1]));
            let maximum_jump = samples
                .chunks_exact(2)
                .map(|frame| frame[0])
                .scan(0.0_f32, |previous, sample| {
                    let jump = (sample - *previous).abs();
                    *previous = sample;
                    Some(jump)
                })
                .fold(0.0_f32, f32::max);
            assert!(maximum_jump < 0.90, "file={file} jump={maximum_jump}");
        }
    }

    let report = fs::read_to_string(first.path().join("lab/measurements.txt")).unwrap();
    assert!(report.starts_with("Pressure Chain listening gate\n"));
    assert!(report.contains("source: docs/ACID_CHAIN_RESEARCH.md\n"));
    assert_eq!(report.matches("topology: ").count(), 3);
    assert_eq!(report.matches("residual_db_midi_").count(), 9);
    assert!(!report.to_ascii_lowercase().contains("emulation"));
}
